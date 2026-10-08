// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Independent positive relations and exact projected witness frontiers.
//!
//! This is a private physical path through the ordinary join and emission kernels.
//! It never collapses a callback's input multiplicity. Each factor is evaluated once
//! against the frozen store; mode tables borrow those matches, and only the product
//! actually named by a head is visited. Source coordinates always remain authored.

use super::{
    ATOM_ARITY, ArgShape, BTreeMap, BTreeSet, Delta, EvalError, GroupArg, GroupProbe, JoinGroup,
    JoinSnapshot, LeapfrogRun, NoGuards, PositionPlan, RelationStore, RoundBuffer, RoundSnapshot,
    RuleEntry, RulePlan, RuleRuntime, Scan, SlotSolution, StepGovernor, TermId, delta_can_match,
    emit_solution, extend_slot_solutions, extend_solutions_leapfrog, source_heights,
    source_preference,
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
pub(super) fn certify(plan: &RulePlan, runtime: &RuleRuntime) -> Vec<Factor> {
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
fn full_matches(
    factor: &Factor,
    entry: RuleEntry<'_>,
    snapshot: RoundSnapshot<'_>,
    governor: &mut StepGovernor,
) -> Result<Vec<SlotSolution>, EvalError> {
    let mut partial = vec![SlotSolution::empty(entry.plan.variables().len())];
    let binary = |position, partial: &[SlotSolution], governor: &mut StepGovernor| {
        extend_slot_solutions(
            entry.plan.operator_at(position),
            snapshot.rel,
            entry.delta,
            Scan::Full,
            partial,
            governor,
        )
    };
    if !entry.plan.has_cyclic_subplan() {
        for &position in &factor.positions {
            partial = binary(position, &partial, governor);
            if partial.is_empty() || governor.spent() {
                break;
            }
        }
    } else {
        for group in entry.plan.join_groups() {
            let position = match group {
                JoinGroup::Binary(atom) => atom.positive_position(),
                JoinGroup::Leapfrog(cycle) => cycle.atoms()[0].positive_position(),
            };
            if !factor.positions.contains(&position) {
                continue;
            }
            partial = match group {
                JoinGroup::Binary(atom) => binary(atom.positive_position(), &partial, governor),
                JoinGroup::Leapfrog(cycle) => extend_solutions_leapfrog(
                    LeapfrogRun {
                        plan: entry.plan,
                        cycle,
                        delta_position: usize::MAX,
                        rel: snapshot.rel,
                        delta: entry.delta,
                    },
                    &partial,
                    governor,
                ),
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
        solution.sources.sort_by_key(|source| source.body_index);
        if factor.negated.iter().any(|&index| {
            governor.spent()
                || entry.runtime.negated[index].satisfied(&solution, snapshot.rel, governor)
        }) {
            continue;
        }
        let mut blocked = false;
        for &index in &factor.negations {
            if entry.runtime.negations[index].holds(
                GroupProbe {
                    solution: &solution,
                    rel: snapshot.rel,
                    guards: &NoGuards,
                    rule: entry.index,
                    clause: entry.rule,
                },
                governor,
            )? {
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

/// A borrowed factor witness, with its content-derived numeric prefix.
#[derive(Debug, Clone, Copy)]
struct Witness<'a> {
    /// The stored Full match; mode and projection tables never clone it.
    solution: &'a SlotSolution,
    /// Raw maximum source height, before the head's saturating increment.
    maximum: u32,
    /// Saturated sum of source heights.
    sum: u64,
}

/// Both prefix winners at a raw-height threshold.
#[derive(Debug, Clone, Copy)]
struct Threshold<'a> {
    /// Inclusive raw source-height threshold.
    height: u32,
    /// Best sum followed by lexical witness at or below this threshold.
    summed: Witness<'a>,
    /// Best purely lexical witness at or below this threshold.
    lexical: Witness<'a>,
}

/// Exact prefix frontiers for one projected binding and one scan mode.
#[derive(Debug)]
struct Frontier<'a> {
    /// Sorted distinct height thresholds; each stores both cumulative winners.
    thresholds: Vec<Threshold<'a>>,
}

impl<'a> Frontier<'a> {
    /// Build cumulative frontiers without assuming local height preference composes.
    fn new(mut witnesses: Vec<Witness<'a>>, rel: &RelationStore) -> Self {
        witnesses.sort_by_key(|witness| witness.maximum);
        let mut thresholds: Vec<Threshold<'a>> = Vec::new();
        for witness in witnesses {
            let mut next = thresholds.last().copied().unwrap_or(Threshold {
                height: witness.maximum,
                summed: witness,
                lexical: witness,
            });
            let lexical = |left: Witness<'_>, right: Witness<'_>| {
                source_preference(&left.solution.sources, 0, &right.solution.sources, 0, rel)
                    .is_lt()
            };
            if witness.sum < next.summed.sum
                || (witness.sum == next.summed.sum && lexical(witness, next.summed))
            {
                next.summed = witness;
            }
            if lexical(witness, next.lexical) {
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
        Self { thresholds }
    }

    /// Minimum attainable raw maximum.
    fn minimum_height(&self) -> u32 {
        self.thresholds[0].height
    }

    /// Both exact winners under a feasible global raw-height threshold.
    fn at(&self, maximum: u32) -> Threshold<'a> {
        let end = self
            .thresholds
            .partition_point(|threshold| threshold.height <= maximum);
        self.thresholds[end - 1]
    }
}

/// A projection table for Full, Old and New, all borrowing a single factor relation.
#[derive(Debug)]
struct Projection<'a> {
    /// Any matching source tuple.
    full: BTreeMap<Vec<TermId>, Frontier<'a>>,
    /// Tuples having no source in the current delta.
    old: BTreeMap<Vec<TermId>, Frontier<'a>>,
    /// Tuples having at least one source in the current delta.
    new: BTreeMap<Vec<TermId>, Frontier<'a>>,
}

/// Build all mode frontiers for one head's projection of one Full factor.
fn project<'a>(
    matches: &'a [SlotSolution],
    slots: &[usize],
    snapshot: RoundSnapshot<'_>,
    delta: Delta,
) -> Projection<'a> {
    let mut full: BTreeMap<Vec<TermId>, Vec<Witness<'a>>> = BTreeMap::new();
    let mut old: BTreeMap<Vec<TermId>, Vec<Witness<'a>>> = BTreeMap::new();
    let mut new: BTreeMap<Vec<TermId>, Vec<Witness<'a>>> = BTreeMap::new();
    for solution in matches {
        let key: Vec<_> = slots
            .iter()
            .map(|&slot| {
                solution
                    .get(slot)
                    .expect("a factor binds its positive slots")
            })
            .collect();
        let (maximum, sum) = source_heights(&solution.sources, snapshot.depth);
        let witness = Witness {
            solution,
            maximum,
            sum,
        };
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
    let seal = |table: BTreeMap<Vec<TermId>, Vec<Witness<'a>>>| {
        table
            .into_iter()
            .map(|(key, witnesses)| (key, Frontier::new(witnesses, snapshot.rel)))
            .collect()
    };
    Projection {
        full: seal(full),
        old: seal(old),
        new: seal(new),
    }
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
struct Product<'a, 'r> {
    /// One mode table per certified factor.
    tables: Vec<&'a BTreeMap<Vec<TermId>, Frontier<'a>>>,
    /// The authored head currently emitted.
    head: &'r [ArgShape; ATOM_ARITY],
    /// Rule identity and variable-frame width.
    entry: RuleEntry<'r>,
    /// Store and source proof heights.
    snapshot: RoundSnapshot<'a>,
}

impl<'r> Product<'_, 'r> {
    /// Stream necessary projected combinations with a borrowed heap odometer.
    fn visit(&self, buffer: &mut RoundBuffer<'r>, governor: &mut StepGovernor) {
        let mut cursors: Vec<_> = self.tables.iter().map(|table| table.values()).collect();
        let mut chosen = Vec::with_capacity(self.tables.len());
        let mut position = 0;
        while !governor.spent() {
            if let Some(frontier) = cursors[position].next() {
                chosen.push(frontier);
                if chosen.len() == self.tables.len() {
                    self.emit(&chosen, buffer, governor);
                    chosen.pop();
                } else {
                    position += 1;
                }
            } else if position == 0 {
                break;
            } else {
                cursors[position] = self.tables[position].values();
                position -= 1;
                chosen.pop();
            }
        }
    }

    /// Choose the exact global witness for one projected combination, then emit it.
    fn emit(
        &self,
        chosen: &[&Frontier<'_>],
        buffer: &mut RoundBuffer<'r>,
        governor: &mut StepGovernor,
    ) {
        if !governor.charge() {
            return;
        }
        // Height masking means a factor's locally shortest proof may not minimize the
        // global sum. Saturated sums in turn mask that sum: pure lex then decides.
        let threshold = global_threshold(
            chosen
                .iter()
                .map(|frontier| frontier.minimum_height())
                .max()
                .unwrap_or(0),
        );
        let winners: Vec<_> = chosen
            .iter()
            .map(|frontier| frontier.at(threshold))
            .collect();
        let lexical = saturated_sum(winners.iter().map(|winner| winner.summed.sum));
        let mut solution = SlotSolution::empty(self.entry.plan.variables().len());
        for winner in winners {
            let witness = if lexical {
                winner.lexical
            } else {
                winner.summed
            };
            for (slot, binding) in witness.solution.bindings.iter().enumerate() {
                if binding.is_some() {
                    solution.bindings[slot] = *binding;
                }
            }
            solution
                .sources
                .extend_from_slice(&witness.solution.sources);
        }
        solution.sources.sort_by_key(|source| source.body_index);
        emit_solution(
            buffer,
            self.head,
            &solution,
            self.entry.index,
            self.snapshot,
        );
    }
}

/// Evaluate independent factors and stream each conjunctive head's necessary product.
pub(super) fn evaluate<'r>(
    entry: RuleEntry<'r>,
    snapshot: RoundSnapshot<'_>,
    governor: &mut StepGovernor,
) -> Result<RoundBuffer<'r>, EvalError> {
    let mut buffer = RoundBuffer::new();
    let join = JoinSnapshot {
        rel: snapshot.rel,
        delta: entry.delta,
    };
    if !entry
        .plan
        .operators()
        .iter()
        .any(|operator| delta_can_match(operator, join))
    {
        return Ok(buffer);
    }
    if ground_blocked(entry, snapshot, governor)? {
        buffer.join_steps = governor.consumed;
        return Ok(buffer);
    }
    let mut relations = Vec::with_capacity(entry.runtime.factors.len());
    for factor in &entry.runtime.factors {
        let matches = full_matches(factor, entry, snapshot, governor)?;
        if matches.is_empty() || governor.spent() {
            buffer.join_steps = governor.consumed;
            return Ok(buffer);
        }
        relations.push(matches);
    }
    for head in &entry.runtime.head {
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
                )
            })
            .collect();
        for anchor in 0..projections.len() {
            let tables: Vec<_> = projections
                .iter()
                .enumerate()
                .map(|(position, projection)| match position.cmp(&anchor) {
                    std::cmp::Ordering::Less => &projection.old,
                    std::cmp::Ordering::Equal => &projection.new,
                    std::cmp::Ordering::Greater => &projection.full,
                })
                .collect();
            if tables.iter().any(|table| table.is_empty()) {
                continue;
            }
            Product {
                tables,
                head,
                entry,
                snapshot,
            }
            .visit(&mut buffer, governor);
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
}

/// Negative predicates with no positive outer slot are round-wide preconditions.
/// Local variables remain local even when their authored name appears in another group.
fn ground_blocked(
    entry: RuleEntry<'_>,
    snapshot: RoundSnapshot<'_>,
    governor: &mut StepGovernor,
) -> Result<bool, EvalError> {
    let empty = SlotSolution::empty(entry.plan.variables().len());
    for (index, atom) in entry.runtime.negated.iter().enumerate() {
        if governor.spent() {
            return Ok(true);
        }
        if !entry
            .runtime
            .factors
            .iter()
            .any(|factor| factor.negated.contains(&index))
            && atom.satisfied(&empty, snapshot.rel, governor)
        {
            return Ok(true);
        }
    }
    for (index, group) in entry.runtime.negations.iter().enumerate() {
        if governor.spent() {
            return Ok(true);
        }
        if !entry
            .runtime
            .factors
            .iter()
            .any(|factor| factor.negations.contains(&index))
            && group.holds(
                GroupProbe {
                    solution: &empty,
                    rel: snapshot.rel,
                    guards: &NoGuards,
                    rule: entry.index,
                    clause: entry.rule,
                },
                governor,
            )?
        {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::{Frontier, Witness, full_matches, global_threshold, project, saturated_sum};
    use crate::clause::{ClauseAtom, ClauseTerm, DlClause, HeadDisjunct};
    use crate::guard::{Guard, Negation, NoGuards};
    use crate::plan::RulePlan;
    use crate::seminaive::{
        Delta, EvalOptions, JoinStrategy, RoundExecution, RoundSnapshot, RuleEntry, RuleRuntime,
        SlotSolution, SourceRow, StepGovernor, compile, evaluate_rule, evaluate_with,
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
            store.insert(subject, predicate, object, graph);
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
                        EvalOptions::default(),
                        RoundExecution::Sequential,
                        JoinStrategy::Planned,
                    )
                    .expect("small factored input completes");
                    let binary = evaluate_with(
                        &exe,
                        seeded(&input),
                        None,
                        &NoGuards,
                        EvalOptions::default(),
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
                        planned.derivations(),
                        binary.derivations(),
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
                    assert_eq!(planned.derivations().len(), expected);
                    for derivation in planned.derivations() {
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
                EvalOptions::default(),
                RoundExecution::Sequential,
                JoinStrategy::Planned,
            )
            .expect("linear input stays within default limits");
            assert_eq!(evaluation.derivations().len(), n);
            assert_eq!(evaluation.budget().join_steps, 3 * n as u64);
        }
    }

    /// Stored Full relations and projected prefix entries grow additively. Mode
    /// frontiers retain borrowed source frames, not cloned Cartesian assignments.
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
                rel: &rel,
                depth: &depth,
                assumed: &[],
            };
            let entry = RuleEntry {
                index: 0,
                rule: &rule,
                plan: &plan,
                runtime: &runtime,
                delta: Delta::all(input.len()),
            };
            let mut governor = StepGovernor::new(u64::MAX);
            let mut stored_rows = 0;
            let mut stored_thresholds = 0;
            for factor in &runtime.factors {
                let matches = full_matches(factor, entry, snapshot, &mut governor)
                    .expect("Full factor completes");
                stored_rows += matches.len();
                let slots = if factor.slots.contains(&head_slot) {
                    vec![head_slot]
                } else {
                    Vec::new()
                };
                let projection = project(&matches, &slots, snapshot, entry.delta);
                assert!(projection.old.is_empty());
                for frontier in projection.full.values().chain(projection.new.values()) {
                    stored_thresholds += frontier.thresholds.len();
                    for threshold in &frontier.thresholds {
                        assert!(
                            matches
                                .iter()
                                .any(|solution| std::ptr::eq(solution, threshold.summed.solution))
                        );
                        assert!(
                            matches
                                .iter()
                                .any(|solution| std::ptr::eq(solution, threshold.lexical.solution))
                        );
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
                    lo,
                    hi: input.len(),
                },
            };
            let snapshot = RoundSnapshot {
                rel: &rel,
                depth: &depth,
                assumed: &[],
            };
            let planned = evaluate_rule(
                entry,
                snapshot,
                JoinStrategy::Planned,
                StepGovernor::new(u64::MAX),
                &NoGuards,
            )
            .expect("factor round completes");
            let binary = evaluate_rule(
                entry,
                snapshot,
                JoinStrategy::ForcedBinary,
                StepGovernor::new(u64::MAX),
                &NoGuards,
            )
            .expect("exhaustive round completes");
            assert_eq!(planned.entries, binary.entries, "delta lower bound {lo}");
            assert!(
                planned.join_steps <= 6 + 12 + 2,
                "both heads reuse the single Full factor relations"
            );
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
            let exe = compile(vec![rule]).expect("stratified negatives compile");
            for seed in 0..4 {
                let shuffled = permute(&input, seed);
                let planned = evaluate_with(
                    &exe,
                    seeded(&shuffled),
                    None,
                    &NoGuards,
                    EvalOptions::default(),
                    RoundExecution::Sequential,
                    JoinStrategy::Planned,
                )
                .expect("small factor model completes");
                let binary = evaluate_with(
                    &exe,
                    seeded(&shuffled),
                    None,
                    &NoGuards,
                    EvalOptions::default(),
                    RoundExecution::Sequential,
                    JoinStrategy::ForcedBinary,
                )
                .expect("small exhaustive model completes");
                assert_eq!(
                    planned.facts().facts_sorted(),
                    binary.facts().facts_sorted()
                );
                assert_eq!(planned.derivations(), binary.derivations());
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
            rel: &rel,
            depth: &depth,
            assumed: &[],
        };
        let entry = RuleEntry {
            index: 0,
            rule: &rule,
            plan: &plan,
            runtime: &runtime,
            delta: Delta::all(input.len()),
        };
        let buffer = evaluate_rule(
            entry,
            snapshot,
            JoinStrategy::Planned,
            StepGovernor::new(3),
            &NoGuards,
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
                EvalOptions::default(),
                RoundExecution::Sequential,
                JoinStrategy::Planned,
            )
            .expect("the factor model completes");
            let binary = evaluate_with(
                &exe,
                seeded(&input),
                None,
                &NoGuards,
                EvalOptions::default(),
                RoundExecution::Sequential,
                JoinStrategy::ForcedBinary,
            )
            .expect("the exhaustive model completes");
            assert_eq!(planned.derivations(), binary.derivations());
            assert_eq!(planned.derivations().len(), 16);
            for proof in planned.derivations() {
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
                EvalOptions::default(),
                RoundExecution::Sequential,
                JoinStrategy::Planned,
            )
            .expect("factor evaluation completes");
            let binary = evaluate_with(
                &exe,
                seeded(&input),
                None,
                &NoGuards,
                EvalOptions::default(),
                RoundExecution::Sequential,
                JoinStrategy::ForcedBinary,
            )
            .expect("exhaustive evaluation completes");
            assert_eq!(planned.derivations(), binary.derivations());
            assert_eq!(planned.derivations()[0].sources[0].subject, surface("a"));
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
                    rel: &rel,
                    depth: &depth,
                    assumed: &[],
                };
                let entry = RuleEntry {
                    index: 0,
                    rule: &rule,
                    plan: &plan,
                    runtime: &runtime,
                    delta: Delta::all(input.len()),
                };
                let planned = evaluate_rule(
                    entry,
                    snapshot,
                    JoinStrategy::Planned,
                    StepGovernor::new(u64::MAX),
                    &NoGuards,
                )
                .expect("factored round completes");
                let binary = evaluate_rule(
                    entry,
                    snapshot,
                    JoinStrategy::ForcedBinary,
                    StepGovernor::new(u64::MAX),
                    &NoGuards,
                )
                .expect("exhaustive round completes");
                assert_eq!(planned.entries, binary.entries);
                let winner = planned.entries.values().next().expect("one head fires");
                assert_eq!(winner.source_facts(&rel)[1].subject, surface("b1"));
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
                .expect("unique source");
            let mut solution = SlotSolution::empty(0);
            solution.sources.push(SourceRow {
                body_index,
                subject: rel.term_id(&subject).expect("interned subject"),
                predicate: rel.term_id(&predicate).expect("interned predicate"),
                object: rel.term_id(&object).expect("interned object"),
                graph: rel
                    .term_id(RelationStore::DEFAULT_GRAPH)
                    .expect("interned graph"),
                row,
            });
            solution
        };
        let lexical = solution("a", 0);
        let summed = solution("z", 0);
        assert!(source_preference(&lexical.sources, 0, &lexical.sources, 1, &rel).is_lt());
        assert!(source_preference(&lexical.sources, 9, &summed.sources, 0, &rel).is_lt());
        let frontier = Frontier::new(
            vec![
                Witness {
                    solution: &lexical,
                    maximum: u32::MAX,
                    sum: u64::MAX,
                },
                Witness {
                    solution: &summed,
                    maximum: u32::MAX - 1,
                    sum: u64::MAX - 1,
                },
            ],
            &rel,
        );
        assert_eq!(frontier.at(u32::MAX - 1).lexical.solution, &summed);
        assert_eq!(frontier.at(u32::MAX).summed.solution, &summed);
        assert_eq!(frontier.at(u32::MAX).lexical.solution, &lexical);
        assert!(saturated_sum([frontier.at(u32::MAX).summed.sum, 1]));
        assert_eq!(
            source_heights(&lexical.sources, &[u32::MAX, u32::MAX]),
            (u32::MAX, u64::from(u32::MAX))
        );
    }
}

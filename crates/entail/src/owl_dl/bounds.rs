// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Source-backed, class-local cardinality contradictions, prepared with the TBox.
//!
//! A contradictory class denotes the empty set. It is not an assertion that the
//! class has an instance: the compiled clause still needs a CURRENT graph membership.
//! Neither named-individual identities nor branch/successor state enter this table.

use core::mem::size_of;
use purrdf_lex::walk::VecReserve;

use super::Kb;
use super::concept::{Decomp, Role};

/// Caller-selected ceilings on schema preparation; absent ceilings are unlimited.
///
/// Work counts native closure/edge/bound checks. Storage covers both retained
/// products and live construction buffers, including the old buffer during growth.
/// These are resource controls, never a different entailment law.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SchemaPreparationBudget {
    /// Maximum counted preparation operations.
    pub work: Option<u64>,
    /// Maximum concurrently admitted preparation bytes.
    pub bytes: Option<usize>,
}

/// Why a schema entry could not be prepared completely.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaObstruction {
    /// The next native operation exceeded the caller's work ceiling.
    Work {
        /// Operations already completed.
        spent: u64,
        /// Caller-selected maximum.
        limit: u64,
    },
    /// A checked native allocation layout exceeded the caller's storage ceiling.
    Storage {
        /// Bytes needed while both original and replacement buffers are live.
        requested: usize,
        /// Caller-selected maximum.
        limit: usize,
    },
    /// A native layout was not physically representable or its allocation failed.
    Allocation,
}

/// Deterministic accounting of the owning knowledge base's schema preparation.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SchemaPreparationStats {
    /// Class entries completed, including complete entries with no contradiction.
    pub classes: u64,
    /// Restriction descriptors examined, across those entries.
    pub restrictions: u64,
    /// Retained contradiction proofs (not a per-individual count).
    pub contradictions: u64,
    /// Native operations spent, including an interrupted preparation's prefix.
    pub work: u64,
    /// Successful native buffer allocations/growths.
    pub allocations: u64,
    /// Actual capacity bytes retained by the preparation.
    pub retained_bytes: usize,
    /// Largest admitted live byte total, including construction and growth overlap.
    pub peak_bytes: usize,
    /// An incomplete preparation is never an exhaustive no-clash result.
    pub obstruction: Option<SchemaObstruction>,
}

/// One checked class-implication step in a schema proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SchemaStep {
    /// Concept the step establishes.
    pub(crate) concept: u32,
    /// Its source-backed justification.
    pub(crate) rule: SchemaRule,
}

/// Native proof rules; each is checked against the consumer's own concept/TBox table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SchemaRule {
    /// The explicitly selected class/qualifier, not an assertion of existence.
    Assume,
    /// The object domain's universal class.
    Top,
    /// A conjunct of an already established conjunction.
    Conjunct(u32),
    /// A conjunction whose every member was already established.
    Conjunction,
    /// The consequent of this source TBox inclusion.
    Inclusion(usize),
}

impl SchemaStep {
    /// The concept established by this step, in the input-bound native table.
    #[must_use]
    pub const fn concept(&self) -> u32 {
        self.concept
    }

    /// A source TBox inclusion index, when this step uses one.
    #[must_use]
    pub const fn inclusion(&self) -> Option<usize> {
        match self.rule {
            SchemaRule::Inclusion(index) => Some(index),
            _ => None,
        }
    }
}

/// The upper side of a prepared cardinality contradiction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaUpperBound {
    /// This exact maximum restriction, established by the class proof.
    Restriction(u32),
    /// The exact size of the lower restriction's concrete value space.
    DataExtent(u64),
}

/// Why the lower qualifier's values are included in the upper qualifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum QualifierProof {
    /// Exactly the same structural qualifier.
    Identity,
    /// The upper restriction counts every value of the role.
    Universal,
    /// A source-backed object-class implication proof.
    Object(Vec<SchemaStep>),
    /// Exact concrete value-space containment, independently recomputed by the checker.
    Data,
}

/// One source role-inclusion/inverse step, never an ABox edge or equality.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RoleStep {
    pub(crate) from: Role,
    pub(crate) to: Role,
}

/// A class-local contradiction with its exact restrictions and source implications.
///
/// The current inhabitant is deliberately absent here. It belongs to the recorded
/// clash's graph witness, so an uninstantiated class cannot make an ontology inconsistent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaBoundEvidence {
    pub(crate) class: u32,
    pub(crate) lower: u32,
    pub(crate) upper: SchemaUpperBound,
    pub(crate) class_steps: Vec<SchemaStep>,
    pub(crate) qualifier: QualifierProof,
    pub(crate) roles: Vec<RoleStep>,
}

impl SchemaBoundEvidence {
    /// The class this proof establishes empty, in the selected ontology's concept table.
    #[must_use]
    pub const fn class(&self) -> u32 {
        self.class
    }

    /// The exact minimum restriction (an existential has minimum one).
    #[must_use]
    pub const fn lower_restriction(&self) -> u32 {
        self.lower
    }

    /// The exact maximum restriction or finite concrete value-space upper bound.
    #[must_use]
    pub const fn upper_bound(&self) -> SchemaUpperBound {
        self.upper
    }

    /// Every source implication needed to establish the two restrictions.
    #[must_use]
    pub fn class_derivation(&self) -> &[SchemaStep] {
        &self.class_steps
    }

    /// Source implications needed for an object qualifier inclusion.
    #[must_use]
    pub fn qualifier_derivation(&self) -> &[SchemaStep] {
        match &self.qualifier {
            QualifierProof::Object(steps) => steps,
            _ => &[],
        }
    }

    /// Actual owned vector capacity bytes, excluding this inline value.
    pub(crate) fn heap_bytes(&self) -> usize {
        self.class_steps.capacity() * size_of::<SchemaStep>()
            + self.roles.capacity() * size_of::<RoleStep>()
            + match &self.qualifier {
                QualifierProof::Object(steps) => steps.capacity() * size_of::<SchemaStep>(),
                _ => 0,
            }
    }

    /// Independently check the proof's steps, bounds and compatible qualification.
    /// No producer closure/cache or hypertableau driver is called here.
    pub(crate) fn check(&self, kb: &Kb) -> Result<bool, super::proof::DlProofError> {
        let mut poll = || {
            if kb
                .stop
                .as_deref()
                .is_some_and(purrdf_datalog::StopSignal::stopped)
            {
                Err(super::proof::DlProofError::Stopped)
            } else {
                Ok(())
            }
        };
        let mut products = Products::new(
            SchemaPreparationBudget::default(),
            SchemaPreparationStats::default(),
            0,
            &mut poll,
        );
        match products.range(|storage| self.check_with(kb, storage)) {
            Ok(value) => Ok(value),
            Err(Failure::Obstruction(obstruction)) => {
                Err(super::proof::DlProofError::SchemaPreparation { obstruction })
            }
            Err(Failure::Boundary(error)) => Err(error),
        }
    }

    fn check_with(
        &self,
        kb: &Kb,
        storage: &mut dyn purrdf_xsd::range::Storage,
    ) -> Result<bool, purrdf_xsd::range::StorageError> {
        storage.work(1)?;
        if self.class as usize >= kb.table.len() || self.lower as usize >= kb.table.len() {
            return Ok(false);
        }
        let Some(lower) = lower_bound(kb, self.lower) else {
            return Ok(false);
        };
        if !check_steps(kb, self.class, &self.class_steps, self.lower, storage)? {
            return Ok(false);
        }
        Ok(match self.upper {
            SchemaUpperBound::DataExtent(extent) => {
                let data = match kb.table.decomp(lower.filler) {
                    Decomp::Data(id) => Some(*id),
                    _ => None,
                };
                data.is_some()
                    && kb
                        .data_ranges
                        .try_exact_cardinality(data.expect("checked data range"), storage)?
                        == Some(extent)
                    && u64::from(lower.count) > extent
                    && self.roles.is_empty()
                    && self.qualifier == QualifierProof::Identity
            }
            SchemaUpperBound::Restriction(upper_id) => {
                if upper_id as usize >= kb.table.len() {
                    return Ok(false);
                }
                let Some(upper) = upper_bound(kb, upper_id) else {
                    return Ok(false);
                };
                if lower.count <= upper.count
                    || !check_steps(kb, self.class, &self.class_steps, upper_id, storage)?
                    || !check_role_path(kb, lower.role, upper.role, &self.roles, storage)?
                {
                    return Ok(false);
                }
                match &self.qualifier {
                    QualifierProof::Identity => lower.filler == upper.filler,
                    QualifierProof::Universal => upper.filler == kb.top,
                    QualifierProof::Object(steps) => {
                        object_qualifier(kb, lower.filler)
                            && object_qualifier(kb, upper.filler)
                            && check_steps(kb, lower.filler, steps, upper.filler, storage)?
                    }
                    QualifierProof::Data => {
                        match (kb.table.decomp(lower.filler), kb.table.decomp(upper.filler)) {
                            (Decomp::Data(sub), Decomp::Data(sup)) => {
                                kb.data_ranges.try_exactly_contains(*sub, *sup, storage)?
                            }
                            _ => false,
                        }
                    }
                }
            }
        })
    }
}

/// Step checking is a linear scan of a supplied derivation, not a closure search.
fn check_steps(
    kb: &Kb,
    root: u32,
    steps: &[SchemaStep],
    target: u32,
    storage: &mut dyn purrdf_xsd::range::Storage,
) -> Result<bool, purrdf_xsd::range::StorageError> {
    let contains =
        |prefix: &[SchemaStep], concept| prefix.iter().any(|step| step.concept == concept);
    for (index, step) in steps.iter().enumerate() {
        storage.work(1)?;
        if step.concept as usize >= kb.table.len() || contains(&steps[..index], step.concept) {
            return Ok(false);
        }
        let prefix = &steps[..index];
        let valid = match step.rule {
            SchemaRule::Assume => step.concept == root,
            SchemaRule::Top => step.concept == kb.top,
            SchemaRule::Conjunct(parent) => {
                contains(prefix, parent)
                    && matches!(kb.table.decomp(parent), Decomp::And(children) if children.contains(&step.concept))
            }
            SchemaRule::Conjunction => {
                matches!(kb.table.decomp(step.concept), Decomp::And(children) if children.iter().all(|&child| contains(prefix, child)))
            }
            SchemaRule::Inclusion(axiom) => kb
                .tbox
                .get(axiom)
                .is_some_and(|&(sub, sup)| sup == step.concept && contains(prefix, sub)),
        };
        if !valid {
            return Ok(false);
        }
    }
    Ok(contains(steps, target))
}

/// Verify a source role step without running the producer's path search.
fn role_step_valid(kb: &Kb, from: Role, to: Role) -> bool {
    match (from, to) {
        (Role::Named(sub), Role::Named(sup)) | (Role::Inv(sub), Role::Inv(sup)) => kb
            .role_sub
            .get(&sup)
            .is_some_and(|members| members.contains(&sub)),
        (Role::Named(left), Role::Inv(right)) | (Role::Inv(left), Role::Named(right)) => kb
            .inverses
            .get(&left)
            .is_some_and(|partners| partners.contains(&right)),
    }
}

fn check_role_path(
    kb: &Kb,
    from: Role,
    to: Role,
    steps: &[RoleStep],
    storage: &mut dyn purrdf_xsd::range::Storage,
) -> Result<bool, purrdf_xsd::range::StorageError> {
    let mut current = from;
    for step in steps {
        storage.work(1)?;
        if step.from != current || !role_step_valid(kb, step.from, step.to) {
            return Ok(false);
        }
        current = step.to;
    }
    Ok(current == to)
}

#[derive(Debug, Default)]
pub(crate) enum Entry {
    #[default]
    Unprepared,
    Clear,
    Clash(SchemaBoundEvidence),
}

/// Intrinsic to one owning, finalized schema. There is no global key or dataset pin.
#[derive(Debug, Default)]
pub(crate) struct Preparation {
    pub(crate) entries: Vec<Entry>,
    /// Original admitted guard clauses, in ascending class order. Clausification
    /// borrows these products instead of allocating a second body or trigger index.
    pub(crate) guards: Vec<(u32, super::clause::DlClause)>,
    pub(crate) stats: SchemaPreparationStats,
}

impl Preparation {
    pub(crate) fn evidence(&self, class: u32) -> Option<&SchemaBoundEvidence> {
        match self.entries.get(class as usize)? {
            Entry::Clash(evidence) => Some(evidence),
            Entry::Clear | Entry::Unprepared => None,
        }
    }

    fn heap_bytes(&self) -> usize {
        self.entries.capacity() * size_of::<Entry>()
            + self.guards.capacity() * size_of::<(u32, super::clause::DlClause)>()
            + self
                .guards
                .iter()
                .map(|(_, clause)| clause.body.capacity() * size_of::<super::clause::BodyAtom>())
                .sum::<usize>()
            + self
                .entries
                .iter()
                .map(|entry| match entry {
                    Entry::Clash(evidence) => evidence.heap_bytes(),
                    _ => 0,
                })
                .sum::<usize>()
    }

    /// Extend only newly interned class entries. A failed entry stays unprepared.
    pub(crate) fn extend_until<E>(
        &mut self,
        kb: &Kb,
        budget: SchemaPreparationBudget,
        poll: &mut impl FnMut() -> Result<(), E>,
    ) -> Result<(), E> {
        self.extend_selected(kb, budget, poll, None)
    }

    /// The uncached measurement control uses this same producer for one CURRENT
    /// class membership. It is compiled only in the owning crate's tests.
    #[cfg(test)]
    pub(crate) fn current_class(kb: &Kb, class: u32) -> Self {
        let mut preparation = Self::default();
        preparation
            .extend_selected(
                kb,
                SchemaPreparationBudget::default(),
                &mut || Ok::<(), core::convert::Infallible>(()),
                Some(class),
            )
            .unwrap();
        preparation
    }

    fn extend_selected<E>(
        &mut self,
        kb: &Kb,
        budget: SchemaPreparationBudget,
        poll: &mut impl FnMut() -> Result<(), E>,
        selected: Option<u32>,
    ) -> Result<(), E> {
        if self.stats.obstruction.is_some() {
            return Ok(());
        }
        // No cardinality obligation means no preparation, retained rows or scratch.
        if self.entries.is_empty()
            && !(0..kb.table.len()).any(|id| lower_bound(kb, id as u32).is_some())
        {
            return Ok(());
        }
        // A warm table does not allocate scratch or revisit a completed class.
        if self.entries.len() == kb.table.len()
            && self.entries.iter().enumerate().all(|(id, entry)| {
                !matches!(entry, Entry::Unprepared)
                    || !matches!(kb.table.decomp(id as u32), Decomp::Named | Decomp::Top)
            })
        {
            return Ok(());
        }
        let retained = self.heap_bytes();
        let mut memory = Products::new(budget, self.stats, retained, poll);
        let result = (|| {
            while self.entries.len() < kb.table.len() {
                memory.push(&mut self.entries, Entry::Unprepared)?;
            }
            let mut closure = Closure::new(kb.table.len(), &mut memory)?;
            let mut qualifier = Closure::new(kb.table.len(), &mut memory)?;
            let index = Implications::new(kb, &mut memory)?;
            let mut lower = Vec::new();
            let mut upper = Vec::new();
            let mut role_queue = Vec::new();
            let mut roles = Vec::new();
            let mut needed = Vec::new();
            for (id, entry) in self.entries.iter_mut().enumerate() {
                if selected.is_some_and(|class| class as usize != id)
                    || !matches!(entry, Entry::Unprepared)
                    || !matches!(kb.table.decomp(id as u32), Decomp::Named | Decomp::Top)
                {
                    continue;
                }
                memory.step()?;
                closure.close(kb, &index, id as u32, &mut memory)?;
                lower.clear();
                upper.clear();
                for step in &closure.steps {
                    memory.step()?;
                    if let Some(bound) = lower_bound(kb, step.concept) {
                        memory.push(&mut lower, bound)?;
                    }
                    if let Some(bound) = upper_bound(kb, step.concept) {
                        memory.push(&mut upper, bound)?;
                    }
                }
                memory.stats.restrictions += (lower.len() + upper.len()) as u64;
                let mut evidence = None;
                'bounds: for min in &lower {
                    if let Decomp::Data(range) = *kb.table.decomp(min.filler) {
                        memory.step()?;
                        if let Some(extent) = memory
                            .range(|storage| kb.data_ranges.try_exact_cardinality(range, storage))?
                            && u64::from(min.count) > extent
                        {
                            evidence = Some(SchemaBoundEvidence {
                                class: id as u32,
                                lower: min.restriction,
                                upper: SchemaUpperBound::DataExtent(extent),
                                class_steps: closure.proof(
                                    kb,
                                    &[min.restriction],
                                    &mut needed,
                                    &mut memory,
                                )?,
                                qualifier: QualifierProof::Identity,
                                roles: Vec::new(),
                            });
                            break;
                        }
                    }
                    for max in &upper {
                        memory.step()?;
                        if min.count <= max.count {
                            continue;
                        }
                        if !role_path(
                            kb,
                            min.role,
                            max.role,
                            &mut role_queue,
                            &mut roles,
                            &mut memory,
                        )? {
                            continue;
                        }
                        let relation = if min.filler == max.filler {
                            Some(QualifierProof::Identity)
                        } else if max.filler == kb.top {
                            Some(QualifierProof::Universal)
                        } else {
                            match (kb.table.decomp(min.filler), kb.table.decomp(max.filler)) {
                                (Decomp::Data(sub), Decomp::Data(sup)) => {
                                    memory.step()?;
                                    memory
                                        .range(|storage| {
                                            kb.data_ranges.try_exactly_contains(*sub, *sup, storage)
                                        })?
                                        .then_some(QualifierProof::Data)
                                }
                                (Decomp::Data(_) | Decomp::NegData(_), _)
                                | (_, Decomp::Data(_) | Decomp::NegData(_)) => None,
                                _ if object_qualifier(kb, min.filler)
                                    && object_qualifier(kb, max.filler) =>
                                {
                                    qualifier.close(kb, &index, min.filler, &mut memory)?;
                                    if qualifier.seen[max.filler as usize].is_some() {
                                        Some(QualifierProof::Object(qualifier.proof(
                                            kb,
                                            &[max.filler],
                                            &mut needed,
                                            &mut memory,
                                        )?))
                                    } else {
                                        None
                                    }
                                }
                                _ => None,
                            }
                        };
                        let Some(relation) = relation else { continue };
                        let class_steps = closure.proof(
                            kb,
                            &[min.restriction, max.restriction],
                            &mut needed,
                            &mut memory,
                        )?;
                        let saved_roles = memory.copy(&roles)?;
                        evidence = Some(SchemaBoundEvidence {
                            class: id as u32,
                            lower: min.restriction,
                            upper: SchemaUpperBound::Restriction(max.restriction),
                            class_steps,
                            qualifier: relation,
                            roles: saved_roles,
                        });
                        break 'bounds;
                    }
                }
                if let Some(evidence) = evidence {
                    let class = id as u32;
                    let mut body = Vec::new();
                    memory.push(
                        &mut body,
                        super::clause::BodyAtom::Concept {
                            var: 0,
                            concept: class,
                        },
                    )?;
                    memory.push(
                        &mut self.guards,
                        (
                            class,
                            super::clause::DlClause {
                                body,
                                head: Vec::new(),
                            },
                        ),
                    )?;
                    *entry = Entry::Clash(evidence);
                } else {
                    *entry = Entry::Clear;
                }
                memory.stats.classes += 1;
                memory.stats.contradictions += u64::from(matches!(entry, Entry::Clash(_)));
            }
            Ok(())
        })();
        // All construction buffers die at the closure above, before their admission
        // is released. Only original retained products remain in the owning table.
        memory.stats.retained_bytes = self.heap_bytes();
        self.stats = memory.stats;
        match result {
            Ok(()) => Ok(()),
            Err(Failure::Boundary(error)) => Err(error),
            Err(Failure::Obstruction(obstruction)) => {
                self.stats.obstruction = Some(obstruction);
                Ok(())
            }
        }
    }
}

/// TBox implications and owl:Thing's seed apply only to object-domain qualifiers.
/// A boolean combination containing concrete range leaves is not silently treated
/// as an object class (in particular when an opaque range is one operand).
fn object_qualifier(kb: &Kb, id: u32) -> bool {
    match kb.table.decomp(id) {
        Decomp::Data(_) | Decomp::NegData(_) => false,
        Decomp::And(children) | Decomp::Or(children) => {
            children.iter().all(|&child| object_qualifier(kb, child))
        }
        _ => true,
    }
}

#[derive(Clone, Copy)]
struct Bound {
    restriction: u32,
    count: u32,
    role: Role,
    filler: u32,
}

fn lower_bound(kb: &Kb, restriction: u32) -> Option<Bound> {
    let (count, role, filler) = match *kb.table.decomp(restriction) {
        Decomp::Min(count, role, filler) => (count, role, filler),
        Decomp::Some(role, filler) => (1, role, filler),
        _ => return None,
    };
    Some(Bound {
        restriction,
        count,
        role,
        filler,
    })
}

fn upper_bound(kb: &Kb, restriction: u32) -> Option<Bound> {
    let Decomp::Max(count, role, filler) = *kb.table.decomp(restriction) else {
        return None;
    };
    Some(Bound {
        restriction,
        count,
        role,
        filler,
    })
}

pub(super) enum Failure<E> {
    Boundary(E),
    Obstruction(SchemaObstruction),
}

/// The preparation's original native product admission, shared by every builder.
pub(super) struct Products<'a, E, P: FnMut() -> Result<(), E>> {
    budget: SchemaPreparationBudget,
    pub(super) stats: SchemaPreparationStats,
    pub(super) live: usize,
    poll: &'a mut P,
}

impl<'a, E, P: FnMut() -> Result<(), E>> Products<'a, E, P> {
    pub(super) const fn new(
        budget: SchemaPreparationBudget,
        stats: SchemaPreparationStats,
        live: usize,
        poll: &'a mut P,
    ) -> Self {
        Self {
            budget,
            stats,
            live,
            poll,
        }
    }

    fn step(&mut self) -> Result<(), Failure<E>> {
        self.charge(1)
    }

    fn charge(&mut self, work: u64) -> Result<(), Failure<E>> {
        (self.poll)().map_err(Failure::Boundary)?;
        let next = self
            .stats
            .work
            .checked_add(work)
            .ok_or(Failure::Obstruction(SchemaObstruction::Allocation))?;
        if let Some(limit) = self.budget.work
            && next > limit
        {
            return Err(Failure::Obstruction(SchemaObstruction::Work {
                spent: self.stats.work,
                limit,
            }));
        }
        self.stats.work = next;
        Ok(())
    }

    fn range<T>(
        &mut self,
        run: impl FnOnce(
            &mut dyn purrdf_xsd::range::Storage,
        ) -> Result<T, purrdf_xsd::range::StorageError>,
    ) -> Result<T, Failure<E>> {
        let base = self.live;
        let mut storage = RangeProducts {
            products: self,
            base,
            failure: None,
        };
        let result = run(&mut storage);
        if let Some(failure) = storage.failure {
            return Err(failure);
        }
        result.map_err(|_| Failure::Obstruction(SchemaObstruction::Allocation))
    }

    fn admit(&mut self, bytes: usize) -> Result<(), Failure<E>> {
        let total = self
            .live
            .checked_add(bytes)
            .ok_or(Failure::Obstruction(SchemaObstruction::Allocation))?;
        if let Some(limit) = self.budget.bytes
            && total > limit
        {
            return Err(Failure::Obstruction(SchemaObstruction::Storage {
                requested: total,
                limit,
            }));
        }
        self.stats.peak_bytes = self.stats.peak_bytes.max(total);
        Ok(())
    }

    pub(super) fn boxed<T>(&mut self, value: T) -> Result<Box<T>, Failure<E>> {
        let bytes = size_of::<T>();
        self.admit(bytes)?;
        let value = purrdf_core::small::try_boxed(value)
            .map_err(|_| Failure::Obstruction(SchemaObstruction::Allocation))?;
        self.live += bytes;
        self.stats.allocations += u64::from(bytes != 0);
        Ok(value)
    }
}

impl<E, P: FnMut() -> Result<(), E>> VecReserve for Products<'_, E, P> {
    type Error = Failure<E>;

    fn reserve<T>(&mut self, values: &mut Vec<T>, additional: usize) -> Result<(), Failure<E>> {
        let needed = values
            .len()
            .checked_add(additional)
            .ok_or(Failure::Obstruction(SchemaObstruction::Allocation))?;
        if needed <= values.capacity() {
            return Ok(());
        }
        let capacity = needed
            .max(
                values
                    .capacity()
                    .checked_mul(2)
                    .ok_or(Failure::Obstruction(SchemaObstruction::Allocation))?,
            )
            .max(1);
        let bytes = capacity
            .checked_mul(size_of::<T>())
            .filter(|&bytes| isize::try_from(bytes).is_ok())
            .ok_or(Failure::Obstruction(SchemaObstruction::Allocation))?;
        self.admit(bytes)?;
        let old = values.capacity() * size_of::<T>();
        values
            .try_reserve_exact(capacity - values.len())
            .map_err(|_| Failure::Obstruction(SchemaObstruction::Allocation))?;
        self.live = self.live - old + values.capacity() * size_of::<T>();
        self.stats.allocations += 1;
        Ok(())
    }
}

/// The concrete-domain owner extends the same preparation grant. It receives
/// actual range allocations, never a class-table or lexical-size proxy.
struct RangeProducts<'a, 'p, E, P: FnMut() -> Result<(), E>> {
    products: &'a mut Products<'p, E, P>,
    base: usize,
    failure: Option<Failure<E>>,
}

impl<E, P: FnMut() -> Result<(), E>> purrdf_xsd::range::Storage for RangeProducts<'_, '_, E, P> {
    fn work(&mut self, operations: u64) -> Result<(), purrdf_xsd::range::StorageError> {
        if self.failure.is_some() {
            return Err(purrdf_xsd::range::StorageError::Refused);
        }
        if let Err(failure) = self.products.charge(operations) {
            self.failure = Some(failure);
            return Err(purrdf_xsd::range::StorageError::Refused);
        }
        Ok(())
    }
    fn resize(&mut self, bytes: usize) -> Result<(), purrdf_xsd::range::StorageError> {
        let Some(total) = self.base.checked_add(bytes) else {
            if self.failure.is_none() {
                self.failure = Some(Failure::Obstruction(SchemaObstruction::Allocation));
            }
            return Err(purrdf_xsd::range::StorageError::Allocation);
        };
        // Shrink after actual value destruction remains possible through refusal,
        // so cleanup neither retains scratch nor replaces the first cause.
        if total > self.products.live {
            if self.failure.is_some() {
                return Err(purrdf_xsd::range::StorageError::Refused);
            }
            if let Some(limit) = self.products.budget.bytes
                && total > limit
            {
                self.failure = Some(Failure::Obstruction(SchemaObstruction::Storage {
                    requested: total,
                    limit,
                }));
                return Err(purrdf_xsd::range::StorageError::Refused);
            }
        }
        self.products.stats.peak_bytes = self.products.stats.peak_bytes.max(total);
        self.products.live = total;
        Ok(())
    }
    fn allocated(&mut self) {
        self.products.stats.allocations += 1;
    }
}

#[derive(Clone, Copy)]
enum Consequence {
    Conjunct(u32),
    Conjunction(u32),
    Inclusion(usize),
}

struct Implications {
    edges: Vec<Vec<Consequence>>,
}

impl Implications {
    fn new<E>(
        kb: &Kb,
        memory: &mut Products<'_, E, impl FnMut() -> Result<(), E>>,
    ) -> Result<Self, Failure<E>> {
        let mut edges = Vec::new();
        memory.reserve(&mut edges, kb.table.len())?;
        edges.resize_with(kb.table.len(), Vec::new);
        for id in 0..kb.table.len() {
            memory.step()?;
            if let Decomp::And(children) = kb.table.decomp(id as u32) {
                for &child in children {
                    memory.step()?;
                    memory.push(&mut edges[id], Consequence::Conjunct(child))?;
                    memory.push(
                        &mut edges[child as usize],
                        Consequence::Conjunction(id as u32),
                    )?;
                }
            }
        }
        for (axiom, &(sub, _)) in kb.tbox.iter().enumerate() {
            memory.step()?;
            memory.push(&mut edges[sub as usize], Consequence::Inclusion(axiom))?;
        }
        Ok(Self { edges })
    }
}

struct Closure {
    seen: Vec<Option<usize>>,
    steps: Vec<SchemaStep>,
}

impl Closure {
    fn new<E>(
        count: usize,
        memory: &mut Products<'_, E, impl FnMut() -> Result<(), E>>,
    ) -> Result<Self, Failure<E>> {
        let mut seen = Vec::new();
        memory.reserve(&mut seen, count)?;
        seen.resize(count, None);
        Ok(Self {
            seen,
            steps: Vec::new(),
        })
    }

    fn add<E>(
        &mut self,
        concept: u32,
        rule: SchemaRule,
        memory: &mut Products<'_, E, impl FnMut() -> Result<(), E>>,
    ) -> Result<(), Failure<E>> {
        if self.seen[concept as usize].is_none() {
            let index = self.steps.len();
            memory.push(&mut self.steps, SchemaStep { concept, rule })?;
            self.seen[concept as usize] = Some(index);
        }
        Ok(())
    }

    fn close<E>(
        &mut self,
        kb: &Kb,
        implications: &Implications,
        root: u32,
        memory: &mut Products<'_, E, impl FnMut() -> Result<(), E>>,
    ) -> Result<(), Failure<E>> {
        self.seen.fill(None);
        self.steps.clear();
        self.add(root, SchemaRule::Assume, memory)?;
        self.add(kb.top, SchemaRule::Top, memory)?;
        let mut next = 0;
        while next < self.steps.len() {
            memory.step()?;
            let from = self.steps[next].concept;
            next += 1;
            for consequence in &implications.edges[from as usize] {
                memory.step()?;
                match *consequence {
                    Consequence::Conjunct(child) => {
                        self.add(child, SchemaRule::Conjunct(from), memory)?;
                    }
                    Consequence::Conjunction(parent) => {
                        if let Decomp::And(children) = kb.table.decomp(parent)
                            && children
                                .iter()
                                .all(|&child| self.seen[child as usize].is_some())
                        {
                            self.add(parent, SchemaRule::Conjunction, memory)?;
                        }
                    }
                    Consequence::Inclusion(axiom) => {
                        self.add(kb.tbox[axiom].1, SchemaRule::Inclusion(axiom), memory)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Retain only the source DAG needed for the selected restriction(s).
    fn proof<E>(
        &self,
        kb: &Kb,
        targets: &[u32],
        needed: &mut Vec<bool>,
        memory: &mut Products<'_, E, impl FnMut() -> Result<(), E>>,
    ) -> Result<Vec<SchemaStep>, Failure<E>> {
        needed.clear();
        memory.reserve(needed, self.steps.len())?;
        needed.resize(self.steps.len(), false);
        for &target in targets {
            needed[self.seen[target as usize].expect("proof target reached by closure")] = true;
        }
        for index in (0..self.steps.len()).rev() {
            memory.step()?;
            if !needed[index] {
                continue;
            }
            let step = self.steps[index];
            let mut require = |concept: u32| {
                needed[self.seen[concept as usize].expect("source precedes its consequence")] =
                    true;
            };
            match step.rule {
                SchemaRule::Assume | SchemaRule::Top => {}
                SchemaRule::Conjunct(parent) => require(parent),
                SchemaRule::Conjunction => {
                    if let Decomp::And(children) = kb.table.decomp(step.concept) {
                        for &child in children {
                            require(child);
                        }
                    }
                }
                SchemaRule::Inclusion(axiom) => require(kb.tbox[axiom].0),
            }
        }
        let mut proof = Vec::new();
        for (index, &step) in self.steps.iter().enumerate() {
            if needed[index] {
                memory.push(&mut proof, step)?;
            }
        }
        Ok(proof)
    }
}

/// Source-only role reachability. No transitive edge or successor graph is read.
fn role_path<E>(
    kb: &Kb,
    from: Role,
    to: Role,
    queue: &mut Vec<(Role, Option<usize>)>,
    path: &mut Vec<RoleStep>,
    memory: &mut Products<'_, E, impl FnMut() -> Result<(), E>>,
) -> Result<bool, Failure<E>> {
    path.clear();
    if from == to {
        return Ok(true);
    }
    queue.clear();
    memory.push(queue, (from, None))?;
    let mut next = 0;
    while next < queue.len() {
        memory.step()?;
        let (current, _) = queue[next];
        let index = next;
        next += 1;
        for role in kb
            .role_sub
            .keys()
            .chain(kb.inverses.keys())
            .copied()
            .flat_map(|id| [Role::Named(id), Role::Inv(id)])
            .chain([to])
        {
            memory.step()?;
            if !role_step_valid(kb, current, role) || queue.iter().any(|&(seen, _)| seen == role) {
                continue;
            }
            memory.push(queue, (role, Some(index)))?;
            if role == to {
                let mut at = queue.len() - 1;
                while let Some(previous) = queue[at].1 {
                    memory.push(
                        path,
                        RoleStep {
                            from: queue[previous].0,
                            to: queue[at].0,
                        },
                    )?;
                    at = previous;
                }
                path.reverse();
                return Ok(true);
            }
        }
    }
    Ok(false)
}

#[cfg(test)]
mod owner_tests {
    use super::*;
    use crate::reasoner::SchemaPreparationBudget;
    use crate::reasoner::schema_tests::shared_schema;
    use purrdf_alloc_probe::CurrentThreadWindow;

    #[test]
    fn original_preparation_layouts_cover_allocator_peak_and_retained_evidence() {
        for contradictory in [false, true] {
            let dataset = shared_schema(8, 16, contradictory);
            // Input concepts, exact range tables and source strings precede this
            // measurement. Only the actual preparation owner is being measured.
            let mut kb = Kb::from_dataset(&dataset).unwrap();
            kb.finalize();
            let window = CurrentThreadWindow::open();
            let mut table = Preparation::default();
            table
                .extend_until(&kb, SchemaPreparationBudget::default(), &mut || {
                    Ok::<(), core::convert::Infallible>(())
                })
                .unwrap();
            let measured = window.sample();
            assert_eq!(
                measured.retained_bytes,
                i64::try_from(table.stats.retained_bytes).unwrap()
            );
            assert!(
                measured.peak_working_bytes <= i64::try_from(table.stats.peak_bytes).unwrap(),
                "{measured:?} vs {:?}",
                table.stats
            );
            assert_eq!(measured.allocations, table.stats.allocations);
            assert_eq!(table.guards.is_empty(), !contradictory);
            drop(table);
            assert_eq!(window.sample().retained_bytes, 0);
        }
    }

    #[test]
    fn compiled_schema_guards_borrow_original_admitted_products_without_allocation() {
        let dataset = shared_schema(8, 16, true);
        let mut kb = Kb::from_dataset(&dataset).unwrap();
        kb.finalize();
        assert_ne!(
            kb.schema.guards.len(),
            0,
            "the source has a contradictory class"
        );
        let clauses = super::super::clause::derive(&kb);
        let window = CurrentThreadWindow::open();
        for (index, class) in clauses.schema_clauses(&kb.schema) {
            let ordinal = kb
                .schema
                .guards
                .binary_search_by_key(&class, |(held, _)| *held)
                .unwrap();
            assert!(core::ptr::eq(
                clauses.clause(index, &kb.schema),
                &raw const kb.schema.guards[ordinal].1,
            ));
            assert_eq!(clauses.schema_class(index, &kb.schema), Some(class));
            assert_eq!(clauses.triggered_by(class, &kb.schema).last(), Some(index));
            assert!(clauses.is_tbox(index));
            assert_eq!(clauses.reads(index), 0);
        }
        assert_eq!(window.sample().allocations, 0);
        assert_eq!(window.sample().retained_bytes, 0);
    }
}

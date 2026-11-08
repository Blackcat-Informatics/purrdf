// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Once-read candidate indexes for the original BGP loop. The row binder,
//! selected join order, row sink, checkpoints and final FILTER are unchanged.

use super::{BgpOrder, CompiledPattern, Pos, components_of, emit_virtual_candidates, query_id};
use crate::dataset_spec::GraphScope;
use crate::eval::EvalCtx;
use crate::parsed_value::ParsedValue;
use crate::scratch::SolutionTerm;
use crate::shapes::window::{WindowSpec, compare, parsed};
use crate::solution::{RetainedRow, VarSchema};
use crate::{AdmittedMap, AdmittedVec, EvalError, WorkspaceCapability};
use core::cmp::Ordering;
use purrdf_core::{DatasetView, GraphMatch, QuadIds, ViewTermId};
use std::ops::ControlFlow;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

type Key<I> = (Option<I>, Option<I>, Option<I>);
struct Candidate<I> {
    quad: QuadIds<I>,
    graph: usize,
    virtual_ordinal: Option<usize>,
    numeric: Option<ParsedValue>,
}
struct Bucket {
    ordered: AdmittedVec<usize>,
    numeric: AdmittedVec<usize>,
    unknown: AdmittedVec<usize>,
}
struct Pattern<I> {
    candidates: AdmittedVec<Candidate<I>>,
    buckets: AdmittedMap<Key<I>, Bucket>,
    bound: [bool; 3],
    other: Option<usize>,
    other_subject: Option<usize>,
    new_left: bool,
}
struct Inferred<I: ViewTermId> {
    term: SolutionTerm<I>,
    value: ParsedValue,
}
/// One stage's unique driving values, ordered once and swept against each
/// candidate partition. It keeps only ordinal ranges, never copied RDF cells.
pub(super) struct StageWindow<I: ViewTermId> {
    ranges: AdmittedMap<(Key<I>, SolutionTerm<I>), (usize, usize)>,
}
struct Driver<I: ViewTermId> {
    key: Key<I>,
    term: SolutionTerm<I>,
    value: ParsedValue,
    ordinal: usize,
}
pub(super) struct PreparedWindow<I: ViewTermId> {
    patterns: AdmittedVec<Pattern<I>>,
    inferred: AdmittedMap<(usize, I), Option<Inferred<I>>>,
    spec: WindowSpec,
    pub(super) visits: u64,
    comparisons: AtomicU64,
}

impl<I: ViewTermId> PreparedWindow<I> {
    pub(super) fn new<D: DatasetView<Id = I> + Sync>(
        compiled: &[CompiledPattern<I>],
        working: &VarSchema,
        order: &BgpOrder,
        scope: &GraphScope<I>,
        spec: &WindowSpec,
        reifies: Option<I>,
        ctx: &EvalCtx<'_, D>,
    ) -> Result<Self, EvalError> {
        let left = working
            .index_of(&spec.left)
            .ok_or(EvalError::WorkspaceBoundOverflow)?;
        let right = working
            .index_of(&spec.right)
            .ok_or(EvalError::WorkspaceBoundOverflow)?;
        let subject = |column| {
            let mut subject = None;
            for cp in compiled {
                if matches!(cp.o, Pos::Slot(object) if object == column) {
                    let Pos::Slot(candidate) = cp.s else {
                        return None;
                    };
                    if subject.is_some_and(|previous| previous != candidate) {
                        return None;
                    }
                    subject = Some(candidate);
                }
            }
            subject
        };
        let left_subject = subject(left);
        let right_subject = subject(right);
        let mut masks = AdmittedVec::with_capacity(compiled.len(), &ctx.growth)?;
        for _ in compiled {
            masks.push([false; 3])?;
        }
        let mut bound_columns = AdmittedVec::with_capacity(working.len(), &ctx.growth)?;
        for _ in working.vars() {
            bound_columns.push(false)?;
        }
        for &index in order.iter() {
            let cp = &compiled[index];
            for (slot, position) in [&cp.s, &cp.p, &cp.o].iter().enumerate() {
                masks.as_mut_slice()[index][slot] = match position {
                    Pos::Bound(_) => true,
                    Pos::Slot(column) => bound_columns[*column],
                    Pos::Triple(_) => return Err(EvalError::WorkspaceBoundOverflow),
                };
            }
            for position in [&cp.s, &cp.p, &cp.o] {
                if let Pos::Slot(column) = position {
                    bound_columns.as_mut_slice()[*column] = true;
                }
            }
        }
        let mut result = Self {
            patterns: AdmittedVec::new(&ctx.growth),
            inferred: AdmittedMap::default(),
            spec: spec.clone(),
            visits: 0,
            comparisons: AtomicU64::new(0),
        };
        for (index, cp) in compiled.iter().enumerate() {
            let mut candidates = AdmittedVec::new(&ctx.growth);
            let constants = |position: &Pos<I>| match position {
                Pos::Bound(id) => Some(*id),
                _ => None,
            };
            let s = constants(&cp.s);
            let p = constants(&cp.p);
            let o = constants(&cp.o);
            let components = components_of(&cp.o, constants);
            let mut scan_graph = |ordinal, graph| -> Result<(), EvalError> {
                let mut virtual_ordinal = 0;
                for quad in ctx.dataset.quads_for_pattern(s, p, o, graph) {
                    if ctx.stop_check().is_some() {
                        break;
                    }
                    result.visits = result
                        .visits
                        .checked_add(1)
                        .ok_or(EvalError::WorkspaceBoundOverflow)?;
                    candidates.push(Candidate {
                        quad,
                        graph: ordinal,
                        virtual_ordinal: None,
                        numeric: None,
                    })?;
                }
                if matches!(scope, GraphScope::One(_)) {
                    let mut failure = None;
                    let range_admission = crate::expr::AnnotationRangeMemory::new(ctx);
                    let range = ctx.annotation_range.as_ref().and_then(|ranges| {
                        ranges.iter().find(|range| matches!(&cp.o, Pos::Slot(slot)
                            if working.vars().get(*slot).is_some_and(|variable| range.matches(variable))))
                    });
                    emit_virtual_candidates(
                        ctx.dataset,
                        cp,
                        s,
                        p,
                        o,
                        components,
                        reifies,
                        graph,
                        range.map(|range| range.range(&ctx.now, &range_admission)),
                        |quad| {
                            if failure.is_some() || ctx.stop_check().is_some() {
                                return ControlFlow::Break(());
                            }
                            let added = (|| {
                                result.visits = result
                                    .visits
                                    .checked_add(1)
                                    .ok_or(EvalError::WorkspaceBoundOverflow)?;
                                candidates.push(Candidate {
                                    quad,
                                    graph: ordinal,
                                    virtual_ordinal: Some(virtual_ordinal),
                                    numeric: None,
                                })?;
                                virtual_ordinal = virtual_ordinal
                                    .checked_add(1)
                                    .ok_or(EvalError::WorkspaceBoundOverflow)?;
                                Ok::<_, EvalError>(())
                            })();
                            if let Err(error) = added {
                                failure = Some(error);
                                ControlFlow::Break(())
                            } else {
                                ControlFlow::Continue(())
                            }
                        },
                    );
                    if let Some(error) = range_admission.take_failure().or(failure) {
                        return Err(error);
                    }
                }
                Ok(())
            };
            match scope {
                GraphScope::One(graph) => scan_graph(0, *graph)?,
                GraphScope::Merge(graphs) => {
                    for (ordinal, &graph) in graphs.iter().enumerate() {
                        scan_graph(ordinal, GraphMatch::Named(graph))?;
                    }
                }
            }
            let graph_of = |ordinal| match scope {
                GraphScope::One(graph) => *graph,
                GraphScope::Merge(graphs) => GraphMatch::Named(graphs[ordinal]),
            };
            let mut failure = None;
            candidates.as_mut_slice().sort_unstable_by(|a, b| {
                if failure.is_some() {
                    return Ordering::Equal;
                }
                let order = a.graph.cmp(&b.graph);
                if order != Ordering::Equal {
                    return order;
                }
                match (a.virtual_ordinal, b.virtual_ordinal) {
                    (Some(a), Some(b)) => a.cmp(&b),
                    (None, Some(_)) => Ordering::Less,
                    (Some(_), None) => Ordering::Greater,
                    (None, None) => {
                        let mask = masks[index];
                        let plan =
                            ctx.dataset
                                .probe_plan(mask[0], mask[1], mask[2], graph_of(a.graph));
                        match ctx.dataset.compare_probe_order(&plan, a.quad, b.quad) {
                            Some(order) => order,
                            None => {
                                failure = Some(EvalError::WorkspaceBoundOverflow);
                                Ordering::Equal
                            }
                        }
                    }
                }
            });
            if let Some(error) = failure {
                return Err(error);
            }
            // The original FROM union keeps the first physical occurrence of a
            // triple. Its first graph does not depend on the row's bound values.
            if matches!(scope, GraphScope::Merge(_)) {
                let mut seen = AdmittedMap::default();
                let mut unique = AdmittedVec::new(&ctx.growth);
                for candidate in candidates {
                    let key = (candidate.quad.s, candidate.quad.p, candidate.quad.o);
                    if seen.get(&key).is_none() {
                        seen.insert_admitted(key, (), &ctx.growth)?;
                        unique.push(candidate)?;
                    }
                }
                candidates = unique;
            }
            let is_left = matches!(cp.o, Pos::Slot(column) if column == left);
            let is_right = matches!(cp.o, Pos::Slot(column) if column == right);
            let is_position = is_left || is_right;
            let new_left = is_left
                || (!is_right && matches!(cp.s, Pos::Slot(column) if Some(column) == left_subject));
            let projected =
                new_left || matches!(cp.s, Pos::Slot(column) if Some(column) == right_subject);
            let other = (is_position || projected).then_some(if new_left { right } else { left });
            if is_position {
                for candidate in candidates.as_mut_slice() {
                    let value =
                        crate::expr::owned_value_of(ctx, SolutionTerm::Existing(candidate.quad.o))?;
                    candidate.numeric = parsed(&value, &ctx.growth)?;
                }
            }
            result.patterns.push(Pattern {
                candidates,
                buckets: AdmittedMap::default(),
                bound: masks[index],
                other,
                other_subject: if new_left {
                    right_subject
                } else {
                    left_subject
                },
                new_left,
            })?;
        }
        // Certify only a single original positional cell per subject. Multiple
        // predicates/values and unknown numeric cells retain the original path.
        for (index, cp) in compiled.iter().enumerate().filter(|_| spec.restrict) {
            let Pos::Slot(column) = cp.o else { continue };
            if column != left && column != right {
                continue;
            }
            let subject = if column == left {
                left_subject
            } else {
                right_subject
            };
            if !matches!(cp.s, Pos::Slot(candidate) if Some(candidate) == subject) {
                continue;
            }
            for candidate in &result.patterns[index].candidates {
                let key = (column, candidate.quad.s);
                if result.inferred.get(&key).is_some() {
                    *result.inferred.get_mut(&key).expect("existing inference") = None;
                } else {
                    let inferred = candidate.numeric.as_ref().map(|value| Inferred {
                        term: SolutionTerm::Existing(candidate.quad.o),
                        value: value.clone(),
                    });
                    result
                        .inferred
                        .insert_admitted(key, inferred, &ctx.growth)?;
                }
            }
        }
        for (index, cp) in compiled.iter().enumerate() {
            let pattern = &mut result.patterns.as_mut_slice()[index];
            let candidates = &mut pattern.candidates;
            if !matches!(cp.o, Pos::Slot(column) if column == left || column == right)
                && pattern.other.is_some()
            {
                let column = if pattern.new_left { left } else { right };
                for candidate in candidates.as_mut_slice() {
                    candidate.numeric = result
                        .inferred
                        .get(&(column, candidate.quad.s))
                        .and_then(Option::as_ref)
                        .map(|inferred| inferred.value.clone());
                }
            }
            let mask = pattern.bound;
            let mut buckets = AdmittedMap::<Key<I>, Bucket>::default();
            for (ordinal, candidate) in candidates.iter().enumerate() {
                let key = key(
                    mask,
                    Some(candidate.quad.s),
                    Some(candidate.quad.p),
                    Some(candidate.quad.o),
                );
                if buckets.get(&key).is_none() {
                    buckets.insert_admitted(
                        key,
                        Bucket {
                            ordered: AdmittedVec::new(&ctx.growth),
                            numeric: AdmittedVec::new(&ctx.growth),
                            unknown: AdmittedVec::new(&ctx.growth),
                        },
                        &ctx.growth,
                    )?;
                }
                let bucket = buckets
                    .get_mut(&key)
                    .expect("the bucket was inserted above");
                bucket.ordered.push(ordinal)?;
                if candidate.numeric.is_some() {
                    bucket.numeric.push(ordinal)?;
                } else {
                    bucket.unknown.push(ordinal)?;
                }
            }
            for (_, bucket) in buckets.iter_mut() {
                let mut failure = None;
                bucket.numeric.as_mut_slice().sort_unstable_by(|&a, &b| {
                    if failure.is_some() {
                        return Ordering::Equal;
                    }
                    match compare(
                        candidates[a].numeric.as_ref().expect("numeric index"),
                        candidates[b].numeric.as_ref().expect("numeric index"),
                        &ctx.growth,
                    ) {
                        Ok(order) => order.then_with(|| a.cmp(&b)),
                        Err(error) => {
                            failure = Some(error);
                            Ordering::Equal
                        }
                    }
                });
                if let Some(error) = failure {
                    return Err(error);
                }
            }
            pattern.buckets = buckets;
        }
        Ok(result)
    }

    /// Prepare one ordered sweep over every distinct driving ordinal. Equal
    /// driver cells share a range, but their original rows are never coalesced.
    pub(super) fn prepare_stage<D: DatasetView<Id = I> + Sync>(
        &self,
        index: usize,
        cp: &CompiledPattern<I>,
        rows: &[RetainedRow<I>],
        ctx: &EvalCtx<'_, D>,
    ) -> Result<StageWindow<I>, EvalError> {
        let pattern = &self.patterns[index];
        let mut result = StageWindow {
            ranges: AdmittedMap::default(),
        };
        if !self.spec.restrict {
            return Ok(result);
        }
        let Some(other) = pattern.other else {
            return Ok(result);
        };
        let mut drivers = AdmittedVec::new(&ctx.growth);
        let mut seen = AdmittedMap::default();
        for (ordinal, row) in rows.iter().enumerate() {
            let inferred = pattern
                .other_subject
                .and_then(|subject| match row[subject] {
                    Some(SolutionTerm::Existing(id)) => {
                        self.inferred.get(&(other, id)).and_then(Option::as_ref)
                    }
                    _ => None,
                });
            let Some(term) = row[other].or_else(|| inferred.map(|value| value.term)) else {
                continue;
            };
            let key = key(
                pattern.bound,
                query_id(&cp.s, row),
                query_id(&cp.p, row),
                query_id(&cp.o, row),
            );
            if pattern.buckets.get(&key).is_none() || seen.get(&(key, term)).is_some() {
                continue;
            }
            seen.insert_admitted((key, term), (), &ctx.growth)?;
            let value = if row[other].is_none() {
                inferred.map(|value| value.value.clone())
            } else {
                let original = crate::expr::owned_value_of(ctx, term)?;
                parsed(&original, &ctx.growth)?
            };
            if let Some(value) = value {
                drivers.push(Driver {
                    key,
                    term,
                    value,
                    ordinal,
                })?;
            }
        }
        let mut failure = None;
        drivers.as_mut_slice().sort_unstable_by(|a, b| {
            let key_order = a.key.cmp(&b.key);
            if key_order != Ordering::Equal || failure.is_some() {
                return key_order;
            }
            match self.compare(&a.value, &b.value, &ctx.growth) {
                Ok(order) => order.then_with(|| a.ordinal.cmp(&b.ordinal)),
                Err(error) => {
                    failure = Some(error);
                    Ordering::Equal
                }
            }
        });
        if let Some(error) = failure {
            return Err(error);
        }
        let mut partition = None;
        let mut lower = 0;
        let mut upper = 0;
        for driver in drivers {
            if partition != Some(driver.key) {
                partition = Some(driver.key);
                lower = 0;
                upper = 0;
            }
            let bucket = pattern
                .buckets
                .get(&driver.key)
                .expect("driver partition exists");
            let range = self
                .spec
                .range(&driver.value, pattern.new_left, &ctx.growth)?;
            if let Some(bound) = &range.lower {
                while lower < bucket.numeric.len() {
                    let candidate = pattern.candidates[bucket.numeric[lower]]
                        .numeric
                        .as_ref()
                        .expect("numeric ordinal");
                    let order = self.compare(candidate, bound, &ctx.growth)?;
                    if order == Ordering::Less || (range.strict && order == Ordering::Equal) {
                        lower += 1;
                    } else {
                        break;
                    }
                }
            }
            if let Some(bound) = &range.upper {
                while upper < bucket.numeric.len() {
                    let candidate = pattern.candidates[bucket.numeric[upper]]
                        .numeric
                        .as_ref()
                        .expect("numeric ordinal");
                    let order = self.compare(candidate, bound, &ctx.growth)?;
                    if order == Ordering::Less || (!range.strict && order == Ordering::Equal) {
                        upper += 1;
                    } else {
                        break;
                    }
                }
            } else {
                upper = bucket.numeric.len();
            }
            result.ranges.insert_admitted(
                (driver.key, driver.term),
                (lower, upper),
                &ctx.growth,
            )?;
        }
        Ok(result)
    }

    fn compare(
        &self,
        a: &purrdf_xsd::XsdValue,
        b: &purrdf_xsd::XsdValue,
        workspace: &WorkspaceCapability,
    ) -> Result<Ordering, EvalError> {
        let order = compare(a, b, workspace)?;
        self.comparisons
            .try_update(AtomicOrdering::Relaxed, AtomicOrdering::Relaxed, |n| {
                n.checked_add(1)
            })
            .map_err(|_| EvalError::WorkspaceBoundOverflow)?;
        Ok(order)
    }

    /// Original per-row probe order, restricted by the stage's completed sweep.
    pub(super) fn candidates<D: DatasetView<Id = I> + Sync>(
        &self,
        stage: &StageWindow<I>,
        index: usize,
        cp: &CompiledPattern<I>,
        row: &RetainedRow<I>,
        ctx: &EvalCtx<'_, D>,
    ) -> Result<AdmittedVec<QuadIds<I>>, EvalError> {
        let pattern = &self.patterns[index];
        let key = key(
            pattern.bound,
            query_id(&cp.s, row),
            query_id(&cp.p, row),
            query_id(&cp.o, row),
        );
        let mut output = AdmittedVec::new(&ctx.growth);
        let Some(bucket) = pattern.buckets.get(&key) else {
            return Ok(output);
        };
        let other = pattern.other.and_then(|column| {
            row[column].or_else(|| {
                pattern
                    .other_subject
                    .and_then(|subject| match row[subject] {
                        Some(SolutionTerm::Existing(id)) => self
                            .inferred
                            .get(&(column, id))
                            .and_then(Option::as_ref)
                            .map(|value| value.term),
                        _ => None,
                    })
            })
        });
        let range = other.and_then(|term| stage.ranges.get(&(key, term)));
        let Some(&(start, end)) = range else {
            for &ordinal in &bucket.ordered {
                output.push(pattern.candidates[ordinal].quad)?;
            }
            return Ok(output);
        };
        let mut chosen = AdmittedVec::new(&ctx.growth);
        if start <= end {
            for &ordinal in &bucket.numeric[start..end] {
                chosen.push(ordinal)?;
            }
        }
        // Float promotion, ill-typed and non-numeric cells are decided by the
        // original FILTER, rather than guessed by an exact-number index.
        for &ordinal in &bucket.unknown {
            chosen.push(ordinal)?;
        }
        chosen.as_mut_slice().sort_unstable();
        for ordinal in chosen {
            output.push(pattern.candidates[ordinal].quad)?;
        }
        Ok(output)
    }

    pub(super) fn comparisons(&self) -> u64 {
        self.comparisons.load(AtomicOrdering::Relaxed)
    }
}

fn key<I: Copy>(mask: [bool; 3], s: Option<I>, p: Option<I>, o: Option<I>) -> Key<I> {
    (
        mask[0].then_some(s).flatten(),
        mask[1].then_some(p).flatten(),
        mask[2].then_some(o).flatten(),
    )
}

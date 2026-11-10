// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Shared path pair production; each reach unit finishes before its evidence escapes.

use super::{
    DatasetView, Endpoint, EvalCtx, EvalError, PathBag, PathCtx, PathMap, PathProgram, PathSet,
    PathStorage, PropertyPathExpression, ROOT_OP, RefCell, SolutionSeq, SolutionTerm, TermPattern,
    ViewTermId, bag_reach, node_universe, path_schema, reach, resolve_end, visible_var,
};

enum Pair<I: ViewTermId> {
    Existing(I, I, Option<crate::RowEvidence>),
    AbsentIdentity(Option<crate::WorkspaceTerm>),
}

enum Mode<I: ViewTermId> {
    Empty,
    Identity(Option<crate::WorkspaceTerm>),
    Bound(I, I),
    Forward(I),
    Backward(I),
    Universe { same: bool },
}

trait PairSource<I: ViewTermId>: Send {
    fn next(&mut self) -> Result<Option<Pair<I>>, EvalError>;
}

struct Pairs<'d, D: DatasetView + Sync> {
    ctx: PathCtx<'d, D>,
    mode: Mode<D::Id>,
    bag: bool,
    started: bool,
    universe: Option<PathSet<D::Id>>,
    universe_index: usize,
    start: Option<D::Id>,
    reached: Option<PathBag<D::Id>>,
    position: usize,
}

impl<D: DatasetView + Sync> Pairs<'_, D> {
    fn expire_completed_root(&self, node: D::Id, forward: bool) {
        if !self.ctx.stopped() {
            let (op, forward) = self.ctx.program.normalize_direction(ROOT_OP, forward);
            // Pre-order descendants never call their ancestor. Every external
            // start is unique (or fixed once), so this completed root is dead.
            // Its copied/pinned evidence survives; all descendant memos remain.
            self.ctx
                .reach_cache
                .borrow_mut()
                .entries
                .remove(&(op, node, forward));
        }
    }

    fn reach(&self, node: D::Id, forward: bool) -> Result<PathBag<D::Id>, EvalError> {
        if self.bag {
            return bag_reach(ROOT_OP, node, forward, &self.ctx);
        }
        let reached = reach(ROOT_OP, node, forward, &self.ctx)?;
        let mut nodes = PathBag::new(&self.ctx.growth);
        for &node in reached.iter() {
            nodes.push(node, reached.evidence.get(&node).cloned(), &self.ctx.growth)?;
        }
        self.expire_completed_root(node, forward);
        Ok(nodes)
    }

    fn next_inner(&mut self) -> Result<Option<Pair<D::Id>>, EvalError> {
        self.ctx.storage.check()?;
        loop {
            if let Some(reached) = &self.reached {
                while let Some(&target) = reached.get(self.position) {
                    let position = self.position;
                    self.position += 1;
                    let start = self
                        .start
                        .ok_or_else(|| EvalError::internal("path reach lost its start"))?;
                    let (subject, object) = match self.mode {
                        Mode::Bound(subject, object) if target == object => (subject, object),
                        Mode::Forward(subject) => (subject, target),
                        Mode::Backward(object) => (target, object),
                        Mode::Universe { same } if !same || target == start => (start, target),
                        _ => continue,
                    };
                    self.ctx.storage.check()?;
                    return Ok(Some(Pair::Existing(
                        subject,
                        object,
                        reached.evidence.get(&position).cloned(),
                    )));
                }
                self.reached = None;
            }
            let (node, forward) = match &mut self.mode {
                Mode::Empty => return Ok(None),
                Mode::Identity(term) => {
                    if self.started {
                        return Ok(None);
                    }
                    self.started = true;
                    return Ok(Some(Pair::AbsentIdentity(term.take())));
                }
                Mode::Bound(subject, object) => {
                    if self.started {
                        return Ok(None);
                    }
                    self.started = true;
                    if !self.bag {
                        let root = *subject;
                        // A set membership probe keeps its original coalesced witness.
                        let reached = reach(ROOT_OP, *subject, true, &self.ctx)?;
                        self.ctx.storage.check()?;
                        let pair = reached.contains(object).then(|| {
                            Pair::Existing(*subject, *object, reached.evidence.get(object).cloned())
                        });
                        self.expire_completed_root(root, true);
                        return Ok(pair);
                    }
                    (*subject, true)
                }
                Mode::Forward(subject) => {
                    if self.started {
                        return Ok(None);
                    }
                    self.started = true;
                    (*subject, true)
                }
                Mode::Backward(object) => {
                    if self.started {
                        return Ok(None);
                    }
                    self.started = true;
                    (*object, false)
                }
                Mode::Universe { same } => {
                    if self.universe.is_none() {
                        self.universe = Some(node_universe(&self.ctx)?);
                    }
                    let Some(&node) = self
                        .universe
                        .as_ref()
                        .and_then(|nodes| nodes.get(self.universe_index))
                    else {
                        return Ok(None);
                    };
                    self.universe_index += 1;
                    if *same && self.ctx.program.is_reflexive() && self.ctx.evidence_input.is_none()
                    {
                        return Ok(Some(Pair::Existing(node, node, None)));
                    }
                    (node, true)
                }
            };
            self.start = Some(node);
            self.position = 0;
            self.reached = Some(self.reach(node, forward)?);
            self.ctx.storage.check()?;
        }
    }
}

impl<D: DatasetView + Sync> PairSource<D::Id> for Pairs<'_, D> {
    fn next(&mut self) -> Result<Option<Pair<D::Id>>, EvalError> {
        let dataset = self.ctx.dataset;
        dataset
            .checked_read(|_| self.next_inner())
            .map_err(|error| self.ctx.workspace.source_error(error))?
    }
}

pub(crate) struct PathRows<'d, I: ViewTermId> {
    pub(crate) schema: crate::solution::SharedSchema,
    source: Option<crate::WorkspaceBox<dyn PairSource<I> + 'd>>,
    subject_column: Option<usize>,
    object_column: Option<usize>,
    produced: usize,
    semantic_ceiling: Option<usize>,
    cell_ceiling: Option<usize>,
}

impl<'d, I: ViewTermId> PathRows<'d, I> {
    pub(crate) fn new<D: DatasetView<Id = I> + Sync>(
        subject: &TermPattern,
        path: &PropertyPathExpression,
        object: &TermPattern,
        ctx: &EvalCtx<'d, D>,
    ) -> Result<Self, EvalError> {
        let schema = path_schema(subject, object, &ctx.growth)?;
        let subject_column = visible_var(subject).and_then(|variable| schema.index_of(&variable));
        let object_column = visible_var(object).and_then(|variable| schema.index_of(&variable));
        let subject = resolve_end(subject, ctx.dataset, &ctx.growth, |error| {
            ctx.workspace.source_error(error)
        })?;
        let object = resolve_end(object, ctx.dataset, &ctx.growth, |error| {
            ctx.workspace.source_error(error)
        })?;
        let storage = PathStorage::new(&ctx.growth)?;
        let pctx = PathCtx {
            dataset: ctx.dataset,
            evidence_input: ctx.evidence_input.clone(),
            scope: ctx.active_dataset.scope_for(ctx.active_graph),
            program: PathProgram::compile_admitted(path, ctx.dataset, &ctx.growth, |error| {
                ctx.workspace.source_error(error)
            })?,
            workspace: ctx.workspace.clone(),
            growth: ctx.growth.clone(),
            reach_cache: RefCell::new(PathMap::new(&storage)),
            storage,
            governors: ctx.governor_state().cloned(),
            ledger: ctx
                .charge_ledger()
                .map(|ledger| (ledger.clone(), ctx.ledger_node)),
        };
        let bag = !pctx.program.has_repetition();
        let mode = match (subject, object) {
            (Endpoint::Bound(subject), Endpoint::Bound(object)) => Mode::Bound(subject, object),
            (Endpoint::BoundAbsent(subject), Endpoint::BoundAbsent(object)) => {
                if ctx.growth.terms_equal(&subject, &object)? && pctx.program.is_reflexive() {
                    Mode::Identity(None)
                } else {
                    Mode::Empty
                }
            }
            (Endpoint::Bound(_), Endpoint::BoundAbsent(_))
            | (Endpoint::BoundAbsent(_), Endpoint::Bound(_)) => Mode::Empty,
            (Endpoint::Bound(subject), Endpoint::Free { .. }) => Mode::Forward(subject),
            (Endpoint::Free { .. }, Endpoint::Bound(object)) => Mode::Backward(object),
            (Endpoint::BoundAbsent(term), Endpoint::Free { .. })
            | (Endpoint::Free { .. }, Endpoint::BoundAbsent(term)) => {
                if pctx.program.is_reflexive() {
                    Mode::Identity(Some(term))
                } else {
                    Mode::Empty
                }
            }
            (Endpoint::Free { var: subject }, Endpoint::Free { var: object }) => Mode::Universe {
                same: subject == object,
            },
        };
        ctx.note_cursor_buffer(crate::eval::CursorBufferReason::PathReachability);
        let source = crate::WorkspaceBox::new_admitted(
            Pairs {
                ctx: pctx,
                mode,
                bag,
                started: false,
                universe: None,
                universe_index: 0,
                start: None,
                reached: None,
                position: 0,
            },
            &ctx.growth,
        )?
        .coerce(|source| -> Box<dyn PairSource<I> + 'd> { source });
        let cell_ceiling = ctx.cell_row_ceiling(schema.len());
        Ok(Self {
            schema: schema.shared_admitted(&ctx.growth)?,
            source: Some(source),
            subject_column,
            object_column,
            produced: 0,
            semantic_ceiling: ctx.row_ceiling(),
            cell_ceiling,
        })
    }

    pub(crate) fn stop(&mut self) {
        self.source = None;
    }

    pub(crate) const fn produced(&self) -> usize {
        self.produced
    }

    pub(crate) fn next<D: DatasetView<Id = I> + Sync>(
        &mut self,
        ctx: &mut EvalCtx<'_, D>,
    ) -> Result<Option<crate::solution::RetainedRow<I>>, EvalError> {
        let Some(pair) = self
            .source
            .as_mut()
            .map(|source| source.next())
            .transpose()?
            .flatten()
        else {
            self.stop();
            return Ok(None);
        };
        let (subject, object, evidence) = match pair {
            Pair::Existing(subject, object, evidence) => (
                Some(SolutionTerm::Existing(subject)),
                Some(SolutionTerm::Existing(object)),
                evidence,
            ),
            Pair::AbsentIdentity(term) => {
                // An absent ground endpoint's zero-length identity is mandatory.
                let term = term
                    .map(|term| ctx.intern_workspace_term(term))
                    .transpose()?
                    .flatten();
                (term, term, None)
            }
        };
        // Preserve the original reach-unit and mandatory absent-term admission
        // before testing the output ceiling. A finite trip can depend on them.
        if self
            .semantic_ceiling
            .is_some_and(|cap| self.produced >= cap)
        {
            self.stop();
            return Ok(None);
        }
        if self.cell_ceiling.is_some_and(|cap| self.produced >= cap) {
            let _ = ctx.observe_cells(self.produced.saturating_add(1), self.schema.len());
            self.stop();
            return Ok(None);
        }
        let row = crate::solution::RetainedRow::try_build(self.schema.len(), &ctx.growth, |row| {
            if let Some(column) = self.subject_column {
                row[column] = subject;
            }
            if let Some(column) = self.object_column {
                row[column] = object;
            }
            Ok(())
        })?;
        let evidence = match &ctx.evidence_input {
            Some(input) => Some(input.found_witness(&evidence.unwrap_or_default(), &ctx.growth)?),
            None => evidence,
        };
        self.produced = self
            .produced
            .checked_add(1)
            .ok_or(EvalError::WorkspaceBoundOverflow)?;
        Ok(Some(row.with_optional_evidence(evidence, &ctx.growth)?))
    }
}

/// Evaluate the same ordered path pairs to the completed answer representation.
#[inline(never)]
pub(crate) fn eval_path<D: DatasetView + Sync>(
    subject: &TermPattern,
    path: &PropertyPathExpression,
    object: &TermPattern,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<SolutionSeq<D::Id>, EvalError> {
    let mut source = PathRows::new(subject, path, object, ctx)?;
    let mut rows = crate::solution::RowsBuilder::new(&ctx.growth);
    while let Some(row) = source.next(ctx)? {
        rows.push_row(row)?;
    }
    Ok(SolutionSeq {
        schema: source.schema,
        rows: rows.finish()?,
    })
}

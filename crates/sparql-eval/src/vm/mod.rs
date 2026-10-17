// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The expression VM: every `FILTER`, `BIND`, `OPTIONAL` condition, `ORDER BY` key,
//! aggregate argument and `UNFOLD` expression is evaluated here.
//!
//! An expression is evaluated in three steps, each paid as rarely as it can be:
//!
//! 1. **Compile**, once per expression site per plan ([`program_at`]): the expression
//!    becomes an [`ExprProgram`] — a flat instruction array, operands emitted left to
//!    right, with the tables the instructions index. A site of the installed plan keeps
//!    its program on the plan shape against its `ExprId`, so every operator call over
//!    that site reuses it; a site outside the plan (a per-row substituted copy, a
//!    predicate built for a call read) compiles once per operator call.
//! 2. **Link**, once per operator call ([`Linked::link`]): each variable the program
//!    reads is resolved to the operator's schema column, a `REGEX`/`REPLACE` whose pattern
//!    and flags are constants has its pattern compiled, and each `EXISTS` is bound to
//!    the pattern it tests. The link also owns the constant pool, which interns each
//!    constant the first time a row reads it.
//! 3. **Run**, once per row ([`Linked::term`], [`Linked::ebv`]): an interpreter over an
//!    explicit value stack. Nothing is recursive, so an expression of any depth
//!    evaluates in heap memory.
//!
//! # What the run preserves
//!
//! Every operand the tree walk evaluated is evaluated, in the same order, and every
//! operand it skipped is jumped over: `&&`/`||` evaluate every operand and fold (the
//! first hard error ends the run), `IF` runs one branch, `COALESCE` stops at its first
//! bound item, `IN` stops at its first match and runs no candidate after an unbound
//! needle. The helpers that compute each operator's value are the tree walk's own,
//! taking already-evaluated operands, so every charge, every draw of the query's random
//! state and every `BNODE` memo lookup happens exactly as it did, and every value is
//! interned where the tree walk interned it — but for a triple term constructor nested in
//! another's object, whose term is built into the enclosing one rather than interned
//! level by level. See [`compile`](self::compile) for how the compiler keeps the set of
//! interned values.
//!
//! # Suspension points
//!
//! `EXISTS` and a SPARQL-bodied function call re-enter pattern evaluation. Their
//! instructions produce a [`Suspend`] (a custom call's probe returns it as a
//! [`VmStep::Suspend`]), which the interpreter resolves at once by calling the
//! evaluator; resolving a suspension is the one place that re-entry happens.

mod compile;
mod evidence;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use purrdf_core::{DatasetView, TermValue};
use purrdf_sparql_algebra::{Expression, Function, GraphPattern};

pub(crate) use compile::ExprProgram;
use compile::{Cmp, Op, StrPred};

use crate::error::{EvalError, EvaluatorInvariant, StackExpectation};
use crate::eval::EvalCtx;
use crate::expr as helpers;
use crate::scratch::SolutionTerm;
use crate::solution::VarSchema;

/// The program for `expr`, attached to `node`: the compiled program on the plan shape when `node`
/// is a node of the installed plan and `expr` one of its attached expressions, compiled
/// on first use; otherwise a program compiled for this call.
pub(crate) fn program_at<D: DatasetView + Sync>(
    ctx: &EvalCtx<'_, D>,
    node: &GraphPattern,
    expr: &Expression,
) -> Result<crate::workspace::SharedWorkspace<ExprProgram>, EvalError> {
    if let Some(plan) = ctx.plan.as_ref()
        && let Some(id) = plan.node_of(node)
        && let Some(position) = attached_position(node, expr)
    {
        if !ctx.growth.is_bounded() {
            return Ok(plan.shape().program(id, position, expr).into());
        }
        if let Some(program) = plan.shape().cached_program(id, position) {
            return Ok(program.into());
        }
    }
    if ctx.growth.is_bounded() {
        let program = ExprProgram::compile_admitted(expr, &ctx.growth)?;
        crate::workspace::SharedWorkspace::new_admitted(program, &ctx.growth)
    } else {
        Ok(Arc::new(ExprProgram::compile(expr)).into())
    }
}

/// `expr`'s position among `node`'s attached expressions, in the order the plan
/// numbers them.
fn attached_position(node: &GraphPattern, expr: &Expression) -> Option<usize> {
    let mut position = 0_usize;
    let mut found = None;
    crate::governor::soundness::visit_pattern_parts(node, &mut |part| {
        if let crate::governor::soundness::PatternPart::Expression(attached) = part {
            if std::ptr::eq(attached, expr) {
                found = Some(position);
                return true;
            }
            position += 1;
        }
        false
    });
    found
}

/// One value-stack entry.
#[derive(Debug)]
enum Val<I: Copy> {
    /// A term, or unbound.
    Term(Option<SolutionTerm<I>>),
    /// An effective boolean value, or an error.
    Ebv(Option<bool>),
    /// A string argument (lexical form, lower-cased tag and base direction), or absent.
    Str(Option<helpers::OwnedStringArg>),
    /// The program's string constant at this index, read in place rather than copied.
    StrConst(u32),
    /// A triple term a constructor built for the constructor it is the object of, not
    /// interned, or unbound.
    Value(Option<crate::WorkspaceTerm>),
    /// An `IN` in progress: the needle, its value, and whether a candidate raised.
    In {
        target: SolutionTerm<I>,
        value: crate::WorkspaceTerm,
        saw_error: bool,
    },
}

/// Actual dispatch still reads its original native `[Option<TermValue>]` slice.
/// This owner keeps every payload's original grant through that entire call,
/// including a SPARQL-bodied suspension, without copying any argument values.
pub(crate) struct Arguments {
    values: crate::AdmittedVec<Option<TermValue>>,
    // Declaration order drops all payloads before their original grants.
    allocations: crate::AdmittedVec<crate::WorkspaceAllocation>,
}

impl std::fmt::Debug for Arguments {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Arguments")
            .field("len", &self.values.len())
            .finish_non_exhaustive()
    }
}

impl std::ops::Deref for Arguments {
    type Target = [Option<TermValue>];
    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

impl Arguments {
    fn new(workspace: &crate::WorkspaceCapability) -> Self {
        Self {
            values: crate::AdmittedVec::new(workspace),
            allocations: crate::AdmittedVec::new(workspace),
        }
    }

    fn with_capacity(
        capacity: usize,
        workspace: &crate::WorkspaceCapability,
    ) -> Result<Self, EvalError> {
        Ok(Self {
            values: crate::AdmittedVec::with_capacity(capacity, workspace)?,
            allocations: if workspace.is_bounded() {
                crate::AdmittedVec::with_capacity(capacity, workspace)?
            } else {
                crate::AdmittedVec::new(workspace)
            },
        })
    }

    fn clear(&mut self) {
        self.values.clear();
        self.allocations.clear();
    }

    fn push(&mut self, value: Option<crate::WorkspaceTerm>) -> Result<(), EvalError> {
        // Admit both metadata slots BEFORE separating the original payload owner.
        // A refused push still drops WorkspaceTerm's payload before its grant.
        let retain = value
            .as_ref()
            .is_some_and(crate::WorkspaceTerm::has_admission);
        self.values.reserve_next()?;
        if retain {
            self.allocations.reserve_next()?;
        }
        if let Some(value) = value {
            let (value, allocation) = value.into_parts();
            self.values.push_reserved(Some(value));
            if retain {
                self.allocations.push_reserved(allocation);
            }
        } else {
            self.values.push_reserved(None);
        }
        Ok(())
    }

    fn take(&mut self, workspace: &crate::WorkspaceCapability) -> Self {
        std::mem::replace(self, Self::new(workspace))
    }
}

/// The value stack. On the heap rather than inline: a link lives in the frame of the
/// operator it evaluates for, and an `EXISTS` it runs, or an `OPTIONAL` whose join it
/// decides, recurses through the plan with that frame live, so inline slots here would
/// be paid once per nesting level of the plan.
type ValStack<I> = Vec<Val<I>>;

/// One constant of a link's pool.
#[derive(Clone, Copy)]
enum ConstCell<I: Copy> {
    /// No row has read the constant yet.
    Unread,
    /// The constant's interned term (`None`: its language tag is one the grammar
    /// refuses, which makes it unbound).
    Read(Option<SolutionTerm<I>>),
}

/// One regex slot of a link.
#[derive(Clone)]
enum RegexSlot {
    /// The pattern or flags are computed per row, and so is the regex.
    PerRow,
    /// The pattern and flags are constants. Their link-time verdict is applied
    /// when the call executes, preserving lazy branches and operational errors.
    Linked(crate::xpath_regex::LinkedPattern),
}

impl RegexSlot {
    /// The link-time pattern, when there is one.
    const fn linked(&self) -> Option<&crate::xpath_regex::LinkedPattern> {
        match self {
            Self::PerRow => None,
            Self::Linked(compiled) => Some(compiled),
        }
    }
}

/// What an instruction that may re-enter the evaluator produced.
pub(crate) enum VmStep<'d, I> {
    /// The instruction's value.
    Value(Option<SolutionTerm<I>>),
    /// The instruction needs pattern evaluation before it has a value.
    Suspend(Suspend<'d>),
}

/// A pattern evaluation an instruction is waiting on.
pub(crate) enum Suspend<'d> {
    /// `EXISTS` at the compiler's pre-order site, for the current row.
    Exists(u32),
    /// A SPARQL-bodied function call: the declaration, the prepared body, the call's
    /// entry in the program's call table, and the evaluated arguments.
    SparqlUdf {
        func: &'d crate::user_fn::UserFunction,
        body: &'d Arc<crate::engine::PreparedQuery>,
        call: u32,
        vals: Arguments,
    },
}

/// A program linked to one operator call: its variable slots resolved to the call's
/// schema, its constant regexes compiled, its `EXISTS` bound to their patterns, and its
/// constant pool and value stack.
pub(crate) struct Linked<'e, I: Copy> {
    program: crate::workspace::SharedWorkspace<ExprProgram>,
    /// Each variable slot's schema column.
    ///
    /// Held inline for a program reading few variables, so linking one allocates
    /// nothing for its slots; likewise the constant pool below. Four slots each: these
    /// are in the frame of the operator the link evaluates for, which may be live while
    /// the plan recurses beneath it.
    slots: purrdf_core::SmallVec<[Option<usize>; 4]>,
    /// Each constant's term, once a row has read it.
    consts: purrdf_core::SmallVec<[ConstCell<I>; 4]>,
    /// Each regex slot's pattern.
    regexes: Vec<RegexSlot>,
    /// Each `EXISTS`'s owner-relative operand path, in compiled pre-order.
    /// The expression owner stays with the caller, so this VM can be moved
    /// beside a copied tree without borrowing its own frame.
    exists: Option<crate::workspace::SharedWorkspace<ExistsSites<'e>>>,
    stack: ValStack<I>,
    /// Born only for an explicit evidence query; ordinary stacks are unchanged.
    evidence: Option<crate::workspace::SharedWorkspace<std::sync::Mutex<evidence::Records>>>,
    /// A function call's argument values, refilled by every call instruction, so a row
    /// that calls a function reuses one buffer instead of allocating its own.
    args: Arguments,
    // The linked buffers are destroyed before their physical admission.
    _allocation: Option<crate::WorkspaceAllocation>,
}

struct ExistsSite {
    steps: Vec<usize>,
    _allocation: Option<crate::WorkspaceAllocation>,
}

struct ExistsSites<'e> {
    owner: ExpressionOwner<'e>,
    sites: Vec<ExistsSite>,
    _allocation: Option<crate::WorkspaceAllocation>,
}

impl ExistsSites<'_> {
    /// Resolve the compiled ordinal against its retained immutable expression
    /// owner. Both the immediate VM and a suspended row use this one path.
    fn pattern<'a>(
        &'a self,
        site: u32,
        expression: Option<&Expression>,
    ) -> Result<&'a GraphPattern, EvalError> {
        let site = self
            .sites
            .get(site as usize)
            .ok_or_else(|| EvalError::Invariant(EvaluatorInvariant::ExistsPatternMissing))?;
        let mut node = self.owner.expression(expression)?;
        for &index in &site.steps {
            let mut child = None;
            let mut at = 0usize;
            compile::visit_operands(node, |operand| {
                if at == index {
                    child = Some(operand);
                }
                at += 1;
            });
            node = child
                .ok_or_else(|| EvalError::Invariant(EvaluatorInvariant::ExistsExpressionMissing))?;
        }
        let Expression::Exists(pattern) = node else {
            return Err(EvalError::Invariant(
                EvaluatorInvariant::ExistsExpressionMissing,
            ));
        };
        Ok(pattern)
    }
}

enum ExpressionOwner<'e> {
    Borrowed(&'e Expression),
    Copied {
        pattern: crate::expr::PreparedPattern,
        steps: Vec<usize>,
        _allocation: Option<crate::WorkspaceAllocation>,
    },
}

impl ExpressionOwner<'_> {
    fn expression<'a>(
        &'a self,
        supplied: Option<&Expression>,
    ) -> Result<&'a Expression, EvalError> {
        use purrdf_sparql_algebra::NodeRef;
        let expression = match self {
            Self::Borrowed(expression) => *expression,
            Self::Copied { pattern, steps, .. } => {
                let node = NodeRef::Pattern(pattern).at_path(steps).ok_or_else(|| {
                    EvalError::Invariant(EvaluatorInvariant::CopiedExpressionChildMissing)
                })?;
                let NodeRef::Expr(expression) = node else {
                    return Err(EvalError::Invariant(
                        EvaluatorInvariant::CopiedExpressionMissing,
                    ));
                };
                expression
            }
        };
        if let Some(supplied) = supplied
            && !std::ptr::eq(supplied, expression)
        {
            return Err(EvalError::Invariant(
                EvaluatorInvariant::ExistsExpressionOwnerChanged,
            ));
        }
        if supplied.is_none() && matches!(self, Self::Borrowed(_)) {
            return Err(EvalError::Invariant(
                EvaluatorInvariant::ExistsExpressionOwnerMissing,
            ));
        }
        Ok(expression)
    }
}

/// Locate an attached expression in the one immutable copied tree. The
/// algebra walk supplies the exhaustive child order; only the path survives.
type CopiedExpressionPath = (Vec<usize>, Option<crate::WorkspaceAllocation>);

fn copied_expression_path(
    pattern: &crate::expr::PreparedPattern,
    target: &Expression,
    workspace: &crate::WorkspaceCapability,
) -> Result<Option<CopiedExpressionPath>, EvalError> {
    use purrdf_sparql_algebra::walk::{Flow, NodeRef, Visit, WalkError, try_walk_pre_post};
    let mut ancestors_allocation = None;
    let mut ancestors: Vec<usize> = Vec::new();
    let mut steps_allocation = None;
    let mut steps = Vec::new();
    let mut found = false;
    let walked = try_walk_pre_post(
        NodeRef::Pattern(pattern),
        |phase, node| match phase {
            Visit::Enter => {
                if let Some(next_child) = ancestors.last_mut() {
                    let at = *next_child;
                    *next_child = (*next_child)
                        .checked_add(1)
                        .ok_or(EvalError::WorkspaceBoundOverflow)?;
                    let required = steps
                        .len()
                        .checked_add(1)
                        .ok_or(EvalError::WorkspaceBoundOverflow)?;
                    workspace.reserve_vec(&mut steps, &mut steps_allocation, required)?;
                    steps.push(at);
                }
                if let NodeRef::Expr(expression) = node
                    && std::ptr::eq(expression, target)
                {
                    found = true;
                    return Ok(Flow::Stop);
                }
                let required = ancestors
                    .len()
                    .checked_add(1)
                    .ok_or(EvalError::WorkspaceBoundOverflow)?;
                workspace.reserve_vec(&mut ancestors, &mut ancestors_allocation, required)?;
                ancestors.push(0usize);
                Ok(Flow::Descend)
            }
            Visit::Exit => {
                ancestors.pop();
                if !ancestors.is_empty() {
                    steps.pop();
                }
                Ok(Flow::Descend)
            }
        },
        |bytes| {
            workspace.charge(u64::try_from(bytes).map_err(|_| EvalError::WorkspaceBoundOverflow)?)
        },
    );
    match walked {
        Ok(_) if found => Ok(Some((steps, steps_allocation))),
        Ok(_) => Ok(None),
        Err(WalkError::Visitor(error)) => Err(error),
        Err(WalkError::Allocation(_)) => Err(EvalError::AllocationFailed {
            construct: "copied expression site walk",
        }),
        Err(WalkError::LayoutOverflow) => Err(EvalError::WorkspaceBoundOverflow),
    }
}

/// Instruction position retained across a recursive suspension. The value
/// stack and evidence remain in the corresponding `Linked` owner.
#[must_use]
pub(crate) struct TermCursor {
    pc: usize,
}

/// One FILTER row's VM continuation. Its program counter survives every
/// `EXISTS` suspension; the linked owner retains the value stack and evidence.
pub(crate) struct FilterRowState {
    cursor: TermCursor,
}

/// The actual sequential FILTER row suspended while its EXISTS child runs.
/// The link retains the copied pattern owner and the row retains its grants;
/// neither is borrowed from this movable frame.
pub(crate) struct FilterRowFrame<'e, I: purrdf_core::ViewTermId> {
    linked: Linked<'e, I>,
    row: crate::solution::RetainedRow<I>,
    state: FilterRowState,
}

pub(crate) type CompletedFilterRow<'e, I> = (
    Linked<'e, I>,
    crate::solution::RetainedRow<I>,
    bool,
    Option<crate::evidence_exec::RowEvidence>,
);

pub(crate) enum FilterRowStep<'e, I: purrdf_core::ViewTermId> {
    Complete(CompletedFilterRow<'e, I>),
    Exists(FilterRowFrame<'e, I>, u32),
}

pub(crate) enum FilterPoll<I> {
    Complete(Option<SolutionTerm<I>>),
    Exists(u32),
}

impl FilterRowState {
    pub(crate) fn begin<D: DatasetView + Sync>(
        linked: &mut Linked<'_, D::Id>,
        ctx: &EvalCtx<'_, D>,
    ) -> Result<Self, EvalError> {
        linked.begin_term(ctx)?;
        Ok(Self {
            cursor: TermCursor { pc: 0 },
        })
    }

    /// Stop before entering an EXISTS child. The caller may retain this
    /// cursor and the linked value stack in its admitted worklist frame.
    pub(crate) fn poll<D: DatasetView + Sync>(
        &mut self,
        linked: &mut Linked<'_, D::Id>,
        expression: &Expression,
        row: &crate::solution::RetainedRow<D::Id>,
        schema: &VarSchema,
        ctx: &mut EvalCtx<'_, D>,
    ) -> Result<FilterPoll<D::Id>, EvalError> {
        loop {
            match linked.advance_term(&mut self.cursor, row, ctx)? {
                VmStep::Value(value) => return Ok(FilterPoll::Complete(value)),
                VmStep::Suspend(Suspend::Exists(site)) => return Ok(FilterPoll::Exists(site)),
                VmStep::Suspend(suspend @ Suspend::SparqlUdf { .. }) => {
                    let (value, record) = resolve(
                        suspend,
                        linked.exists.as_deref(),
                        Some(expression),
                        &linked.program,
                        row,
                        schema,
                        ctx,
                    )?;
                    self.resume(linked, value, record, ctx)?;
                }
            }
        }
    }

    pub(crate) fn resume<D: DatasetView + Sync>(
        &self,
        linked: &mut Linked<'_, D::Id>,
        value: Option<SolutionTerm<D::Id>>,
        record: Option<crate::evidence_exec::RowEvidence>,
        ctx: &EvalCtx<'_, D>,
    ) -> Result<(), EvalError> {
        linked.resume_term(value, record, &self.cursor, ctx)
    }

    fn finish<D: DatasetView + Sync>(
        self,
        linked: &Linked<'_, D::Id>,
        expression: &Expression,
        row: &crate::solution::RetainedRow<D::Id>,
        schema: &VarSchema,
        value: Option<SolutionTerm<D::Id>>,
        ctx: &mut EvalCtx<'_, D>,
    ) -> Result<(bool, Option<crate::evidence_exec::RowEvidence>), EvalError> {
        let value = match value {
            Some(term) => helpers::ebv_term(ctx, term)?,
            None => None,
        };
        linked.finish_filter_value(value, expression, row, schema, ctx)
    }
}

impl<'e, I: purrdf_core::ViewTermId> FilterRowFrame<'e, I> {
    pub(crate) fn row(&self) -> &crate::solution::RetainedRow<I> {
        &self.row
    }

    pub(crate) fn begin<D: DatasetView<Id = I> + Sync>(
        mut linked: Linked<'e, I>,
        row: crate::solution::RetainedRow<I>,
        ctx: &EvalCtx<'_, D>,
    ) -> Result<Self, EvalError> {
        let state = FilterRowState::begin(&mut linked, ctx)?;
        Ok(Self { linked, row, state })
    }

    /// Prepare the exact compiled EXISTS site while this row and link remain
    /// retained in the admitted suspension frame. The same decision can be
    /// completed by the resident adapter or entered by the graph worklist.
    pub(crate) fn prepare_exists<D: DatasetView<Id = I> + Sync>(
        &self,
        site: u32,
        expression: &Expression,
        schema: &VarSchema,
        ctx: &mut EvalCtx<'_, D>,
    ) -> Result<
        (
            crate::expr::ExistsDecision<I>,
            Option<crate::evidence_exec::RowEvidence>,
        ),
        EvalError,
    > {
        let sites = self
            .linked
            .exists
            .as_deref()
            .ok_or_else(|| EvalError::Invariant(EvaluatorInvariant::ExistsPatternMissing))?;
        let pattern = sites.pattern(site, Some(expression))?;
        let mut record = ctx
            .may_have_evidence()
            .then(crate::evidence_exec::RowEvidence::default);
        let decision =
            crate::expr::exists_recorded_decide(pattern, &self.row, schema, ctx, record.as_mut())?;
        Ok((decision, record))
    }

    pub(crate) fn step<D: DatasetView<Id = I> + Sync>(
        mut self,
        expression: &Expression,
        schema: &VarSchema,
        ctx: &mut EvalCtx<'_, D>,
    ) -> Result<FilterRowStep<'e, I>, EvalError> {
        match self
            .state
            .poll(&mut self.linked, expression, &self.row, schema, ctx)?
        {
            FilterPoll::Exists(site) => Ok(FilterRowStep::Exists(self, site)),
            FilterPoll::Complete(value) => {
                let (keep, evidence) =
                    self.state
                        .finish(&self.linked, expression, &self.row, schema, value, ctx)?;
                Ok(FilterRowStep::Complete((
                    self.linked,
                    self.row,
                    keep,
                    evidence,
                )))
            }
        }
    }

    pub(crate) fn resume<D: DatasetView<Id = I> + Sync>(
        &mut self,
        value: Option<SolutionTerm<I>>,
        record: Option<crate::evidence_exec::RowEvidence>,
        ctx: &EvalCtx<'_, D>,
    ) -> Result<(), EvalError> {
        self.state.resume(&mut self.linked, value, record, ctx)
    }
}

impl<'e, I: purrdf_core::ViewTermId> Linked<'e, I> {
    /// Link `program`, compiled from `expr`, to a call over `schema`.
    #[cfg(test)]
    pub(crate) fn link<D: DatasetView<Id = I> + Sync>(
        program: impl Into<crate::workspace::SharedWorkspace<ExprProgram>>,
        expr: &'e Expression,
        schema: &VarSchema,
        ctx: &mut EvalCtx<'_, D>,
    ) -> Self {
        Self::link_admitted(program, expr, schema, ctx).expect("resident VM link allocation")
    }

    pub(crate) fn link_admitted<D: DatasetView<Id = I> + Sync>(
        program: impl Into<crate::workspace::SharedWorkspace<ExprProgram>>,
        expr: &'e Expression,
        schema: &VarSchema,
        ctx: &mut EvalCtx<'_, D>,
    ) -> Result<Self, EvalError> {
        Self::link_with_owner(
            program.into(),
            expr,
            ExpressionOwner::Borrowed(expr),
            schema,
            ctx,
        )
    }

    /// A FILTER in a copied window chooses the same immutable owner the
    /// substitution runner retains. The owned link itself has no borrow into
    /// that window, so it can move with a suspended worklist frame.
    pub(crate) fn link_filter_admitted<D: DatasetView<Id = I> + Sync>(
        program: impl Into<crate::workspace::SharedWorkspace<ExprProgram>>,
        expr: &'e Expression,
        schema: &VarSchema,
        ctx: &mut EvalCtx<'_, D>,
    ) -> Result<Self, EvalError> {
        let program = program.into();
        if program.has_exists()
            && let Some(pattern) = ctx.active_copied_pattern.clone()
            && let Some((steps, allocation)) = copied_expression_path(&pattern, expr, &ctx.growth)?
        {
            return Self::link_copied_filter_admitted(
                program, expr, pattern, steps, allocation, schema, ctx,
            );
        }
        Self::link_admitted(program, expr, schema, ctx)
    }

    /// Link against an explicitly retained copied tree. A suspension frame may
    /// own the returned link without borrowing the temporary `expr` used to
    /// compile it. The caller derives the path with the one copied-tree walk.
    pub(crate) fn link_copied_filter_admitted<D: DatasetView<Id = I> + Sync>(
        program: crate::workspace::SharedWorkspace<ExprProgram>,
        expr: &Expression,
        pattern: crate::expr::PreparedPattern,
        steps: Vec<usize>,
        allocation: Option<crate::WorkspaceAllocation>,
        schema: &VarSchema,
        ctx: &mut EvalCtx<'_, D>,
    ) -> Result<Linked<'static, I>, EvalError> {
        Self::link_with_owner(
            program,
            expr,
            ExpressionOwner::Copied {
                pattern,
                steps,
                _allocation: allocation,
            },
            schema,
            ctx,
        )
    }

    /// Resolve the compiled FILTER against the immutable copied pattern kept
    /// by a movable graph frame. No expression borrow escapes this call.
    pub(crate) fn link_copied_filter_owner_admitted<D: DatasetView<Id = I> + Sync>(
        program: crate::workspace::SharedWorkspace<ExprProgram>,
        expr: &Expression,
        pattern: crate::expr::PreparedPattern,
        schema: &VarSchema,
        ctx: &mut EvalCtx<'_, D>,
    ) -> Result<Linked<'static, I>, EvalError> {
        let (steps, allocation) = if program.has_exists() {
            copied_expression_path(&pattern, expr, &ctx.growth)?.ok_or_else(|| {
                EvalError::Invariant(EvaluatorInvariant::CopiedFilterExpressionMissing)
            })?
        } else {
            (Vec::new(), None)
        };
        Self::link_copied_filter_admitted(program, expr, pattern, steps, allocation, schema, ctx)
    }

    fn link_with_owner<'owner, D: DatasetView<Id = I> + Sync>(
        program: crate::workspace::SharedWorkspace<ExprProgram>,
        expr: &Expression,
        owner: ExpressionOwner<'owner>,
        schema: &VarSchema,
        ctx: &mut EvalCtx<'_, D>,
    ) -> Result<Linked<'owner, I>, EvalError> {
        let mut exists_allocation = None;
        let mut exists = Vec::new();
        if program.has_exists() {
            struct WalkFrame<'x> {
                expr: &'x Expression,
                child_start: usize,
                child_count: usize,
                next: usize,
                entered: bool,
            }
            let mut pending_allocation = None;
            let mut pending = Vec::new();
            ctx.growth
                .reserve_vec(&mut pending, &mut pending_allocation, 1)?;
            pending.push(WalkFrame {
                expr,
                child_start: 0,
                child_count: 0,
                next: 0,
                entered: false,
            });
            let mut children_allocation = None;
            let mut children = Vec::new();
            let mut path_allocation = None;
            let mut path = Vec::new();
            while let Some(frame) = pending.last_mut() {
                if !frame.entered {
                    if matches!(frame.expr, Expression::Exists(_)) {
                        let required = exists
                            .len()
                            .checked_add(1)
                            .ok_or(EvalError::WorkspaceBoundOverflow)?;
                        ctx.growth
                            .reserve_vec(&mut exists, &mut exists_allocation, required)?;
                        let mut allocation = None;
                        let mut steps = Vec::new();
                        ctx.growth
                            .reserve_vec(&mut steps, &mut allocation, path.len())?;
                        steps.extend_from_slice(&path);
                        exists.push(ExistsSite {
                            steps,
                            _allocation: allocation,
                        });
                    }
                    frame.child_start = children.len();
                    let mut failure = None;
                    compile::visit_operands(frame.expr, |operand| {
                        if failure.is_some() {
                            return;
                        }
                        let Some(required) = children.len().checked_add(1) else {
                            failure = Some(EvalError::WorkspaceBoundOverflow);
                            return;
                        };
                        match ctx.growth.reserve_vec(
                            &mut children,
                            &mut children_allocation,
                            required,
                        ) {
                            Ok(()) => children.push(operand),
                            Err(error) => failure = Some(error),
                        }
                    });
                    if let Some(error) = failure {
                        return Err(error);
                    }
                    frame.child_count = children.len() - frame.child_start;
                    frame.entered = true;
                }
                if frame.next < frame.child_count {
                    let next = frame.next;
                    let child = children[frame.child_start + next];
                    frame.next += 1;
                    let required_pending = pending
                        .len()
                        .checked_add(1)
                        .ok_or(EvalError::WorkspaceBoundOverflow)?;
                    ctx.growth.reserve_vec(
                        &mut pending,
                        &mut pending_allocation,
                        required_pending,
                    )?;
                    let required_path = path
                        .len()
                        .checked_add(1)
                        .ok_or(EvalError::WorkspaceBoundOverflow)?;
                    ctx.growth
                        .reserve_vec(&mut path, &mut path_allocation, required_path)?;
                    path.push(next);
                    pending.push(WalkFrame {
                        expr: child,
                        child_start: 0,
                        child_count: 0,
                        next: 0,
                        entered: false,
                    });
                } else {
                    let done = pending.pop().expect("nonempty DFS stack");
                    children.truncate(done.child_start);
                    if !pending.is_empty() {
                        path.pop();
                    }
                }
            }
        }
        let exists = if exists.is_empty() {
            None
        } else {
            Some(crate::workspace::SharedWorkspace::new_admitted(
                ExistsSites {
                    owner,
                    sites: exists,
                    _allocation: exists_allocation,
                },
                &ctx.growth,
            )?)
        };
        Linked::link_parts(program, exists, schema, ctx)
    }

    /// Link a program whose expression is not at hand. An `EXISTS` in it has no
    /// pattern to test and fails the run as an internal error.
    pub(crate) fn link_without_exists_admitted<D: DatasetView<Id = I> + Sync>(
        program: impl Into<crate::workspace::SharedWorkspace<ExprProgram>>,
        schema: &VarSchema,
        ctx: &mut EvalCtx<'_, D>,
    ) -> Result<Self, EvalError> {
        Self::link_parts(program.into(), None, schema, ctx)
    }

    fn link_parts<D: DatasetView<Id = I> + Sync>(
        program: crate::workspace::SharedWorkspace<ExprProgram>,
        exists: Option<crate::workspace::SharedWorkspace<ExistsSites<'e>>>,
        schema: &VarSchema,
        ctx: &mut EvalCtx<'_, D>,
    ) -> Result<Self, EvalError> {
        let stack_capacity = program.ops.len();
        let args_capacity = program
            .ops
            .iter()
            .filter_map(|op| match op {
                Op::Call { argc, .. } | Op::CallCustom { argc, .. } => Some(*argc as usize),
                _ => None,
            })
            .max()
            .unwrap_or(0);
        let arrays = [
            std::alloc::Layout::array::<Option<usize>>(if program.vars.len() > 4 {
                program.vars.len()
            } else {
                0
            }),
            std::alloc::Layout::array::<ConstCell<I>>(if program.consts.len() > 4 {
                program.consts.len()
            } else {
                0
            }),
            std::alloc::Layout::array::<RegexSlot>(program.regexes.len()),
            std::alloc::Layout::array::<Val<I>>(stack_capacity),
        ];
        let mut bytes = 0usize;
        for layout in arrays {
            bytes = bytes
                .checked_add(
                    layout
                        .map_err(|_| EvalError::WorkspaceBoundOverflow)?
                        .size(),
                )
                .ok_or(EvalError::WorkspaceBoundOverflow)?;
        }
        let allocation = ctx
            .growth
            .charge(u64::try_from(bytes).map_err(|_| EvalError::WorkspaceBoundOverflow)?)?;
        let mut slots = purrdf_core::SmallVec::new();
        slots
            .try_reserve_exact(program.vars.len())
            .map_err(|_| EvalError::AllocationFailed {
                construct: "VM variable slots",
            })?;
        for var in &program.vars {
            slots.push(schema.index_of(var));
        }
        let mut consts = purrdf_core::SmallVec::new();
        consts
            .try_reserve_exact(program.consts.len())
            .map_err(|_| EvalError::AllocationFailed {
                construct: "VM constant cells",
            })?;
        for _ in 0..program.consts.len() {
            consts.push(ConstCell::Unread);
        }
        let mut regexes = Vec::new();
        regexes
            .try_reserve_exact(program.regexes.len())
            .map_err(|_| EvalError::AllocationFailed {
                construct: "VM regex slots",
            })?;
        for constant in &program.regexes {
            regexes.push(match constant {
                Some((pattern, flags)) => {
                    RegexSlot::Linked(crate::xpath_regex::LinkedPattern::link(ctx, pattern, flags))
                }
                None => RegexSlot::PerRow,
            });
        }
        let mut stack = Vec::new();
        stack
            .try_reserve_exact(stack_capacity)
            .map_err(|_| EvalError::AllocationFailed {
                construct: "VM value stack",
            })?;
        let args = Arguments::with_capacity(args_capacity, &ctx.growth)?;
        Ok(Self {
            consts,
            slots,
            regexes,
            exists,
            stack,
            evidence: None,
            args,
            program,
            _allocation: Some(allocation),
        })
    }

    /// A copy for a forked worker: the same slots, regexes and patterns, with a constant
    /// pool of its own, since the worker interns into its own scratch.
    pub(crate) fn fresh(&self) -> Self {
        Self {
            program: self.program.clone(),
            slots: self.slots.clone(),
            consts: purrdf_core::smallvec![ConstCell::Unread; self.consts.len()],
            regexes: self.regexes.clone(),
            exists: self.exists.clone(),
            stack: Vec::new(),
            evidence: None,
            args: Arguments::new(&crate::WorkspaceCapability::resident()),
            _allocation: None,
        }
    }

    /// The expression's effective boolean value over `row` (`Ok(None)` is an error or
    /// unbound).
    pub(crate) fn ebv<D: DatasetView<Id = I> + Sync>(
        &mut self,
        expression: Option<&Expression>,
        row: &[Option<SolutionTerm<I>>],
        schema: &VarSchema,
        ctx: &mut EvalCtx<'_, D>,
    ) -> Result<Option<bool>, EvalError> {
        match self.term(expression, row, schema, ctx)? {
            Some(term) => Ok(helpers::ebv_term(ctx, term)?),
            None => Ok(None),
        }
    }

    /// Decide a native filter and retain its original witness/gap records.
    /// A kept ordinary row has no replacement attachment; rejection is separate
    /// from that physical absence so no virtual payload crosses the row loop.
    pub(crate) fn filter<D: DatasetView<Id = I> + Sync>(
        &mut self,
        expression: &Expression,
        row: &crate::solution::RetainedRow<I>,
        schema: &VarSchema,
        ctx: &mut EvalCtx<'_, D>,
    ) -> Result<(bool, Option<crate::evidence_exec::RowEvidence>), EvalError> {
        let value = self.ebv(Some(expression), row, schema, ctx)?;
        self.finish_filter_value(value, expression, row, schema, ctx)
    }

    fn finish_filter_value<D: DatasetView<Id = I> + Sync>(
        &self,
        value: Option<bool>,
        expression: &Expression,
        row: &crate::solution::RetainedRow<I>,
        schema: &VarSchema,
        ctx: &EvalCtx<'_, D>,
    ) -> Result<(bool, Option<crate::evidence_exec::RowEvidence>), EvalError> {
        if self.evidence.is_none() && ctx.evidence_input.is_none() && !row.has_evidence_attachment()
        {
            return Ok((value == Some(true), None));
        }
        let evidence =
            helpers::filter_evidence(value, self.result_evidence()?, expression, row, schema, ctx)?;
        Ok((evidence.is_some(), evidence))
    }

    /// The expression's value over `row`: `Ok(Some)` a term, `Ok(None)` an error or
    /// unbound, `Err` a hard failure. `schema` is the schema the program was linked to.
    pub(crate) fn term<D: DatasetView<Id = I> + Sync>(
        &mut self,
        expression: Option<&Expression>,
        row: &[Option<SolutionTerm<I>>],
        schema: &VarSchema,
        ctx: &mut EvalCtx<'_, D>,
    ) -> Result<Option<SolutionTerm<I>>, EvalError> {
        self.begin_term(ctx)?;
        let mut cursor = TermCursor { pc: 0 };
        loop {
            match self.advance_term(&mut cursor, row, ctx)? {
                VmStep::Value(value) => return Ok(value),
                VmStep::Suspend(suspend) => {
                    let (value, record) = resolve(
                        suspend,
                        self.exists.as_deref(),
                        expression,
                        &self.program,
                        row,
                        schema,
                        ctx,
                    )?;
                    self.resume_term(value, record, &cursor, ctx)?;
                }
            }
        }
    }

    /// Start one expression exactly once; a suspended execution retains its
    /// program counter and stack instead of replaying any previous instruction.
    fn begin_term<D: DatasetView<Id = I> + Sync>(
        &mut self,
        ctx: &EvalCtx<'_, D>,
    ) -> Result<(), EvalError> {
        self.stack.clear();
        if ctx.may_have_evidence()
            && self.evidence.is_none()
            && (self.program.has_exists()
                || self.program.calls.iter().any(|function| {
                    matches!(function, Function::Custom(iri) if ctx.user_functions.resolve(iri.as_str()).is_some())
                }))
        {
            self.evidence = Some(crate::workspace::SharedWorkspace::new_admitted(
                std::sync::Mutex::new(evidence::Records::admitted(&ctx.growth)),
                &ctx.growth,
            )?);
        }
        if let Some(owner) = &self.evidence {
            owner.lock().map_err(|_| evidence_poisoned())?.clear();
        }
        Ok(())
    }

    fn advance_term<'d, D: DatasetView<Id = I> + Sync>(
        &mut self,
        cursor: &mut TermCursor,
        row: &[Option<SolutionTerm<I>>],
        ctx: &mut EvalCtx<'d, D>,
    ) -> Result<VmStep<'d, I>, EvalError> {
        if self.evidence.is_some() {
            self.advance::<true, D>(&mut cursor.pc, row, ctx)
        } else {
            self.advance::<false, D>(&mut cursor.pc, row, ctx)
        }
    }

    fn resume_term<D: DatasetView<Id = I> + Sync>(
        &mut self,
        value: Option<SolutionTerm<I>>,
        record: Option<crate::evidence_exec::RowEvidence>,
        cursor: &TermCursor,
        ctx: &EvalCtx<'_, D>,
    ) -> Result<(), EvalError> {
        self.stack.push(Val::Term(value));
        if let Some(owner) = &self.evidence {
            owner.lock().map_err(|_| evidence_poisoned())?.resume(
                self.stack.len() - 1,
                record,
                cursor.pc,
                &ctx.growth,
            )?;
        }
        Ok(())
    }

    /// Borrowed-value execution's original evidence, never an expression replay.
    pub(crate) fn result_evidence(
        &self,
    ) -> Result<Option<crate::evidence_exec::RowEvidence>, EvalError> {
        self.evidence.as_ref().map_or(Ok(None), |owner| {
            Ok(owner
                .lock()
                .map_err(|_| evidence_poisoned())?
                .result
                .clone())
        })
    }

    /// Run instructions up to a recursive suspension or the completed value.
    /// The instruction temporaries must die before `term` resolves the suspension:
    /// keeping this frame live would charge every opcode's native owners to each
    /// nested EXISTS or SPARQL function, even when that opcode never runs there.
    #[inline(never)]
    fn advance<'d, const TRACKING: bool, D: DatasetView<Id = I> + Sync>(
        &mut self,
        pc: &mut usize,
        row: &[Option<SolutionTerm<I>>],
        ctx: &mut EvalCtx<'d, D>,
    ) -> Result<VmStep<'d, I>, EvalError> {
        let Self {
            program,
            slots,
            consts,
            regexes,
            exists,
            stack,
            evidence,
            args,
            ..
        } = self;
        let mut records = if TRACKING {
            evidence
                .as_ref()
                .map(|owner| owner.lock().map_err(|_| evidence_poisoned()))
                .transpose()?
        } else {
            None
        };
        let ops = &program.ops;
        while let Some(op) = ops.get(*pc) {
            *pc += 1;
            let before = stack.len();
            let record = if TRACKING {
                records
                    .as_mut()
                    .map(|records| records.begin(*op, before, ctx))
                    .transpose()?
                    .flatten()
            } else {
                None
            };
            match *op {
                Op::Const(k) => {
                    let k = k as usize;
                    let term = match consts[k] {
                        ConstCell::Read(term) => term,
                        ConstCell::Unread => {
                            let value = program.consts[k].value_admitted(&ctx.growth)?;
                            let term = ctx.intern_workspace_term(value)?;
                            consts[k] = ConstCell::Read(term);
                            term
                        }
                    };
                    stack.push(Val::Term(term));
                }
                Op::Var(slot) => {
                    stack.push(Val::Term(slots[slot as usize].and_then(|c| row[c])));
                }
                Op::Bound(slot) => {
                    let bound = slots[slot as usize].and_then(|c| row[c]).is_some();
                    stack.push(Val::Term(Some(helpers::intern_boolean(ctx, bound)?)));
                }
                Op::EbvOf => {
                    let term = pop_term(stack)?;
                    stack.push(Val::Ebv(
                        term.map(|t| helpers::ebv_term(ctx, t))
                            .transpose()?
                            .flatten(),
                    ));
                }
                Op::Kleene { or, n } => {
                    let start = stack.len().checked_sub(n as usize).ok_or_else(underflow)?;
                    let mut value = Some(!or);
                    for entry in stack.drain(start..) {
                        let Val::Ebv(operand) = entry else {
                            return Err(mistyped(StackExpectation::Boolean));
                        };
                        value = if or {
                            helpers::kleene_or(value, operand)
                        } else {
                            helpers::kleene_and(value, operand)
                        };
                    }
                    stack.push(Val::Term(
                        value.map(|b| helpers::intern_boolean(ctx, b)).transpose()?,
                    ));
                }
                Op::Not | Op::NotExists => {
                    let ordinary = pop_ebv(stack)?.map(|value| !value);
                    let value = if matches!(op, Op::NotExists)
                        && let Some(record) = &record
                        && !record.is_undecided()
                    {
                        Some(record.with_record(|record| {
                            record.information().fallback()
                                != purrdf_core::evidence::Information::Opposed
                                || !record.information().entries().is_empty()
                        })?)
                    } else {
                        ordinary
                    };
                    stack.push(Val::Term(
                        value.map(|b| helpers::intern_boolean(ctx, b)).transpose()?,
                    ));
                }
                Op::Equal => {
                    let b = pop_term(stack)?;
                    let a = pop_term(stack)?;
                    stack.push(Val::Term(helpers::equal_terms(ctx, a, b)?));
                }
                Op::SameTerm => {
                    let b = pop_term(stack)?;
                    let a = pop_term(stack)?;
                    stack.push(Val::Term(match (a, b) {
                        (Some(x), Some(y)) => Some(helpers::intern_boolean(ctx, x == y)?),
                        _ => None,
                    }));
                }
                Op::Cmp(cmp) => {
                    let b = pop_term(stack)?;
                    let a = pop_term(stack)?;
                    stack.push(Val::Term(compare(ctx, cmp, a, b)?));
                }
                Op::Branch { on_false, end } => match pop_ebv(stack)? {
                    Some(true) => {}
                    Some(false) => *pc = on_false as usize,
                    None => {
                        stack.push(Val::Term(None));
                        *pc = end as usize;
                    }
                },
                Op::Jmp(target) => *pc = target as usize,
                Op::CoalesceNext(end) => {
                    if matches!(stack.last(), Some(Val::Term(Some(_)))) {
                        *pc = end as usize;
                    } else {
                        pop_term(stack)?;
                    }
                }
                Op::PushUnbound => stack.push(Val::Term(None)),
                Op::InNeedle(end) => match pop_term(stack)? {
                    None => {
                        stack.push(Val::Term(None));
                        *pc = end as usize;
                    }
                    Some(target) => {
                        let value = helpers::owned_value_of(ctx, target)?;
                        stack.push(Val::In {
                            target,
                            value,
                            saw_error: false,
                        });
                    }
                },
                Op::InItem(end) => {
                    let candidate = pop_term(stack)?;
                    let Some(Val::In {
                        target,
                        value,
                        saw_error,
                    }) = stack.last_mut()
                    else {
                        return Err(mistyped(StackExpectation::OpenIn));
                    };
                    let matched = match candidate {
                        None => {
                            *saw_error = true;
                            false
                        }
                        Some(candidate) => {
                            match helpers::in_candidate(ctx, *target, value, candidate)? {
                                Some(true) => true,
                                Some(false) => false,
                                None => {
                                    *saw_error = true;
                                    false
                                }
                            }
                        }
                    };
                    if matched {
                        stack.pop();
                        stack.push(Val::Term(Some(helpers::intern_boolean(ctx, true)?)));
                        *pc = end as usize;
                    }
                }
                Op::InEnd => {
                    let Some(Val::In { saw_error, .. }) = stack.pop() else {
                        return Err(mistyped(StackExpectation::OpenIn));
                    };
                    stack.push(Val::Term(if saw_error {
                        None
                    } else {
                        Some(helpers::intern_boolean(ctx, false)?)
                    }));
                }
                Op::Arith(operator) => {
                    let right = pop_term(stack)?;
                    let left = pop_term(stack)?;
                    stack.push(Val::Term(match (left, right) {
                        (Some(a), Some(b)) => helpers::arithmetic_step(ctx, operator, a, b)?,
                        _ => None,
                    }));
                }
                Op::UnaryPlus => {
                    let operand = pop_term(stack)?;
                    stack.push(Val::Term(helpers::unary_numeric_term(
                        ctx,
                        operand,
                        purrdf_xsd::numeric::NumericUnaryOperator::Plus,
                    )?));
                }
                Op::UnaryMinus => {
                    let operand = pop_term(stack)?;
                    stack.push(Val::Term(helpers::unary_numeric_term(
                        ctx,
                        operand,
                        purrdf_xsd::numeric::NumericUnaryOperator::Minus,
                    )?));
                }
                Op::Exists(site) => {
                    exists
                        .as_ref()
                        .and_then(|sites| sites.sites.get(site as usize))
                        .ok_or_else(|| {
                            EvalError::Invariant(EvaluatorInvariant::ExistsPatternMissing)
                        })?;
                    if let Some(records) = &mut records {
                        records.suspend(record);
                    }
                    return Ok(VmStep::Suspend(Suspend::Exists(site)));
                }
                Op::Call { call, argc, regex } => {
                    pop_values(stack, argc, ctx, args)?;
                    let compiled = regex.and_then(|slot| regexes[slot as usize].linked());
                    let result =
                        helpers::apply_function(&program.calls[call as usize], args, ctx, compiled);
                    args.clear();
                    stack.push(Val::Term(result?));
                }
                Op::CallCustom { call, argc } => {
                    pop_values(stack, argc, ctx, args)?;
                    let result = call_custom(program, call, args, ctx);
                    args.clear();
                    match result? {
                        VmStep::Value(value) => stack.push(Val::Term(value)),
                        suspension @ VmStep::Suspend(_) => {
                            if let Some(records) = &mut records {
                                records.suspend(record);
                            }
                            return Ok(suspension);
                        }
                    }
                }
                Op::Triple { intern } => {
                    let object = match stack.pop() {
                        Some(Val::Term(term)) => {
                            term.map(|t| helpers::owned_value_of(ctx, t)).transpose()?
                        }
                        Some(Val::Value(value)) => value,
                        Some(_) => return Err(mistyped(StackExpectation::TermOrTriple)),
                        None => return Err(underflow()),
                    };
                    let predicate = pop_term(stack)?
                        .map(|t| helpers::owned_value_of(ctx, t))
                        .transpose()?;
                    let subject = pop_term(stack)?
                        .map(|t| helpers::owned_value_of(ctx, t))
                        .transpose()?;
                    let value =
                        helpers::triple_value_admitted(subject, predicate, object, &ctx.growth)?;
                    stack.push(if intern {
                        Val::Term(
                            value
                                .map(|value| ctx.intern_workspace_term(value))
                                .transpose()?
                                .flatten(),
                        )
                    } else {
                        Val::Value(value)
                    });
                }
                Op::StrConst(k) => {
                    stack.push(Val::StrConst(k));
                }
                Op::StrNone => stack.push(Val::Str(None)),
                Op::ToStrArg => {
                    let term = pop_term(stack)?;
                    stack.push(Val::Str(
                        term.map(|t| helpers::string_arg_of_term(ctx, t))
                            .transpose()?
                            .flatten(),
                    ));
                }
                Op::ToStrLexical => {
                    let term = pop_term(stack)?;
                    stack.push(Val::Str(
                        term.map(|t| helpers::str_lexical_term(ctx, t))
                            .transpose()?
                            .flatten(),
                    ));
                }
                Op::ToLangLexical => {
                    let term = pop_term(stack)?;
                    stack.push(Val::Str(
                        term.map(|t| helpers::lang_lexical_term(ctx, t))
                            .transpose()?
                            .flatten(),
                    ));
                }
                Op::StrPred(pred) => {
                    let needle = pop_str(stack, &program.strs)?;
                    let haystack = pop_str(stack, &program.strs)?;
                    let value = match (haystack, needle) {
                        // §17.4.1.1: an incompatible pair is an error, not `false`.
                        (Some(h), Some(n))
                            if helpers::args_compatible(
                                h.parts().1,
                                h.parts().2,
                                n.parts().1,
                                n.parts().2,
                            ) =>
                        {
                            let (h, n) = (h.parts().0, n.parts().0);
                            let holds = match pred {
                                StrPred::Contains => h.contains(n),
                                StrPred::StrStarts => h.starts_with(n),
                                StrPred::StrEnds => h.ends_with(n),
                            };
                            Some(helpers::intern_boolean(ctx, holds)?)
                        }
                        _ => None,
                    };
                    stack.push(Val::Term(value));
                }
                Op::Regex { slot, flags } => {
                    // Omitted flags are no flags. Supplied flags are a simple literal
                    // (§17.4.3.14); unbound, an error, any other term or a tagged
                    // string is an error, never "no flags". The stack holds the
                    // operands text, pattern, flags, so they pop in reverse.
                    let flags = if flags {
                        pop_str(stack, &program.strs)?.filter(|f| f.parts().1.is_none())
                    } else {
                        Some(StrArg::Borrowed(&NO_FLAGS))
                    };
                    // The pattern is a simple literal too; the text may be any string.
                    let pattern = pop_str(stack, &program.strs)?.filter(|p| p.parts().1.is_none());
                    let text = pop_str(stack, &program.strs)?;
                    let value = match (text, pattern, flags) {
                        (Some(text), Some(pattern), Some(flags)) => {
                            let flags = flags.parts().0;
                            let compiled = crate::xpath_regex::resolve(
                                ctx,
                                pattern.parts().0,
                                flags,
                                regexes[slot as usize].linked(),
                            )?;
                            match compiled {
                                Some(re) => re
                                    .is_match(text.parts().0, &ctx.growth)?
                                    .map(|holds| helpers::intern_boolean(ctx, holds))
                                    .transpose()?,
                                None => None,
                            }
                        }
                        _ => None,
                    };
                    stack.push(Val::Term(value));
                }
                Op::LangMatches => {
                    let range = pop_str(stack, &program.strs)?;
                    let tag = pop_str(stack, &program.strs)?;
                    // Both are simple literals (§17.4.3.13); a tagged one is an error.
                    let value = match (tag, range) {
                        (Some(tag), Some(range))
                            if tag.parts().1.is_none() && range.parts().1.is_none() =>
                        {
                            Some(helpers::intern_boolean(
                                ctx,
                                helpers::lang_matches(tag.parts().0, range.parts().0),
                            )?)
                        }
                        _ => None,
                    };
                    stack.push(Val::Term(value));
                }
            }
            if let Some(records) = &mut records {
                records.finish(*op, before, stack.len(), *pc, record, &ctx.growth)?;
            }
        }
        if let Some(records) = &mut records {
            let position = stack.len().checked_sub(1).ok_or_else(underflow)?;
            records.complete(position);
        }
        drop(records);
        let value = pop_term(stack)?;
        if !stack.is_empty() {
            return Err(EvalError::Invariant(EvaluatorInvariant::VmExtraStackValues));
        }
        Ok(VmStep::Value(value))
    }
}

/// The ordering comparison `cmp` over two evaluated operands.
fn compare<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    cmp: Cmp,
    a: Option<SolutionTerm<D::Id>>,
    b: Option<SolutionTerm<D::Id>>,
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    use crate::cdt_fn::CdtRelation;
    use std::cmp::Ordering;
    match cmp {
        Cmp::Greater => {
            helpers::compare_terms(ctx, a, b, CdtRelation::Greater, |c| c == Ordering::Greater)
        }
        Cmp::GreaterOrEqual => {
            helpers::compare_terms(ctx, a, b, CdtRelation::GreaterOrEqual, |c| {
                c != Ordering::Less
            })
        }
        Cmp::Less => helpers::compare_terms(ctx, a, b, CdtRelation::Less, |c| c == Ordering::Less),
        Cmp::LessOrEqual => helpers::compare_terms(ctx, a, b, CdtRelation::LessOrEqual, |c| {
            c != Ordering::Greater
        }),
    }
}

/// A caller-registered function call over its evaluated arguments, probed in the order
/// the registry documents: SPARQL-bodied (which suspends), native, expression-bodied,
/// then an XSD constructor cast; an IRI none of them names is unsupported.
fn call_custom<'d, D: DatasetView + Sync>(
    program: &ExprProgram,
    call: u32,
    vals: &mut Arguments,
    ctx: &mut EvalCtx<'d, D>,
) -> Result<VmStep<'d, D::Id>, EvalError> {
    let Function::Custom(iri) = &program.calls[call as usize] else {
        return Err(EvalError::Invariant(
            EvaluatorInvariant::CustomInstructionBuiltinFunction,
        ));
    };
    let registry: &'d crate::user_fn::BoundFunctionRegistry = ctx.user_functions;
    if let Some((func, body)) = registry.resolve(iri.as_str()) {
        return Ok(VmStep::Suspend(Suspend::SparqlUdf {
            func,
            body,
            call,
            vals: vals.take(&ctx.growth),
        }));
    }
    helpers::apply_custom_host(iri.as_str(), vals, ctx).map(VmStep::Value)
}

/// Resolve a suspension by running the pattern evaluation it waits on, now.
type Resolved<I> = (
    Option<SolutionTerm<I>>,
    Option<crate::evidence_exec::RowEvidence>,
);

fn resolve<D: DatasetView + Sync>(
    suspend: Suspend<'_>,
    exists: Option<&ExistsSites<'_>>,
    expression: Option<&Expression>,
    program: &ExprProgram,
    row: &[Option<SolutionTerm<D::Id>>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Resolved<D::Id>, EvalError> {
    match suspend {
        Suspend::Exists(site) => {
            let sites = exists
                .ok_or_else(|| EvalError::Invariant(EvaluatorInvariant::ExistsPatternMissing))?;
            let pattern = sites.pattern(site, expression)?;
            if ctx.may_have_evidence() {
                let (found, record) = helpers::exists_evidence(pattern, row, schema, ctx)?;
                Ok((Some(helpers::intern_boolean(ctx, found)?), Some(record)))
            } else {
                let found = helpers::exists(pattern, row, schema, ctx)?;
                Ok((Some(helpers::intern_boolean(ctx, found)?), None))
            }
        }
        Suspend::SparqlUdf {
            func,
            body,
            call,
            vals,
        } => {
            let Function::Custom(iri) = &program.calls[call as usize] else {
                return Err(EvalError::Invariant(
                    EvaluatorInvariant::SuspendedCallBuiltinFunction,
                ));
            };
            let mut evidence = None;
            let result = crate::user_fn::eval_user_function_evidence(
                func,
                body,
                iri.as_str(),
                &vals,
                ctx,
                &mut evidence,
            )?;
            Ok((
                result
                    .map(|value| helpers::intern(ctx, value))
                    .transpose()?
                    .flatten(),
                evidence,
            ))
        }
    }
}

fn evidence_poisoned() -> EvalError {
    EvalError::Invariant(EvaluatorInvariant::VmEvidencePoisoned)
}

/// Pop `argc` terms and materialize their values into `vals`, in argument order,
/// replacing what it held.
fn pop_values<D: DatasetView + Sync>(
    stack: &mut ValStack<D::Id>,
    argc: u32,
    ctx: &EvalCtx<'_, D>,
    vals: &mut Arguments,
) -> Result<(), EvalError> {
    let start = stack
        .len()
        .checked_sub(argc as usize)
        .ok_or_else(underflow)?;
    vals.clear();
    for entry in stack.drain(start..) {
        let Val::Term(term) = entry else {
            return Err(mistyped(StackExpectation::Term));
        };
        vals.push(term.map(|t| helpers::owned_value_of(ctx, t)).transpose()?)?;
    }
    Ok(())
}

fn pop_term<I: Copy>(stack: &mut ValStack<I>) -> Result<Option<SolutionTerm<I>>, EvalError> {
    match stack.pop() {
        Some(Val::Term(term)) => Ok(term),
        Some(_) => Err(mistyped(StackExpectation::Term)),
        None => Err(underflow()),
    }
}

fn pop_ebv<I: Copy>(stack: &mut ValStack<I>) -> Result<Option<bool>, EvalError> {
    match stack.pop() {
        Some(Val::Ebv(value)) => Ok(value),
        Some(_) => Err(mistyped(StackExpectation::Boolean)),
        None => Err(underflow()),
    }
}

/// Constants are borrowed from the authored program; runtime operands preserve
/// their own original term lease. Neither representation deep-clones on a pop.
enum StrArg<'p> {
    Borrowed(&'p helpers::StringArg),
    Owned(helpers::OwnedStringArg),
}

impl StrArg<'_> {
    fn parts(&self) -> (&str, Option<&str>, Option<purrdf_core::RdfTextDirection>) {
        match self {
            Self::Borrowed(value) => (&value.0, value.1.as_deref(), value.2),
            Self::Owned(value) => value.parts(),
        }
    }
}

/// The flags an omitted REGEX argument reads as, without an allocation.
static NO_FLAGS: helpers::StringArg = (String::new(), None, None);

fn pop_str<'p, I: Copy>(
    stack: &mut ValStack<I>,
    strs: &'p [helpers::StringArg],
) -> Result<Option<StrArg<'p>>, EvalError> {
    match stack.pop() {
        Some(Val::Str(value)) => Ok(value.map(StrArg::Owned)),
        Some(Val::StrConst(k)) => Ok(Some(StrArg::Borrowed(&strs[k as usize]))),
        Some(_) => Err(mistyped(StackExpectation::String)),
        None => Err(underflow()),
    }
}

fn underflow() -> EvalError {
    EvalError::Invariant(EvaluatorInvariant::VmStackUnderflow)
}

fn mistyped(expected: StackExpectation) -> EvalError {
    EvalError::Invariant(EvaluatorInvariant::UnexpectedStackValue { expected })
}

/// A recorder of every charge call, in order, for tests that compare two evaluations'
/// charge sequences.
#[cfg(test)]
pub(crate) mod charge_trace {
    use std::cell::RefCell;

    thread_local! {
        static TRACE: RefCell<Option<Vec<String>>> = const { RefCell::new(None) };
    }

    /// Record one charge call, when a trace is being taken on this thread.
    pub(crate) fn record(entry: impl FnOnce() -> String) {
        TRACE.with(|trace| {
            if let Some(trace) = trace.borrow_mut().as_mut() {
                trace.push(entry());
            }
        });
    }

    /// Run `f`, returning its result and every charge call it made on this thread.
    pub(crate) fn capture<T>(f: impl FnOnce() -> T) -> (T, Vec<String>) {
        let previous = TRACE.with(|trace| trace.borrow_mut().replace(Vec::new()));
        let result = f();
        let taken = TRACE.with(|trace| {
            let taken = trace.borrow_mut().take().unwrap_or_default();
            *trace.borrow_mut() = previous;
            taken
        });
        (result, taken)
    }
}

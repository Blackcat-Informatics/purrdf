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
//! taking already-evaluated operands, so every value interned, every charge, every
//! draw of the query's random state and every `BNODE` memo lookup happens exactly as
//! it did. See [`compile`](self::compile) for how the compiler keeps the set of
//! interned values identical.
//!
//! # Suspension points
//!
//! `EXISTS` and a SPARQL-bodied function call re-enter pattern evaluation. Their
//! instructions produce a [`Suspend`] (a custom call's probe returns it as a
//! [`VmStep::Suspend`]), which the interpreter resolves at once by calling the
//! evaluator; resolving a suspension is the one place that re-entry happens.

mod compile;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use purrdf_core::{DatasetView, TermValue};
use purrdf_sparql_algebra::{Expression, Function, GraphPattern};

pub(crate) use compile::ExprProgram;
use compile::{Cmp, Op, StrPred};

use crate::error::EvalError;
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
) -> Arc<ExprProgram> {
    if let Some(plan) = ctx.plan.as_ref()
        && let Some(id) = plan.node_of(node)
        && let Some(position) = attached_position(node, expr)
    {
        return plan.shape().program(id, position, expr);
    }
    Arc::new(ExprProgram::compile(expr))
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
    /// A string argument (lexical form and lower-cased tag), or absent.
    Str(Option<(String, Option<String>)>),
    /// The program's string constant at this index, read in place rather than copied.
    StrConst(u32),
    /// An `IN` in progress: the needle, its value, and whether a candidate raised.
    In {
        target: SolutionTerm<I>,
        value: TermValue,
        saw_error: bool,
    },
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
    /// The pattern and flags are constants, compiled at link time (`None`: they do not
    /// compile).
    Linked(Option<Arc<purrdf_core::xsd_regex::CompiledPattern>>),
}

impl RegexSlot {
    /// The link-time pattern, when there is one.
    const fn linked(&self) -> Option<&Option<Arc<purrdf_core::xsd_regex::CompiledPattern>>> {
        match self {
            Self::PerRow => None,
            Self::Linked(compiled) => Some(compiled),
        }
    }
}

/// What an instruction that may re-enter the evaluator produced.
pub(crate) enum VmStep<'e, 'd, I> {
    /// The instruction's value.
    Value(Option<SolutionTerm<I>>),
    /// The instruction needs pattern evaluation before it has a value.
    Suspend(Suspend<'e, 'd>),
}

/// A pattern evaluation an instruction is waiting on.
pub(crate) enum Suspend<'e, 'd> {
    /// `EXISTS` over `pattern`, for the current row.
    Exists(&'e GraphPattern),
    /// A SPARQL-bodied function call: the declaration, the prepared body, the call's
    /// entry in the program's call table, and the evaluated arguments.
    SparqlUdf {
        func: &'d crate::user_fn::UserFunction,
        body: &'d Arc<crate::engine::PreparedQuery>,
        call: u32,
        vals: Vec<Option<TermValue>>,
    },
}

/// A program linked to one operator call: its variable slots resolved to the call's
/// schema, its constant regexes compiled, its `EXISTS` bound to their patterns, and its
/// constant pool and value stack.
pub(crate) struct Linked<'e, I: Copy> {
    program: Arc<ExprProgram>,
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
    /// Each `EXISTS`'s pattern, in the expression's pre-order.
    exists: Vec<&'e GraphPattern>,
    stack: ValStack<I>,
    /// A function call's argument values, refilled by every call instruction, so a row
    /// that calls a function reuses one buffer instead of allocating its own.
    args: Vec<Option<TermValue>>,
}

impl<'e, I: Copy + PartialEq> Linked<'e, I> {
    /// Link `program`, compiled from `expr`, to a call over `schema`.
    pub(crate) fn link<D: DatasetView<Id = I> + Sync>(
        program: Arc<ExprProgram>,
        expr: &'e Expression,
        schema: &VarSchema,
        ctx: &mut EvalCtx<'_, D>,
    ) -> Self {
        let mut exists = Vec::new();
        if program.has_exists() {
            let mut pending = vec![expr];
            while let Some(expr) = pending.pop() {
                if let Expression::Exists(pattern) = expr {
                    exists.push(&**pattern);
                }
                compile::push_operands(expr, &mut pending);
            }
        }
        Self::link_parts(program, exists, schema, ctx)
    }

    /// Link a program whose expression is not at hand. An `EXISTS` in it has no
    /// pattern to test and fails the run as an internal error.
    pub(crate) fn link_without_exists<D: DatasetView<Id = I> + Sync>(
        program: Arc<ExprProgram>,
        schema: &VarSchema,
        ctx: &mut EvalCtx<'_, D>,
    ) -> Self {
        Self::link_parts(program, Vec::new(), schema, ctx)
    }

    fn link_parts<D: DatasetView<Id = I> + Sync>(
        program: Arc<ExprProgram>,
        exists: Vec<&'e GraphPattern>,
        schema: &VarSchema,
        ctx: &mut EvalCtx<'_, D>,
    ) -> Self {
        let slots = program
            .vars
            .iter()
            .map(|var| schema.index_of(var))
            .collect();
        let regexes = program
            .regexes
            .iter()
            .map(|constant| match constant {
                Some((pattern, flags)) => {
                    RegexSlot::Linked(helpers::cached_regex(ctx, pattern, flags))
                }
                None => RegexSlot::PerRow,
            })
            .collect();
        Self {
            consts: purrdf_core::smallvec![ConstCell::Unread; program.consts.len()],
            slots,
            regexes,
            exists,
            stack: Vec::new(),
            args: Vec::new(),
            program,
        }
    }

    /// A copy for a forked worker: the same slots, regexes and patterns, with a constant
    /// pool of its own, since the worker interns into its own scratch.
    pub(crate) fn fresh(&self) -> Self {
        Self {
            program: Arc::clone(&self.program),
            slots: self.slots.clone(),
            consts: purrdf_core::smallvec![ConstCell::Unread; self.consts.len()],
            regexes: self.regexes.clone(),
            exists: self.exists.clone(),
            stack: Vec::new(),
            args: Vec::new(),
        }
    }

    /// The expression's effective boolean value over `row` (`Ok(None)` is an error or
    /// unbound).
    pub(crate) fn ebv<D: DatasetView<Id = I> + Sync>(
        &mut self,
        row: &[Option<SolutionTerm<I>>],
        schema: &VarSchema,
        ctx: &mut EvalCtx<'_, D>,
    ) -> Result<Option<bool>, EvalError> {
        match self.term(row, schema, ctx)? {
            Some(term) => Ok(helpers::ebv_term(ctx, term)),
            None => Ok(None),
        }
    }

    /// The expression's value over `row`: `Ok(Some)` a term, `Ok(None)` an error or
    /// unbound, `Err` a hard failure. `schema` is the schema the program was linked to.
    pub(crate) fn term<D: DatasetView<Id = I> + Sync>(
        &mut self,
        row: &[Option<SolutionTerm<I>>],
        schema: &VarSchema,
        ctx: &mut EvalCtx<'_, D>,
    ) -> Result<Option<SolutionTerm<I>>, EvalError> {
        let Self {
            program,
            slots,
            consts,
            regexes,
            exists,
            stack,
            args,
        } = self;
        stack.clear();
        let ops = &program.ops;
        let mut pc = 0_usize;
        while let Some(op) = ops.get(pc) {
            pc += 1;
            match *op {
                Op::Const(k) => {
                    let k = k as usize;
                    let term = match consts[k] {
                        ConstCell::Read(term) => term,
                        ConstCell::Unread => {
                            let term = helpers::intern_leaf(ctx, program.consts[k].value());
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
                    stack.push(Val::Term(Some(helpers::bool_term(ctx, bound))));
                }
                Op::EbvOf => {
                    let term = pop_term(stack)?;
                    stack.push(Val::Ebv(term.and_then(|t| helpers::ebv_term(ctx, t))));
                }
                Op::Kleene { or, n } => {
                    let start = stack.len().checked_sub(n as usize).ok_or_else(underflow)?;
                    let mut value = Some(!or);
                    for entry in stack.drain(start..) {
                        let Val::Ebv(operand) = entry else {
                            return Err(mistyped("an effective boolean value"));
                        };
                        value = if or {
                            helpers::kleene_or(value, operand)
                        } else {
                            helpers::kleene_and(value, operand)
                        };
                    }
                    stack.push(Val::Term(value.map(|b| helpers::bool_term(ctx, b))));
                }
                Op::Not => {
                    let value = pop_ebv(stack)?;
                    stack.push(Val::Term(value.map(|b| helpers::bool_term(ctx, !b))));
                }
                Op::Equal => {
                    let b = pop_term(stack)?;
                    let a = pop_term(stack)?;
                    stack.push(Val::Term(helpers::equal_terms(ctx, a, b)));
                }
                Op::SameTerm => {
                    let b = pop_term(stack)?;
                    let a = pop_term(stack)?;
                    stack.push(Val::Term(match (a, b) {
                        (Some(x), Some(y)) => Some(helpers::bool_term(ctx, x == y)),
                        _ => None,
                    }));
                }
                Op::Cmp(cmp) => {
                    let b = pop_term(stack)?;
                    let a = pop_term(stack)?;
                    stack.push(Val::Term(compare(ctx, cmp, a, b)));
                }
                Op::Branch { on_false, end } => match pop_ebv(stack)? {
                    Some(true) => {}
                    Some(false) => pc = on_false as usize,
                    None => {
                        stack.push(Val::Term(None));
                        pc = end as usize;
                    }
                },
                Op::Jmp(target) => pc = target as usize,
                Op::CoalesceNext(end) => {
                    if matches!(stack.last(), Some(Val::Term(Some(_)))) {
                        pc = end as usize;
                    } else {
                        pop_term(stack)?;
                    }
                }
                Op::PushUnbound => stack.push(Val::Term(None)),
                Op::InNeedle(end) => match pop_term(stack)? {
                    None => {
                        stack.push(Val::Term(None));
                        pc = end as usize;
                    }
                    Some(target) => {
                        let value = helpers::value_of(ctx, target);
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
                        return Err(mistyped("an open IN"));
                    };
                    let matched = match candidate {
                        None => {
                            *saw_error = true;
                            false
                        }
                        Some(candidate) => {
                            match helpers::in_candidate(ctx, *target, value, candidate) {
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
                        stack.push(Val::Term(Some(helpers::bool_term(ctx, true))));
                        pc = end as usize;
                    }
                }
                Op::InEnd => {
                    let Some(Val::In { saw_error, .. }) = stack.pop() else {
                        return Err(mistyped("an open IN"));
                    };
                    stack.push(Val::Term(if saw_error {
                        None
                    } else {
                        Some(helpers::bool_term(ctx, false))
                    }));
                }
                Op::Arith(operator) => {
                    let right = pop_term(stack)?;
                    let left = pop_term(stack)?;
                    stack.push(Val::Term(match (left, right) {
                        (Some(a), Some(b)) => helpers::arithmetic_step(ctx, operator, a, b),
                        _ => None,
                    }));
                }
                Op::UnaryPlus => {
                    let operand = pop_term(stack)?;
                    stack.push(Val::Term(helpers::unary_numeric_term(
                        ctx,
                        operand,
                        purrdf_xsd::numeric_unary_plus,
                    )));
                }
                Op::UnaryMinus => {
                    let operand = pop_term(stack)?;
                    stack.push(Val::Term(helpers::unary_numeric_term(
                        ctx,
                        operand,
                        purrdf_xsd::value_unary_minus,
                    )));
                }
                Op::Exists(site) => {
                    let pattern = exists.get(site as usize).copied().ok_or_else(|| {
                        EvalError::internal(
                            "an EXISTS instruction was run by a link that holds no pattern \
                             for it",
                        )
                    })?;
                    let value = resolve(Suspend::Exists(pattern), program, row, schema, ctx)?;
                    stack.push(Val::Term(value));
                }
                Op::Call { call, argc, regex } => {
                    pop_values(stack, argc, ctx, args)?;
                    let compiled = regex.and_then(|slot| regexes[slot as usize].linked());
                    let value = helpers::apply_function(
                        &program.calls[call as usize],
                        args,
                        ctx,
                        compiled,
                    )?;
                    stack.push(Val::Term(value));
                }
                Op::CallCustom { call, argc } => {
                    pop_values(stack, argc, ctx, args)?;
                    let value = match call_custom(program, call, args, ctx)? {
                        VmStep::Value(value) => value,
                        VmStep::Suspend(suspend) => resolve(suspend, program, row, schema, ctx)?,
                    };
                    stack.push(Val::Term(value));
                }
                Op::StrConst(k) => {
                    stack.push(Val::StrConst(k));
                }
                Op::StrNone => stack.push(Val::Str(None)),
                Op::ToStrArg => {
                    let term = pop_term(stack)?;
                    stack.push(Val::Str(
                        term.and_then(|t| helpers::string_arg_of_term(ctx, t)),
                    ));
                }
                Op::ToStrLexical => {
                    let term = pop_term(stack)?;
                    stack.push(Val::Str(
                        term.and_then(|t| helpers::str_lexical_term(ctx, t))
                            .map(|s| (s, None)),
                    ));
                }
                Op::ToLangLexical => {
                    let term = pop_term(stack)?;
                    stack.push(Val::Str(
                        term.and_then(|t| helpers::lang_lexical_term(ctx, t))
                            .map(|s| (s, None)),
                    ));
                }
                Op::StrPred(pred) => {
                    let needle = pop_str(stack, &program.strs)?;
                    let haystack = pop_str(stack, &program.strs)?;
                    let value = match (haystack, needle) {
                        (Some(h), Some(n)) => {
                            let (h, n) = (h.0.as_str(), n.0.as_str());
                            let holds = match pred {
                                StrPred::Contains => h.contains(n),
                                StrPred::StrStarts => h.starts_with(n),
                                StrPred::StrEnds => h.ends_with(n),
                            };
                            Some(helpers::bool_term(ctx, holds))
                        }
                        _ => None,
                    };
                    stack.push(Val::Term(value));
                }
                Op::Regex(slot) => {
                    let flags = pop_str(stack, &program.strs)?;
                    let pattern = pop_str(stack, &program.strs)?;
                    let text = pop_str(stack, &program.strs)?;
                    let value = match (text, pattern) {
                        (Some(text), Some(pattern)) => {
                            let flags = flags.as_ref().map_or("", |f| f.0.as_str());
                            let compiled = match &regexes[slot as usize] {
                                RegexSlot::Linked(compiled) => compiled.clone(),
                                RegexSlot::PerRow => helpers::cached_regex(ctx, &pattern.0, flags),
                            };
                            compiled
                                .map(|re| helpers::bool_term(ctx, re.as_regex().is_match(&text.0)))
                        }
                        _ => None,
                    };
                    stack.push(Val::Term(value));
                }
                Op::LangMatches => {
                    let range = pop_str(stack, &program.strs)?;
                    let tag = pop_str(stack, &program.strs)?;
                    let value = match (tag, range) {
                        (Some(tag), Some(range)) => Some(helpers::bool_term(
                            ctx,
                            helpers::lang_matches(&tag.0, &range.0),
                        )),
                        _ => None,
                    };
                    stack.push(Val::Term(value));
                }
            }
        }
        let value = pop_term(stack)?;
        if !stack.is_empty() {
            return Err(EvalError::internal(
                "an expression program left more than its value on the stack",
            ));
        }
        Ok(value)
    }
}

/// The ordering comparison `cmp` over two evaluated operands.
fn compare<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    cmp: Cmp,
    a: Option<SolutionTerm<D::Id>>,
    b: Option<SolutionTerm<D::Id>>,
) -> Option<SolutionTerm<D::Id>> {
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
fn call_custom<'e, 'd, D: DatasetView + Sync>(
    program: &ExprProgram,
    call: u32,
    vals: &[Option<TermValue>],
    ctx: &mut EvalCtx<'d, D>,
) -> Result<VmStep<'e, 'd, D::Id>, EvalError> {
    let Function::Custom(iri) = &program.calls[call as usize] else {
        return Err(EvalError::internal(
            "a custom-call instruction names a built-in function",
        ));
    };
    let registry: &'d crate::user_fn::BoundFunctionRegistry = ctx.user_functions;
    if let Some((func, body)) = registry.resolve(iri.as_str()) {
        return Ok(VmStep::Suspend(Suspend::SparqlUdf {
            func,
            body,
            call,
            vals: vals.to_vec(),
        }));
    }
    helpers::apply_custom_host(iri.as_str(), vals, ctx).map(VmStep::Value)
}

/// Resolve a suspension by running the pattern evaluation it waits on, now.
fn resolve<D: DatasetView + Sync>(
    suspend: Suspend<'_, '_>,
    program: &ExprProgram,
    row: &[Option<SolutionTerm<D::Id>>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    match suspend {
        Suspend::Exists(pattern) => {
            let found = helpers::exists(pattern, row, schema, ctx)?;
            Ok(Some(helpers::bool_term(ctx, found)))
        }
        Suspend::SparqlUdf {
            func,
            body,
            call,
            vals,
        } => {
            let Function::Custom(iri) = &program.calls[call as usize] else {
                return Err(EvalError::internal(
                    "a suspended function call names a built-in function",
                ));
            };
            let result = crate::user_fn::eval_user_function(func, body, iri.as_str(), &vals, ctx)?;
            Ok(result.and_then(|value| helpers::intern(ctx, value)))
        }
    }
}

/// Pop `argc` terms and materialize their values into `vals`, in argument order,
/// replacing what it held.
fn pop_values<D: DatasetView + Sync>(
    stack: &mut ValStack<D::Id>,
    argc: u32,
    ctx: &EvalCtx<'_, D>,
    vals: &mut Vec<Option<TermValue>>,
) -> Result<(), EvalError> {
    let start = stack
        .len()
        .checked_sub(argc as usize)
        .ok_or_else(underflow)?;
    vals.clear();
    for entry in stack.drain(start..) {
        let Val::Term(term) = entry else {
            return Err(mistyped("a term"));
        };
        vals.push(term.map(|t| helpers::value_of(ctx, t)));
    }
    Ok(())
}

fn pop_term<I: Copy>(stack: &mut ValStack<I>) -> Result<Option<SolutionTerm<I>>, EvalError> {
    match stack.pop() {
        Some(Val::Term(term)) => Ok(term),
        Some(_) => Err(mistyped("a term")),
        None => Err(underflow()),
    }
}

fn pop_ebv<I: Copy>(stack: &mut ValStack<I>) -> Result<Option<bool>, EvalError> {
    match stack.pop() {
        Some(Val::Ebv(value)) => Ok(value),
        Some(_) => Err(mistyped("an effective boolean value")),
        None => Err(underflow()),
    }
}

/// A string argument as an instruction reads it: its lexical form and lower-cased tag,
/// owned when the program computed it and borrowed when it is one of the program's
/// constants.
type StrArg<'p> = std::borrow::Cow<'p, (String, Option<String>)>;

/// Pop a string argument: one the program computed, or one of its constants `strs`,
/// borrowed where it stands.
fn pop_str<'p, I: Copy>(
    stack: &mut ValStack<I>,
    strs: &'p [(String, Option<String>)],
) -> Result<Option<StrArg<'p>>, EvalError> {
    match stack.pop() {
        Some(Val::Str(value)) => Ok(value.map(std::borrow::Cow::Owned)),
        Some(Val::StrConst(k)) => Ok(Some(std::borrow::Cow::Borrowed(&strs[k as usize]))),
        Some(_) => Err(mistyped("a string argument")),
        None => Err(underflow()),
    }
}

fn underflow() -> EvalError {
    EvalError::internal("an expression program popped an empty stack")
}

fn mistyped(expected: &str) -> EvalError {
    EvalError::internal(format!(
        "an expression program found a stack entry that is not {expected}"
    ))
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

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The SEP-0009 **composite-datatype** functions (`cdt:List`, `cdt:get`,
//! `cdt:merge`, …) as executable SPARQL.
//!
//! `purrdf-cdt` owns the value layer — the lexical scanner, the canonical form,
//! the fifteen operations and the two comparison relations — and knows nothing
//! about the evaluator's term representation, by design: it is a closed leaf that
//! must not depend on `purrdf-core`. This module is the other half of that
//! contract, the **bridge**, and it is the only place in the workspace that
//! converts between [`TermValue`] and [`CdtTerm`].
//!
//! The parser has already resolved a call-position `cdt:` IRI to a
//! [`CdtFn`] and checked its argument count (see
//! [`purrdf_sparql_algebra::CdtCall`]), so [`dispatch`] is a total match over the
//! closed registry with no arity re-check and no unknown-function arm.
//!
//! # The tri-state is carried end to end
//!
//! [`purrdf_cdt::CdtOutcome`] has three states and this module keeps all three
//! apart, because collapsing any two of them changes query answers:
//!
//! * `Value` → `Ok(Some(term))`;
//! * `Error` → `Ok(None)`, a SPARQL **expression error** — the `BIND` leaves its
//!   variable unbound and a `FILTER` drops the row. This is what the corpus writes
//!   as `FILTER(!BOUND(?x))`, and it is emphatically not `false`;
//! * `Bound` → <code>Err([EvalError::CompositeBound])</code>, a hard failure of the whole
//!   query. Degrading a refused-because-too-large mint to an unbound variable would
//!   let a hostile query silently change a result set rather than be refused.
//!
//! # How a composite gets into and out of a solution
//!
//! A composite value lives in a solution as an ordinary
//! [`TermValue::Literal`] whose datatype is `cdt:List` or `cdt:Map`; there is no
//! side table and no new term kind. Two directions, and they are not symmetric:
//!
//! * **In** ([`to_cdt_term_admitted`]) — a literal the query *authored* keeps its lexical
//!   form byte for byte, so `"[  1 ,  2 ]"^^cdt:List` and `cdt:List(1,2)` stay
//!   different RDF terms (`list-functions/sameterm-04.rq`). Only when a
//!   `cdt:`-typed literal actually *parses* is it lifted to a
//!   [`CdtTerm::Composite`], so nesting a composite inside a composite costs one
//!   bracket level rather than a fresh round of string escaping.
//! * **Out** ([`from_cdt_term_admitted`]) — a value PurRDF *computed* is spelled in
//!   `purrdf-cdt`'s canonical form. That is what makes two independent evaluations
//!   of `cdt:List()` the SAME term (`sameterm-01.rq`), which they must be.
//!
//! The one place the two rules meet is `cdt:remove` on a key the map does not
//! hold: it returns the caller's ORIGINAL literal, not a re-rendered equal one,
//! because `map-functions/remove-01.rq` asserts the result with `SAMETERM`. That
//! is what [`purrdf_cdt::MapRemoval::Unchanged`] exists to say.
//!
//! # Blank nodes
//!
//! A blank node inside a composite is a real blank node, and two occurrences of one
//! label denote one node: `vectors/sparql-cdt/bnodes/bnodes-sparql-01.rq` binds
//! `"[_:b, 42, _:b]"^^cdt:List` and requires `cdt:get(?list,1)` and
//! `cdt:get(?list,3)` to be `=`. The label is carried through
//! [`purrdf_core::BlankScope::qualify_label`] / [`purrdf_core::BlankScope::unqualify_label`] — the kernel's
//! existing `(label, scope)` encoding, not a second scoping scheme — so the round
//! trip is the identity on every `(label, scope)` pair and a `BNODE()` put into a
//! list comes back out `sameTerm` with itself (`list-constructor-16.rq`).
//!
//! # Everything is iterative
//!
//! A composite is a tree over attacker-controlled lexical input and a stack
//! overflow in Rust is an `abort` no caller can catch, so both conversions walk
//! with an explicit heap worklist and neither recurses — the same discipline
//! `purrdf-cdt` holds itself to.

use purrdf_cdt::{
    CDT_LIST, CDT_MAP, CdtDatatype, CdtError, CdtLiteral, CdtOutcome, CdtTerm, CdtTripleTerm,
    CdtValue, MapRemoval,
};
use purrdf_core::{DatasetView, TermValue};
use purrdf_sparql_algebra::CdtFn;

use crate::error::EvalError;
use crate::eval::EvalCtx;
use crate::expr::{intern_boolean, intern_integer};
use crate::scratch::SolutionTerm;

/// Evaluate a SEP-0009 composite-datatype function call.
///
/// `vals` holds each argument already evaluated, with `None` for an argument that
/// was unbound or whose own evaluation raised. Which of those two an argument is
/// does not matter to any function here — SEP-0009 treats them identically — but
/// *where* the argument sits does: a failed constructor argument becomes the
/// `null` element (`list-functions/list-constructor-null-01.rq`), while a failed
/// argument anywhere else raises.
///
/// The result follows the module's tri-state contract: `Ok(Some)` is a value,
/// `Ok(None)` is a SPARQL expression error, and `Err` is a hard failure.
pub(crate) fn dispatch<D: DatasetView + Sync>(
    func: CdtFn,
    vals: &[Option<TermValue>],
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    use crate::workspace::{AdmittedVec, LexicalFrame};
    let workspace = ctx.growth.clone();
    let mut input_frame = LexicalFrame::new(&workspace);
    let mut owners = AdmittedVec::new(&workspace);
    let mut input = Vec::new();
    if matches!(func, CdtFn::ListConstructor | CdtFn::MapConstructor) {
        for value in vals {
            let term = match value {
                Some(value) => to_cdt_term_admitted(value, true, &workspace)?,
                None => OwnedCdtTerm {
                    value: CdtTerm::Null,
                    _frame: LexicalFrame::new(&workspace),
                },
            };
            owners.push(term)?;
            let value = std::mem::replace(
                &mut owners
                    .as_mut_slice()
                    .last_mut()
                    .expect("constructor owner was just inserted")
                    .value,
                CdtTerm::Null,
            );
            let live = input_frame.admitted_bytes();
            purrdf_lex::allocation::Memory::resume(&mut input_frame, live)
                .push(&mut input, value)
                .map_err(|error| {
                    storage_error(&mut input_frame, error, "composite constructor arguments")
                })?;
        }
        if func == CdtFn::ListConstructor {
            return kernel_value(ctx, |storage| {
                purrdf_cdt::functions::list_constructor_admitted(input, storage)
            });
        }
        let mut pairs = Vec::new();
        let mut input = input.into_iter();
        while let (Some(key), Some(value)) = (input.next(), input.next()) {
            let live = input_frame.admitted_bytes();
            purrdf_lex::allocation::Memory::resume(&mut input_frame, live)
                .push(&mut pairs, (key, value))
                .map_err(|error| {
                    storage_error(&mut input_frame, error, "composite map constructor pairs")
                })?;
        }
        return kernel_value(ctx, |storage| {
            purrdf_cdt::functions::map_constructor_admitted(&pairs, storage)
        });
    }
    if matches!(func, CdtFn::Concat | CdtFn::Merge) {
        let mut values = Vec::new();
        for value in vals {
            let Some(value) = value.as_ref() else {
                return Ok(None);
            };
            let Some(value) = composite_argument_admitted(value, &workspace)? else {
                return Ok(None);
            };
            let (value, _) = value
                .clone_admitted(&mut CdtStorage::new(&mut input_frame))
                .map_err(|error| {
                    storage_error(&mut input_frame, error, "composite variadic argument")
                })?;
            let live = input_frame.admitted_bytes();
            purrdf_lex::allocation::Memory::resume(&mut input_frame, live)
                .push(&mut values, value)
                .map_err(|error| {
                    storage_error(&mut input_frame, error, "composite variadic argument array")
                })?;
        }
        return kernel_value(ctx, |storage| match func {
            CdtFn::Concat => purrdf_cdt::functions::concat_admitted(&values, storage),
            _ => purrdf_cdt::functions::merge_admitted(&values, storage),
        });
    }
    let Some(first) = vals.first().and_then(Option::as_ref) else {
        return Ok(None);
    };
    let Some(value) = composite_argument_admitted(first, &workspace)? else {
        return Ok(None);
    };
    match func {
        CdtFn::Size => return intern_integer(ctx, purrdf_cdt::size(&value) as u64).map(Some),
        CdtFn::Head => {
            return kernel_term(ctx, |storage| {
                purrdf_cdt::functions::head_admitted(&value, storage)
            });
        }
        CdtFn::Tail => {
            return kernel_value(ctx, |storage| {
                purrdf_cdt::functions::tail_admitted(&value, storage)
            });
        }
        CdtFn::Reverse => {
            return kernel_value(ctx, |storage| {
                purrdf_cdt::functions::reverse_admitted(&value, storage)
            });
        }
        CdtFn::Keys => {
            return kernel_value(ctx, |storage| {
                purrdf_cdt::functions::keys_admitted(&value, storage)
            });
        }
        _ => {}
    }
    let Some(second) = vals.get(1).and_then(Option::as_ref) else {
        return Ok(None);
    };
    let key = to_cdt_term_admitted(second, true, &workspace)?;
    match func {
        CdtFn::Get => kernel_term(ctx, |storage| {
            purrdf_cdt::functions::get_admitted(&value, &key.value, storage)
        }),
        CdtFn::Contains => kernel_bool(ctx, |storage| {
            purrdf_cdt::functions::contains_admitted(&value, &key.value, storage)
        }),
        CdtFn::ContainsKey => kernel_bool(ctx, |storage| {
            purrdf_cdt::functions::contains_key_admitted(&value, &key.value, storage)
        }),
        CdtFn::Subseq => {
            let length = match vals.get(2) {
                None => None,
                Some(None) => return Ok(None),
                Some(Some(value)) => Some(to_cdt_term_admitted(value, true, &workspace)?),
            };
            kernel_value(ctx, |storage| {
                purrdf_cdt::functions::subseq_admitted(
                    &value,
                    &key.value,
                    length.as_ref().map(|value| &value.value),
                    storage,
                )
            })
        }
        CdtFn::Put => {
            let item = match vals.get(2).and_then(Option::as_ref) {
                Some(value) => to_cdt_term_admitted(value, true, &workspace)?,
                None => OwnedCdtTerm {
                    value: CdtTerm::Null,
                    _frame: LexicalFrame::new(&workspace),
                },
            };
            kernel_value(ctx, |storage| {
                purrdf_cdt::functions::put_admitted(&value, &key.value, &item.value, storage)
            })
        }
        CdtFn::Remove => {
            let mut frame = LexicalFrame::new(&workspace);
            let (outcome, _) = purrdf_cdt::functions::remove_admitted(
                &value,
                &key.value,
                &mut CdtStorage::new(&mut frame),
            )
            .map_err(|error| storage_error(&mut frame, error, "composite removal"))?;
            match outcome {
                CdtOutcome::Value(MapRemoval::Removed(value)) => {
                    let term = composite_literal_admitted(&value, &workspace)?;
                    ctx.intern_workspace_term(term)
                }
                CdtOutcome::Value(MapRemoval::Unchanged) => {
                    let original = workspace.clone_term(first)?;
                    ctx.intern_workspace_term(original)
                }
                CdtOutcome::Error(_) => Ok(None),
                CdtOutcome::Bound(error) => Err(bound_admitted(&error, &workspace)),
            }
        }
        CdtFn::ListConstructor
        | CdtFn::MapConstructor
        | CdtFn::Concat
        | CdtFn::Merge
        | CdtFn::Size
        | CdtFn::Head
        | CdtFn::Tail
        | CdtFn::Reverse
        | CdtFn::Keys => {
            unreachable!("unary and variadic functions returned before strict arguments")
        }
    }
}

fn composite_argument_admitted(
    value: &TermValue,
    workspace: &crate::WorkspaceCapability,
) -> Result<Option<crate::composite_value::CompositeValue>, EvalError> {
    let TermValue::Literal {
        lexical_form,
        datatype,
        language: None,
        ..
    } = value
    else {
        return Ok(None);
    };
    crate::composite_value::CompositeValue::parse(lexical_form, datatype, workspace)
}

fn kernel_value<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    call: impl FnOnce(
        &mut CdtStorage<'_>,
    ) -> Result<(CdtOutcome<CdtValue>, usize), purrdf_cdt::memory::StorageError>,
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    let workspace = ctx.growth.clone();
    let mut frame = crate::workspace::LexicalFrame::new(&workspace);
    let (outcome, _) = call(&mut CdtStorage::new(&mut frame))
        .map_err(|error| storage_error(&mut frame, error, "composite function output"))?;
    match outcome {
        CdtOutcome::Value(value) => {
            let output = composite_literal_admitted(&value, &workspace)?;
            ctx.intern_workspace_term(output)
        }
        CdtOutcome::Error(_) => Ok(None),
        CdtOutcome::Bound(error) => Err(bound_admitted(&error, &workspace)),
    }
}

fn kernel_term<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    call: impl FnOnce(
        &mut CdtStorage<'_>,
    ) -> Result<(CdtOutcome<CdtTerm>, usize), purrdf_cdt::memory::StorageError>,
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    let workspace = ctx.growth.clone();
    let mut frame = crate::workspace::LexicalFrame::new(&workspace);
    let (outcome, _) = call(&mut CdtStorage::new(&mut frame))
        .map_err(|error| storage_error(&mut frame, error, "composite element output"))?;
    match outcome {
        CdtOutcome::Value(value) => from_cdt_term_admitted(&value, &workspace)?
            .map(|term| ctx.intern_workspace_term(term))
            .transpose()
            .map(Option::flatten),
        CdtOutcome::Error(_) => Ok(None),
        CdtOutcome::Bound(error) => Err(bound_admitted(&error, &workspace)),
    }
}

fn kernel_bool<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    call: impl FnOnce(
        &mut CdtStorage<'_>,
    ) -> Result<(CdtOutcome<bool>, usize), purrdf_cdt::memory::StorageError>,
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    let workspace = ctx.growth.clone();
    let mut frame = crate::workspace::LexicalFrame::new(&workspace);
    let (outcome, _) = call(&mut CdtStorage::new(&mut frame))
        .map_err(|error| storage_error(&mut frame, error, "composite membership"))?;
    match outcome {
        CdtOutcome::Value(value) => intern_boolean(ctx, value).map(Some),
        CdtOutcome::Error(_) => Ok(None),
        CdtOutcome::Bound(error) => Err(bound_admitted(&error, &workspace)),
    }
}

/// Whether a term is a `cdt:List` / `cdt:Map`-typed literal — **regardless of
/// whether its lexical form parses**.
///
/// This is the gate the comparison operators use, and the ill-formed case is
/// exactly why it is not `as_composite(..).is_some()`: `"1"^^cdt:List` denotes
/// nothing, so `list-functions/list-less-than-error-03.rq` requires a comparison
/// with it to RAISE. Routing it to the ordinary XSD path instead would answer
/// "two literals that cannot be value-compared", which happens to be the same
/// unbound result here but is a different judgement and does not stay the same
/// under `=` (`literal_equal` reports an ill-typed operand as an error whatever
/// the other side is).
pub(crate) fn is_composite_typed(value: &TermValue) -> bool {
    matches!(
        value,
        TermValue::Literal {
            datatype,
            language: None,
            ..
        } if CdtDatatype::from_iri(datatype).is_some()
    )
}

// ---------------------------------------------------------------------------
// the bridge: TermValue <-> CdtTerm
// ---------------------------------------------------------------------------

/// One step of the iterative [`to_cdt_term_admitted`] walk.
enum InJob<'a> {
    /// Convert this term and push the result.
    Visit(&'a TermValue),
    /// Pop three converted components and combine them into a triple term.
    Triple,
}

/// One step of the iterative [`from_cdt_term_admitted`] walk.
enum OutJob<'a> {
    /// Convert this element and push the result.
    Visit(&'a CdtTerm),
    /// Pop three converted components and combine them into a triple term.
    Triple,
}

// ---------------------------------------------------------------------------
// comparison — SEP-0009 `=` and `<` over composite-typed operands
// ---------------------------------------------------------------------------

/// Which comparison a caller is asking [`compare_admitted`] for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum CdtRelation {
    /// SPARQL `=`.
    Equal,
    /// SPARQL `<`.
    Less,
    /// SPARQL `<=`.
    LessOrEqual,
    /// SPARQL `>`.
    Greater,
    /// SPARQL `>=`.
    GreaterOrEqual,
}

/// Compare two terms under SEP-0009's relations, where at least one of them is a
/// `cdt:List` / `cdt:Map`-typed literal. `None` is a SPARQL type error.
///
/// # `<=` is not `(a < b) || (a = b)`
///
/// `list-functions/list-less-equal-28.rq` requires `"[_:b]"^^cdt:List <=
/// "[_:b]"^^cdt:List` to be **unbound**, while `list-equals-07.rq` requires the
/// very same operands to be `=`-equal. Under SPARQL's `||`, `error || true` is
/// `true`, so the disjunction would answer `true` where the corpus demands an
/// error. The rule is therefore sequential: if `<` raised, raise; if `<` was true,
/// true; otherwise ask `=`. `list-greater-equal-28.rq` says the same for `>=`.
///
/// # Total, and it cannot fail
///
/// Every refusal is a `None` — a comparison with no answer is ordinary SPARQL
/// three-valued logic, not a failure of the query — which is what lets the
/// evaluator's `=` / `<` / `IN` paths all route through here uniformly.
#[cfg(test)]
pub(crate) fn compare(relation: CdtRelation, left: &TermValue, right: &TermValue) -> Option<bool> {
    compare_admitted(
        relation,
        left,
        right,
        &crate::WorkspaceCapability::resident(),
    )
    .expect("resident composite comparison")
}

pub(crate) fn compare_admitted(
    relation: CdtRelation,
    left: &TermValue,
    right: &TermValue,
    workspace: &crate::WorkspaceCapability,
) -> Result<Option<bool>, EvalError> {
    let left = to_cdt_term_admitted(left, false, workspace)?;
    let right = to_cdt_term_admitted(right, false, workspace)?;
    let (first, second) = match relation {
        CdtRelation::Greater | CdtRelation::GreaterOrEqual => (&right.value, &left.value),
        _ => (&left.value, &right.value),
    };
    let mut frame = crate::workspace::LexicalFrame::new(workspace);
    let result = match relation {
        CdtRelation::Equal => {
            purrdf_cdt::ops::try_term_equal(first, second, &mut CdtStorage::new(&mut frame))
        }
        CdtRelation::Less | CdtRelation::Greater => {
            purrdf_cdt::ops::try_term_less_than(first, second, &mut CdtStorage::new(&mut frame))
        }
        CdtRelation::LessOrEqual | CdtRelation::GreaterOrEqual => {
            match purrdf_cdt::ops::try_term_less_than(
                first,
                second,
                &mut CdtStorage::new(&mut frame),
            ) {
                Ok(Ok(false)) => {
                    purrdf_cdt::ops::try_term_equal(first, second, &mut CdtStorage::new(&mut frame))
                }
                other => other,
            }
        }
    };
    result
        .map(Result::ok)
        .map_err(|error| storage_error(&mut frame, error, "composite comparison"))
}

// ---------------------------------------------------------------------------
// interning helpers
// ---------------------------------------------------------------------------

/// A nested CDT kernel updates the same original grant while the enclosing
/// converted arguments remain live.
pub(crate) struct CdtStorage<'a> {
    frame: &'a mut crate::workspace::LexicalFrame,
    base: usize,
}

/// Native kernels return the original physical error even when their subsequent
/// refund fails. Only admission errors need the original frame's typed cause;
/// an explicit allocator/layout error must not become cleanup-only refusal.
pub(crate) fn storage_error(
    frame: &mut crate::workspace::LexicalFrame,
    error: purrdf_cdt::memory::StorageError,
    construct: &'static str,
) -> EvalError {
    if let Some(first) = frame.take_failure() {
        match (error, first) {
            (purrdf_cdt::memory::StorageError::AdmissionFailed, first) => return first,
            (
                purrdf_cdt::memory::StorageError::AllocationFailed,
                first @ EvalError::AllocationFailed { .. },
            ) => return first,
            (
                purrdf_cdt::memory::StorageError::SizeOverflow,
                first @ EvalError::WorkspaceBoundOverflow,
            ) => return first,
            (
                purrdf_cdt::memory::StorageError::FormattingFailed,
                first @ EvalError::UnstableNativeDiagnostic,
            ) => return first,
            _ => {}
        }
    }
    frame.storage_error(error, construct)
}

pub(crate) struct OwnedCdtTerm {
    pub(crate) value: CdtTerm,
    pub(crate) _frame: crate::workspace::LexicalFrame,
}

/// Construct a bridge operand with its original lexical and child-box owner.
pub(crate) fn to_cdt_term_admitted(
    value: &TermValue,
    lift: bool,
    workspace: &crate::WorkspaceCapability,
) -> Result<OwnedCdtTerm, EvalError> {
    use crate::workspace::{AdmittedVec, LexicalFrame};
    let mut frame = LexicalFrame::new(workspace);
    let mut jobs = AdmittedVec::new(workspace);
    let mut done = AdmittedVec::new(workspace);
    jobs.push(InJob::Visit(value))?;
    while let Some(job) = jobs.pop() {
        let value = match job {
            InJob::Triple => {
                let object = done.pop().ok_or(EvalError::WorkspaceBoundOverflow)?;
                let predicate = done.pop().ok_or(EvalError::WorkspaceBoundOverflow)?;
                let subject = done.pop().ok_or(EvalError::WorkspaceBoundOverflow)?;
                if lift {
                    match CdtTerm::triple_admitted(
                        subject,
                        predicate,
                        object,
                        &mut CdtStorage::new(&mut frame),
                    ) {
                        Ok((value, _)) => value,
                        Err(purrdf_cdt::memory::ReadError::Storage(error)) => {
                            return Err(storage_error(
                                &mut frame,
                                error,
                                "composite triple element",
                            ));
                        }
                        Err(purrdf_cdt::memory::ReadError::Lexical(error)) => {
                            return Err(bound_admitted(&error, workspace));
                        }
                    }
                } else {
                    let live = frame.admitted_bytes();
                    let boxed = purrdf_lex::allocation::Memory::resume(&mut frame, live).boxed(
                        CdtTripleTerm {
                            subject,
                            predicate,
                            object,
                        },
                    );
                    CdtTerm::TripleTerm(boxed.map_err(|error| {
                        storage_error(&mut frame, error, "composite comparison triple")
                    })?)
                }
            }
            InJob::Visit(TermValue::Triple { s, p, o }) => {
                jobs.push(InJob::Triple)?;
                jobs.push(InJob::Visit(o))?;
                jobs.push(InJob::Visit(p))?;
                jobs.push(InJob::Visit(s))?;
                continue;
            }
            InJob::Visit(TermValue::Iri(iri)) => {
                let live = frame.admitted_bytes();
                let copied = purrdf_lex::allocation::Memory::resume(&mut frame, live).string(iri);
                CdtTerm::Iri(
                    copied.map_err(|error| {
                        storage_error(&mut frame, error, "composite IRI operand")
                    })?,
                )
            }
            InJob::Visit(TermValue::Blank { label, scope }) => {
                let result = (|| {
                    let live = frame.admitted_bytes();
                    let mut memory = purrdf_lex::allocation::Memory::resume(&mut frame, live);
                    let label = purrdf_core::blank_label::encode_blank_label_with_memory(
                        label,
                        *scope,
                        purrdf_core::blank_label::LabelAlphabet::Unconstrained,
                        &mut memory,
                    )?;
                    match label {
                        std::borrow::Cow::Owned(label) => Ok(label),
                        std::borrow::Cow::Borrowed(label) => memory.string(label),
                    }
                })();
                CdtTerm::Blank(
                    result.map_err(|error| {
                        storage_error(&mut frame, error, "composite blank operand")
                    })?,
                )
            }
            InJob::Visit(TermValue::Literal {
                lexical_form,
                datatype,
                language,
                direction,
            }) => {
                if lift
                    && language.is_none()
                    && let Some(value) = crate::composite_value::CompositeValue::parse(
                        lexical_form,
                        datatype,
                        workspace,
                    )?
                {
                    let (value, _) = value
                        .clone_admitted(&mut CdtStorage::new(&mut frame))
                        .map_err(|error| {
                            storage_error(&mut frame, error, "composite argument copy")
                        })?;
                    match CdtTerm::composite_admitted(value, &mut CdtStorage::new(&mut frame)) {
                        Ok((value, _)) => {
                            done.push(value)?;
                            continue;
                        }
                        Err(purrdf_cdt::memory::ReadError::Storage(error)) => {
                            return Err(storage_error(&mut frame, error, "composite element box"));
                        }
                        Err(purrdf_cdt::memory::ReadError::Lexical(error)) => {
                            return Err(bound_admitted(&error, workspace));
                        }
                    }
                }
                let result = (|| {
                    let live = frame.admitted_bytes();
                    let mut memory = purrdf_lex::allocation::Memory::resume(&mut frame, live);
                    Ok::<_, purrdf_cdt::memory::StorageError>(CdtLiteral {
                        lexical: memory.string(lexical_form)?,
                        datatype: memory.string(datatype)?,
                        language: language
                            .as_deref()
                            .map(|language| memory.string(language))
                            .transpose()?,
                        direction: *direction,
                    })
                })();
                CdtTerm::Literal(result.map_err(|error| {
                    storage_error(&mut frame, error, "composite literal operand")
                })?)
            }
        };
        done.push(value)?;
    }
    Ok(OwnedCdtTerm {
        value: done.pop().ok_or(EvalError::WorkspaceBoundOverflow)?,
        _frame: frame,
    })
}

pub(crate) fn bound_admitted(
    error: &CdtError,
    workspace: &crate::WorkspaceCapability,
) -> EvalError {
    crate::error::NativeDiagnostic::error(
        crate::error::NativeDiagnosticKind::CompositeBound,
        error,
        workspace,
    )
}
impl<'a> CdtStorage<'a> {
    pub(crate) fn new(frame: &'a mut crate::workspace::LexicalFrame) -> Self {
        let base = frame.admitted_bytes();
        Self { frame, base }
    }
    fn boxed<T>(
        &mut self,
        value: T,
        construct: &'static str,
    ) -> Result<Box<T>, purrdf_cdt::memory::StorageError> {
        purrdf_lex::allocation::try_boxed(value).map_err(|_| {
            self.frame
                .latch_failure(EvalError::AllocationFailed { construct });
            purrdf_cdt::memory::StorageError::AllocationFailed
        })
    }
}
impl purrdf_cdt::memory::Admission for CdtStorage<'_> {
    fn resize(&mut self, bytes: usize) -> Result<(), purrdf_cdt::memory::StorageError> {
        let total = self
            .base
            .checked_add(bytes)
            .ok_or(purrdf_cdt::memory::StorageError::SizeOverflow)?;
        purrdf_lex::allocation::Admission::resize(self.frame, total)
    }
}
impl purrdf_cdt::memory::Storage for CdtStorage<'_> {
    fn boxed_value(
        &mut self,
        value: CdtValue,
    ) -> Result<Box<CdtValue>, purrdf_cdt::memory::StorageError> {
        self.boxed(value, "native composite value box")
    }
    fn boxed_triple(
        &mut self,
        value: CdtTripleTerm,
    ) -> Result<Box<CdtTripleTerm>, purrdf_cdt::memory::StorageError> {
        self.boxed(value, "native composite triple box")
    }
}

pub(crate) fn composite_literal_admitted(
    value: &CdtValue,
    workspace: &crate::WorkspaceCapability,
) -> Result<crate::WorkspaceTerm, EvalError> {
    let mut frame = crate::workspace::LexicalFrame::new(workspace);
    let rendered =
        purrdf_cdt::render::try_canonical_lexical(value, &mut CdtStorage::new(&mut frame));
    let (lexical_form, _) =
        rendered.map_err(|error| storage_error(&mut frame, error, "composite lexical output"))?;
    let result = (|| {
        let mut memory =
            purrdf_lex::allocation::Memory::resume(&mut frame, lexical_form.capacity());
        let datatype = memory.string(match value.datatype() {
            CdtDatatype::List => CDT_LIST,
            CdtDatatype::Map => CDT_MAP,
        })?;
        Ok::<_, purrdf_cdt::memory::StorageError>(TermValue::Literal {
            lexical_form,
            datatype,
            language: None,
            direction: None,
        })
    })();
    let term =
        result.map_err(|error| storage_error(&mut frame, error, "composite literal datatype"))?;
    frame.finish_term(|| term)
}

pub(crate) fn from_cdt_term_admitted(
    term: &CdtTerm,
    workspace: &crate::WorkspaceCapability,
) -> Result<Option<crate::WorkspaceTerm>, EvalError> {
    use crate::workspace::AdmittedVec;
    let mut jobs = AdmittedVec::new(workspace);
    let mut done = AdmittedVec::new(workspace);
    jobs.push(OutJob::Visit(term))?;
    while let Some(job) = jobs.pop() {
        match job {
            OutJob::Triple => {
                let object = done.pop().ok_or(EvalError::WorkspaceBoundOverflow)?;
                let predicate = done.pop().ok_or(EvalError::WorkspaceBoundOverflow)?;
                let subject = done.pop().ok_or(EvalError::WorkspaceBoundOverflow)?;
                done.push(crate::WorkspaceTerm::triple(
                    subject, predicate, object, workspace,
                )?)?;
            }
            OutJob::Visit(CdtTerm::Null) => return Ok(None),
            OutJob::Visit(CdtTerm::Iri(iri)) => done.push(workspace.iri(iri)?)?,
            OutJob::Visit(CdtTerm::Blank(qualified)) => {
                let mut frame = crate::workspace::LexicalFrame::new(workspace);
                let decoded = purrdf_core::blank_label::decode_blank_label_with_memory(
                    qualified,
                    purrdf_core::blank_label::LabelAlphabet::Unconstrained,
                    &mut purrdf_lex::allocation::Memory::new(&mut frame),
                );
                let (label, scope) = decoded.map_err(|error| {
                    storage_error(&mut frame, error, "composite blank decoding")
                })?;
                let result = workspace.blank(&label, scope)?;
                drop(label);
                done.push(result)?;
            }
            OutJob::Visit(CdtTerm::Literal(literal)) => {
                done.push(cdt_literal_admitted(literal, workspace)?)?;
            }
            OutJob::Visit(CdtTerm::Composite(value)) => {
                done.push(composite_literal_admitted(value, workspace)?)?;
            }
            OutJob::Visit(CdtTerm::TripleTerm(triple)) => {
                jobs.push(OutJob::Triple)?;
                jobs.push(OutJob::Visit(&triple.object))?;
                jobs.push(OutJob::Visit(&triple.predicate))?;
                jobs.push(OutJob::Visit(&triple.subject))?;
            }
        }
    }
    Ok(done.pop())
}

pub(crate) fn cdt_literal_admitted(
    literal: &CdtLiteral,
    workspace: &crate::WorkspaceCapability,
) -> Result<crate::WorkspaceTerm, EvalError> {
    let mut frame = crate::workspace::LexicalFrame::new(workspace);
    let result = (|| {
        let mut memory = purrdf_lex::allocation::Memory::new(&mut frame);
        Ok::<_, purrdf_cdt::memory::StorageError>(TermValue::Literal {
            lexical_form: memory.string(&literal.lexical)?,
            datatype: memory.string(&literal.datatype)?,
            language: literal
                .language
                .as_deref()
                .map(|language| memory.string(language))
                .transpose()?,
            direction: literal.direction,
        })
    })();
    let term =
        result.map_err(|error| storage_error(&mut frame, error, "composite literal output"))?;
    frame.finish_term(|| term)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermValue};
    use purrdf_sparql_algebra::SparqlParser;

    use crate::error::EvalError;
    use crate::eval::{EvalCtx, Outcome, evaluate_query};

    /// The SEP-0009 prologue every case below is written under. The namespace is
    /// the spec's own, fixed string — recognized, never minted.
    const PREFIX: &str = "PREFIX cdt: <http://w3id.org/awslabs/neptune/SPARQL-CDTs/> \
                          PREFIX xsd: <http://www.w3.org/2001/XMLSchema#> ";

    /// An empty dataset. Every SEP-0009 list/map conformance case runs against
    /// `empty.ttl`: composite values live in literals, so none of this needs data.
    fn empty() -> Arc<RdfDataset> {
        RdfDatasetBuilder::new().freeze().expect("freeze")
    }

    /// Evaluate an `ASK` over the empty dataset.
    fn ask(body: &str) -> bool {
        let query = format!("{PREFIX} ASK {{ {body} }}");
        let parsed = SparqlParser::new().parse_query(&query).expect("parse");
        let dataset = empty();
        let mut ctx = EvalCtx::new(&dataset);
        match evaluate_query(&parsed, &mut ctx).expect("eval") {
            Outcome::Boolean(answer) => answer,
            other => panic!("expected a boolean, got {other:?}"),
        }
    }

    /// Evaluate an `ASK` expected to hard-fail, returning the error.
    fn ask_err(body: &str) -> EvalError {
        let query = format!("{PREFIX} ASK {{ {body} }}");
        let parsed = SparqlParser::new().parse_query(&query).expect("parse");
        let dataset = empty();
        let mut ctx = EvalCtx::new(&dataset);
        evaluate_query(&parsed, &mut ctx).expect_err("expected a hard failure")
    }

    /// The single projected cell of a one-row `SELECT`, or `None` when unbound.
    fn select_one(body: &str) -> Option<TermValue> {
        let query = format!("{PREFIX} SELECT ?x WHERE {{ {body} }}");
        let parsed = SparqlParser::new().parse_query(&query).expect("parse");
        let dataset = empty();
        let mut ctx = EvalCtx::new(&dataset);
        match evaluate_query(&parsed, &mut ctx).expect("eval") {
            Outcome::Solutions(sequence) => {
                assert_eq!(sequence.rows.len(), 1, "expected exactly one row");
                sequence.rows[0][0].map(|term| ctx.scratch.value_of(ctx.dataset, term))
            }
            other => panic!("expected solutions, got {other:?}"),
        }
    }

    // ── the fifteen functions are reachable end to end ────────────────────────

    #[test]
    fn every_registry_member_evaluates_through_the_engine() {
        // The parse-time registry is closed and the evaluator's dispatch is total
        // over it; this walks the whole registry so a future member cannot be added
        // to `purrdf-cdt` and silently left unreachable from SPARQL.
        let cases: &[(purrdf_cdt::CdtFn, &str, &str)] = &[
            (
                purrdf_cdt::CdtFn::ListConstructor,
                "cdt:List(1, 2)",
                "[1,2]",
            ),
            (
                purrdf_cdt::CdtFn::MapConstructor,
                "cdt:Map(1, 2)",
                "{ 1 : 2 }",
            ),
            (
                purrdf_cdt::CdtFn::Concat,
                "cdt:concat(\"[1]\"^^cdt:List, \"[2]\"^^cdt:List)",
                "[1,2]",
            ),
            (
                purrdf_cdt::CdtFn::Tail,
                "cdt:tail(\"[1,2]\"^^cdt:List)",
                "[2]",
            ),
            (
                purrdf_cdt::CdtFn::Reverse,
                "cdt:reverse(\"[1,2]\"^^cdt:List)",
                "[2,1]",
            ),
            (
                purrdf_cdt::CdtFn::Subseq,
                "cdt:subseq(\"[1,2,3]\"^^cdt:List, 2, 2)",
                "[2,3]",
            ),
            (
                purrdf_cdt::CdtFn::Keys,
                "cdt:keys(\"{1:'a', 2:'b'}\"^^cdt:Map)",
                "[1,2]",
            ),
            (
                purrdf_cdt::CdtFn::Merge,
                "cdt:merge(\"{1:'a'}\"^^cdt:Map, \"{2:'b'}\"^^cdt:Map)",
                "{1:'a', 2:'b'}",
            ),
            (
                purrdf_cdt::CdtFn::Put,
                "cdt:put(\"{1:'a'}\"^^cdt:Map, 2, 'b')",
                "{1:'a', 2:'b'}",
            ),
            (
                purrdf_cdt::CdtFn::Remove,
                "cdt:remove(\"{1:'a', 2:'b'}\"^^cdt:Map, 2)",
                "{1:'a'}",
            ),
        ];
        for (fn_kind, call, expected) in cases {
            let datatype = if expected.starts_with('[') {
                "cdt:List"
            } else {
                "cdt:Map"
            };
            assert!(
                ask(&format!("FILTER({call} = \"{expected}\"^^{datatype})")),
                "{fn_kind:?}: {call} should equal {expected}"
            );
        }
        // The five whose result is not a composite, checked in their own shapes.
        assert!(ask("FILTER(cdt:size(\"[1,2,3]\"^^cdt:List) = 3)"));
        assert!(ask("FILTER(cdt:get(\"[1,2]\"^^cdt:List, 2) = 2)"));
        assert!(ask("FILTER(cdt:head(\"[7,8]\"^^cdt:List) = 7)"));
        assert!(ask("FILTER(cdt:contains(\"[1,2]\"^^cdt:List, 2))"));
        assert!(ask("FILTER(cdt:containsKey(\"{1:'a'}\"^^cdt:Map, 1))"));
    }

    // ── constructors mint a canonical, deterministic lexical form ─────────────

    #[test]
    fn two_evaluations_of_one_constructor_are_the_same_term() {
        // `list-functions/sameterm-01.rq` / `-02.rq` and the map twins: a
        // constructor's output must be a pure function of its arguments, or two
        // occurrences of the same call would be two different RDF terms.
        for call in [
            "cdt:List()",
            "cdt:List(1)",
            "cdt:Map()",
            "cdt:Map(1,1)",
            "cdt:List(1, ?undef, 2)",
        ] {
            assert!(
                ask(&format!(
                    "BIND({call} AS ?a) BIND({call} AS ?b) FILTER(SAMETERM(?a, ?b))"
                )),
                "{call} must be sameTerm with itself"
            );
        }
    }

    #[test]
    fn a_constructor_result_is_not_sameterm_with_an_authored_spelling() {
        // The other half of the rule, and the reason the canonical form is applied
        // ONLY to values PurRDF mints: `list-functions/sameterm-03.rq` and
        // `-04.rq` require a constructor's output NOT to be `sameTerm` with a
        // hand-written literal of the same value that differs in whitespace or in
        // how a datatype is abbreviated. Canonicalizing an authored literal would
        // destroy that distinction — and with it the workspace's byte-fidelity rule
        // for literals.
        assert!(ask(
            "BIND(cdt:List(1,2,3) AS ?l) FILTER(!SAMETERM(?l, \"[  1 ,  2  ,   3   ]\"^^cdt:List))"
        ));
        assert!(ask("BIND(cdt:List(1,2) AS ?l) \
             FILTER(!SAMETERM(?l, \"[1,'2'^^<http://www.w3.org/2001/XMLSchema#integer>]\"^^cdt:List))"));
        // …while the two are still EQUAL, because `=` is the value space.
        assert!(ask(
            "BIND(cdt:List(1,2,3) AS ?l) FILTER(?l = \"[  1 ,  2  ,   3   ]\"^^cdt:List)"
        ));
    }

    #[test]
    fn an_authored_literal_keeps_its_own_lexical_form() {
        // The direct statement of the same rule at the term level: binding a
        // composite literal must not re-spell it.
        let bound = select_one("BIND(\"[  1 ,  2 ]\"^^cdt:List AS ?x)").expect("bound");
        let TermValue::Literal { lexical_form, .. } = bound else {
            panic!("expected a literal");
        };
        assert_eq!(lexical_form, "[  1 ,  2 ]");
    }

    // ── the tri-state, kept honest ────────────────────────────────────────────

    #[test]
    fn an_expression_error_is_unbound_and_not_false() {
        // `CdtOutcome::Error` → `Ok(None)`. Each of these is a DIFFERENT way for a
        // SEP-0009 function to have no answer, and every one of them is an
        // expression error rather than `false` or a query failure.
        for call in [
            "cdt:get(\"[1,2,3]\"^^cdt:List, 10)",     // past the end
            "cdt:get(\"[1,2,3]\"^^cdt:List, 0)",      // the index is 1-based
            "cdt:get(\"[null]\"^^cdt:List, 1)",       // the position holds a null
            "cdt:get(\"[1]\"^^cdt:List, 2.0)",        // an xsd:decimal is not an index
            "cdt:head(\"[]\"^^cdt:List)",             // empty
            "cdt:size(\"[1,2]\")",                    // not a composite at all
            "cdt:size(\"1\"^^cdt:List)",              // an ill-formed composite literal
            "cdt:keys(\"[1]\"^^cdt:List)",            // a list has no keys
            "cdt:put(\"{}\"^^cdt:Map, BNODE(), 'a')", // a blank node is not a key
        ] {
            assert!(
                ask(&format!("BIND({call} AS ?r) FILTER(!BOUND(?r))")),
                "{call} must be a SPARQL expression error"
            );
        }
        // …and `cdt:contains` on a missing element is a BOUND `false`, which is the
        // discrimination the corpus draws with its three-way idiom.
        assert!(ask("BIND(cdt:contains(\"[1,2]\"^^cdt:List, 9) AS ?r) \
             FILTER(BOUND(?r)) FILTER(?r = false)"));
    }

    #[test]
    fn a_constructor_argument_that_failed_becomes_a_null_element() {
        // `list-functions/list-constructor-null-01.rq` / `-02.rq`: unbound and
        // errored arguments are the SEP-0009 `null` element, which is the opposite
        // of the ordinary SPARQL rule and so is pinned here explicitly.
        assert!(ask(
            "BIND(cdt:List(?unbound) AS ?l) FILTER(REGEX(STR(?l), \"\\\\[\\\\s*null\\\\s*\\\\]\"))"
        ));
        assert!(ask(
            "BIND(cdt:List(1/0) AS ?l) FILTER(REGEX(STR(?l), \"\\\\[\\\\s*null\\\\s*\\\\]\"))"
        ));
        // A `cdt:Map` key that failed drops the whole pair; a value that failed
        // keeps the entry and stores a null (`map-constructor-08.rq`/`-10.rq`).
        assert!(ask(
            "FILTER(cdt:Map(1,2, ?unbound,4, 5,6) = \"{1:2, 5:6}\"^^cdt:Map)"
        ));
        assert!(ask(
            "BIND(cdt:Map(1,2, 3,?unbound) AS ?m) FILTER(cdt:size(?m) = 2) \
             FILTER(cdt:containsKey(?m, 3)) BIND(cdt:get(?m,3) AS ?v) FILTER(!BOUND(?v))"
        ));
    }

    #[test]
    fn remove_of_an_absent_key_returns_the_caller_s_own_term() {
        // `map-functions/remove-01.rq` asserts this with `SAMETERM`, so returning a
        // re-rendered equal map would fail it. This is `MapRemoval::Unchanged`.
        assert!(ask("BIND(\"{1:'one',  2:'two'}\"^^cdt:Map AS ?in) \
             BIND(cdt:remove(?in, BNODE()) AS ?out) \
             FILTER(BOUND(?out)) FILTER(SAMETERM(?in, ?out))"));
        // Removing a key the map DOES hold mints a fresh, canonical map instead.
        assert!(ask("BIND(\"{1:'one', 2:'two'}\"^^cdt:Map AS ?in) \
             BIND(cdt:remove(?in, 1) AS ?out) FILTER(?out = \"{2:'two'}\"^^cdt:Map)"));
    }

    // ── comparison: SEP-0009's own relations ──────────────────────────────────

    #[test]
    fn a_list_compares_by_value_and_a_map_s_keys_by_term() {
        // The sharpest contrast in the corpus: `list-equals-04.rq` requires
        // `[1,2] = ['+1'^^xsd:integer, 2.0]` to be TRUE (list elements compare in
        // the value space), while `map-equals-04.rq` requires the map twin to be
        // FALSE (a map's KEYS compare by term, so `+1` and `1` are two keys).
        assert!(ask("FILTER(\"[1,2]\"^^cdt:List = \
             \"['+1'^^<http://www.w3.org/2001/XMLSchema#integer>, 2.0]\"^^cdt:List)"));
        assert!(ask("BIND(cdt:Map(1,2) AS ?m) \
             BIND(?m = \"{'+1'^^<http://www.w3.org/2001/XMLSchema#integer> : 2.0}\"^^cdt:Map \
             AS ?r) FILTER(!?r)"));
    }

    #[test]
    fn an_ill_formed_composite_literal_raises_at_evaluation() {
        // The evaluation half of the parse-time rule: `"1"^^cdt:List` parses fine
        // and denotes nothing, so every comparison with it RAISES —
        // `list-functions/list-less-than-error-03.rq` / `-error-04.rq` and the map
        // twins require exactly an unbound `BIND`, on either side of the operator.
        for body in [
            "BIND((\"1\"^^cdt:List < cdt:List(2)) AS ?r)",
            "BIND((cdt:List(1) < \"2\"^^cdt:List) AS ?r)",
            "BIND((\"1\"^^cdt:List = cdt:List(2)) AS ?r)",
            "BIND((\"1\"^^cdt:Map > cdt:Map(1,2)) AS ?r)",
        ] {
            assert!(
                ask(&format!("{body} FILTER(!BOUND(?r))")),
                "{body} must raise"
            );
        }
    }

    #[test]
    fn less_or_equal_is_not_less_or_equal() {
        // `list-less-equal-28.rq` requires `"[_:b]" <= "[_:b]"` to be UNBOUND while
        // `list-equals-07.rq` requires the same operands to be `=`-equal. Computing
        // `<=` as `(a < b) || (a = b)` would answer `true`, because SPARQL's `||`
        // reads `error || true` as `true`. The two operands here are also the SAME
        // RDF term, which is why the composite diversion has to happen before the
        // evaluator's sameTerm short-circuit.
        assert!(ask(
            "BIND((\"[_:b]\"^^cdt:List <= \"[_:b]\"^^cdt:List) AS ?r) FILTER(!BOUND(?r))"
        ));
        assert!(ask(
            "BIND((\"[_:b]\"^^cdt:List >= \"[_:b]\"^^cdt:List) AS ?r) FILTER(!BOUND(?r))"
        ));
        assert!(ask(
            "FILTER(\"[   _:b   ]\"^^cdt:List = \"[_:b]\"^^cdt:List)"
        ));
        // Two DIFFERENT blank nodes are undecidable under `=`, not `false`
        // (`list-equals-06.rq`).
        assert!(ask(
            "BIND((\"[_:b1]\"^^cdt:List = \"[_:b2]\"^^cdt:List) AS ?r) FILTER(!BOUND(?r))"
        ));
    }

    #[test]
    fn ordering_is_lexicographic_then_by_length() {
        assert!(ask("FILTER(\"[1,2]\"^^cdt:List < \"[1,3]\"^^cdt:List)"));
        assert!(ask("FILTER(\"[1]\"^^cdt:List < \"[1,2]\"^^cdt:List)"));
        assert!(ask("FILTER(\"[1,3]\"^^cdt:List > \"[1,2]\"^^cdt:List)"));
        assert!(ask("FILTER(\"[  ]\"^^cdt:List >= \"[]\"^^cdt:List)"));
        assert!(ask("BIND((\"[]\"^^cdt:List < \"[  ]\"^^cdt:List) AS ?r) \
             FILTER(BOUND(?r)) FILTER(?r = false)"));
    }

    // ── blank nodes ───────────────────────────────────────────────────────────

    #[test]
    fn one_label_inside_a_composite_is_one_blank_node() {
        // `bnodes/bnodes-sparql-01.rq` and `-02.rq`.
        assert!(ask("BIND(\"[_:b, 42, _:b]\"^^cdt:List AS ?l) \
             BIND(cdt:get(?l,1) AS ?a) BIND(cdt:get(?l,3) AS ?c) \
             FILTER(isBLANK(?a)) FILTER(isBLANK(?c)) FILTER(?a = ?c)"));
        assert!(ask(
            "BIND(\"{ '1': _:b, '2': 42, '3': _:b }\"^^cdt:Map AS ?m) \
             BIND(cdt:get(?m,'1') AS ?a) BIND(cdt:get(?m,'3') AS ?c) \
             FILTER(isBLANK(?a)) FILTER(isBLANK(?c)) FILTER(?a = ?c)"
        ));
        // …and two different labels are two different nodes (`bnodes-sparql-03.rq`).
        assert!(ask("BIND(\"[_:b1, 42, _:b2]\"^^cdt:List AS ?l) \
             BIND(cdt:get(?l,1) AS ?a) BIND(cdt:get(?l,3) AS ?c) \
             FILTER(isBLANK(?a)) FILTER(isBLANK(?c)) FILTER(?a != ?c)"));
        // A label shared across two composite literals, and across a nesting level,
        // is still one node (`bnodes-sparql-05.rq`, `-09.rq`, `-11.rq`).
        assert!(ask(
            "BIND(\"[_:b, 42]\"^^cdt:List AS ?l) BIND(\"{ '1': _:b }\"^^cdt:Map AS ?m) \
             BIND(cdt:get(?l,1) AS ?a) BIND(cdt:get(?m,'1') AS ?c) FILTER(?a = ?c)"
        ));
        assert!(ask("BIND(\"[_:b, 42, [_:b] ]\"^^cdt:List AS ?l) \
             BIND(cdt:get(?l,1) AS ?a) BIND(cdt:get(cdt:get(?l,3),1) AS ?c) \
             FILTER(isBLANK(?a)) FILTER(?a = ?c)"));
    }

    #[test]
    fn a_minted_blank_node_survives_the_round_trip() {
        // `list-constructor-16.rq`: a `BNODE()` put into a list and read back out is
        // the SAME term, which is the `qualify_label`/`unqualify_label` round trip
        // being the identity rather than a lossy re-labelling.
        assert!(ask(
            "BIND(BNODE() AS ?b) BIND(cdt:List(?b) AS ?l) FILTER(BOUND(?l)) \
             BIND(cdt:get(?l,1) AS ?e) FILTER(isBLANK(?e)) FILTER(SAMETERM(?e, ?b))"
        ));
    }

    // ── RDF 1.2 term types survive the bridge ─────────────────────────────────

    #[test]
    fn rdf_12_term_types_round_trip_through_a_composite() {
        // A triple term and a directional language-tagged string are both RDF 1.2
        // first-class terms, and both are PurRDF supersets of the SEP-0009 lexical
        // space. Refusing either would be refusing an RDF 1.2 term type outright, so
        // they must survive a constructor → `cdt:get` round trip as the same term.
        assert!(ask(
            "BIND(TRIPLE(<http://example.org/s>, <http://example.org/p>, 42) AS ?t) \
             BIND(cdt:List(?t) AS ?l) BIND(cdt:get(?l,1) AS ?e) \
             FILTER(isTRIPLE(?e)) FILTER(SAMETERM(?e, ?t))"
        ));
        assert!(ask("BIND(STRLANGDIR('hello', 'en', 'ltr') AS ?d) \
             BIND(cdt:List(?d) AS ?l) BIND(cdt:get(?l,1) AS ?e) \
             FILTER(SAMETERM(?e, ?d)) FILTER(LANGDIR(?e) = 'ltr')"));
        // An IRI and a plain string round-trip too, so the leaf mapping is total.
        assert!(ask("BIND(cdt:List(<http://example.org/a>, 'b') AS ?l) \
             FILTER(SAMETERM(cdt:get(?l,1), <http://example.org/a>)) \
             FILTER(SAMETERM(cdt:get(?l,2), 'b'))"));
    }

    // ── bounds and termination ────────────────────────────────────────────────

    #[test]
    fn nesting_is_bounded_by_the_element_and_byte_bounds_alone() {
        // A composite nests as deep as its elements and bytes allow: each level is
        // one element, so no separate depth bound exists. 256 nested constructors
        // mint a value whose canonical form is exactly that many bracket pairs, and
        // whose size is the one element of its outermost list.
        let nest = |depth: usize| {
            let mut expression = "cdt:List()".to_owned();
            for _ in 1..depth {
                expression = format!("cdt:List({expression})");
            }
            expression
        };
        let brackets = "[".repeat(256) + &"]".repeat(256);
        assert!(ask(&format!(
            "BIND({} AS ?x) FILTER(BOUND(?x)) FILTER(STR(?x) = \"{brackets}\") \
             FILTER(cdt:size(?x) = 1)",
            nest(256)
        )));
        // The neighbour with no nesting at all: an empty list has no element.
        assert!(ask(
            "BIND(cdt:List() AS ?x) FILTER(STR(?x) = \"[]\") FILTER(cdt:size(?x) = 0)"
        ));
    }

    #[test]
    fn a_doubling_put_chain_is_refused_rather_than_exhausting_memory() {
        // `cdt:put(?m, ?k, ?m)` with a fresh key each time roughly DOUBLES the map's
        // element count per application, so a query of a couple of dozen lines asks
        // for a value no host can hold. `purrdf-cdt` measures each prospective
        // result from BORROWED parts before cloning any of it, so the chain is
        // refused at the step that would cross `MAX_ELEMENTS` — with the previous,
        // admissible value the largest thing ever allocated.
        //
        // Driven through the real engine, not the value layer, because the property
        // under test is that the refusal survives every layer between them: the
        // `CdtOutcome::Bound` must reach the query boundary as a failure rather than
        // being folded into an expression error somewhere on the way out.
        use std::fmt::Write as _;

        let mut binds = String::from("BIND(\"{'a'@en: null}\"^^cdt:Map AS ?m0) ");
        let steps = 24;
        for k in 1..=steps {
            let key = char::from(b'a' + u8::try_from(k).expect("k < 26"));
            write!(
                binds,
                "BIND(cdt:put(?m{p}, '{key}'@en, ?m{p}) AS ?m{k}) ",
                p = k - 1
            )
            .expect("writing to a String cannot fail");
        }
        let error = ask_err(&format!("{binds} FILTER(BOUND(?m{steps}))"));
        assert!(
            error.diagnostic_code() == Some(EvalError::COMPOSITE_BOUND_CODE),
            "got {error:?}"
        );
        assert!(error.to_string().contains("elements"), "got {error}");
    }
}

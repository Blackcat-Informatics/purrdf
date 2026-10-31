// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The SEP-0009 **`UNFOLD` graph pattern**: one solution per element of a
//! composite value.
//!
//! ```text
//! [174] Unfold ::= 'UNFOLD' '(' Expression 'AS' Var ( ',' Var )? ')'
//! ```
//!
//! [`crate::cdt_agg`]'s dual: `FOLD` collapses a group's rows into one composite,
//! `UNFOLD` expands one composite back into rows. Each input solution contributes
//! its own contiguous block of output solutions, in the composite's own order,
//! and the blocks follow their input rows' order — which is what makes this
//! operator's edge to its child monotone (see
//! `crate::governor::soundness::visit_pattern_parts`).
//!
//! # What the two variables bind
//!
//! Decided by the value's DATATYPE, not by the syntax — the same expression
//! reaching a `cdt:List` in one row and a `cdt:Map` in the next binds different
//! things in each, which is why [`purrdf_sparql_algebra::GraphPattern::Unfold`]'s
//! fields are named for their position:
//!
//! | value | `element` | `companion` |
//! |---|---|---|
//! | `cdt:List` | the element, in list order, duplicates preserved (`unfold-list-1var-01/02`) | the **1-based** `xsd:integer` index (`unfold-list-2vars-01`) |
//! | `cdt:Map` | the entry's key (`unfold-map-1var-01`) | the entry's value (`unfold-map-2vars-01`) |
//!
//! A `null` in either position binds NOTHING and does **not** suppress the row: a
//! null list element yields a row whose `element` is unbound while its index is
//! still bound (`unfold-list-1var-09/10`, `unfold-list-2vars-09`), and a null map
//! value yields a row binding the key alone (`unfold-map-2vars-08`). That is the
//! same `null`-is-not-a-term rule `cdt:get` follows when it answers an expression
//! error for a null position — read as a graph pattern rather than an expression,
//! "no term" is an unbound column, not a dropped row.
//!
//! A blank node inside the composite is a real blank node scoped to the literal
//! and shared within it, so two occurrences of one label bind the SAME node
//! (`unfold-list-1var-08`) and two labels bind two (`-07`). Nothing here does that
//! on purpose: it falls out of [`crate::cdt_fn::from_cdt_term_admitted`]'s `(label, scope)`
//! round trip, which is also why an unfolded element is `sameTerm` with what
//! `cdt:get` returns for the same position (all ten `unfold-get-*` cases).
//!
//! # A row whose expression denotes no composite PASSES THROUGH
//!
//! Unbound, raised, not `cdt:`-typed, or a `cdt:`-typed literal whose lexical form
//! does not parse: all four are "this row has nothing to expand", and all four
//! emit the INPUT ROW UNCHANGED, with both targets unbound.
//!
//! This is not a first-party choice. SEP-0009 §12.3 fixes it in the definitions
//! of both algebra operators, in the same words: "If var ∉ dom(μ) and expr(μ) is
//! an error or an RDF term that is neither a well-formed cdt:List literal nor a
//! well-formed cdt:Map literal, then Unfold1(μ, var, expr) = { μ }", with
//! cardinality 1 at μ. `Unfold2` repeats it for its pair.
//!
//! The vendored corpus does NOT pin this — all 42 `unfold` cases use a
//! well-formed composite — so reading the corpus alone leads to the opposite,
//! plausible-sounding answer (a graph pattern that matches nothing yields no
//! rows). The spec text is the authority here, and it says otherwise.
//!
//! An EMPTY composite is a DIFFERENT condition with a different answer: zero
//! rows. `UNFOLD` is one row per element, and zero elements is zero rows — see
//! the Notes under both §12.3 definitions, which state the empty-multiset result
//! for the empty term list and the empty term map explicitly. It is not one row
//! with an unbound target either, since that would claim an element exists whose
//! value is `null`, a different value from an empty list.
//!
//! Keeping the two apart is load-bearing rather than pedantic: under `OPTIONAL`,
//! `MINUS` and `FILTER NOT EXISTS`, a column that is only SOMETIMES a composite
//! gives different answers depending on which reading is used.
//!
//! The one thing that is NOT swallowed is a hard failure while evaluating the
//! expression itself ([`crate::error::EvalError::CompositeBound`] from a
//! constructor whose result crosses a `purrdf-cdt` bound): that propagates and
//! fails the query, exactly as it does in every other expression position.
//!
//! # A pre-bound target is a join, not a shadow
//!
//! The parser refuses `UNFOLD` over a variable already in scope (`BIND`'s §19.6
//! rule, applied to both targets), so query text never reaches the case below.
//! Algebra built directly through the public API can, and the answer is SPARQL's
//! own: the produced binding is JOINED against what the row already holds — the
//! row survives only if the two agree — rather than overwriting it. A `null`
//! position produces no binding at all, so it agrees with whatever was there and
//! leaves it untouched.
//!
//! # A nested composite unfolds again
//!
//! An element that is itself a composite comes back as a `cdt:`-typed literal in
//! canonical form — the same term `cdt:get` returns for that position — because
//! [`crate::cdt_fn::from_cdt_term_admitted`] re-renders a nested value rather than
//! flattening it. A second `UNFOLD` over the bound element therefore expands the
//! inner composite with no further ceremony, which is what makes the operator
//! compose with itself.

use purrdf_cdt::{CdtContents, CdtValue};
use purrdf_core::{DatasetView, TermValue, TrippedGovernor};
use purrdf_sparql_algebra::{Expression, GraphPattern};

use crate::error::EvalError;
use crate::eval::EvalCtx;
use crate::governor::lift::{Evaluated, Truncation};
use crate::row_ingest::{GovernedRowIngest, IngestVerdict};
use crate::solution::{Solution, SolutionSeq};

/// One original UNFOLD input row remains suspended until its expression completes.
pub(crate) struct UnfoldSequence<'e, I: purrdf_core::ViewTermId> {
    input_schema: crate::solution::SharedSchema,
    out_schema: crate::solution::SharedSchema,
    input: Option<crate::solution::RowBagIntoIter<I>>,
    output: crate::solution::RowsBuilder<I>,
    checkpoint: crate::row_checkpoint::RowCheckpoint,
    ingest: GovernedRowIngest,
    linked: Option<crate::vm::Linked<'e, I>>,
    active: Option<crate::vm::RowExpressionFrame<'e, I>>,
    lift: crate::governor::lift::DetachedLift,
    element_col: usize,
    companion_col: Option<usize>,
    next_row: u64,
    tripped: Option<TrippedGovernor>,
}

#[expect(
    clippy::large_enum_variant,
    reason = "inline handoff preserves the driver admitted frame ownership without another allocation"
)]
pub(crate) enum UnfoldStart<'e, I: purrdf_core::ViewTermId> {
    Complete(Evaluated<I>),
    Sequence(UnfoldSequence<'e, I>),
}
#[expect(
    clippy::large_enum_variant,
    reason = "inline handoff preserves the driver admitted frame ownership without another allocation"
)]
pub(crate) enum UnfoldStep<'e, 'd, I: purrdf_core::ViewTermId> {
    Complete(Evaluated<I>),
    Suspend(UnfoldSequence<'e, I>, crate::vm::Suspend<'d>),
}

pub(crate) fn begin_unfold_child_with_owner<'e, D: DatasetView + Sync>(
    node: &GraphPattern,
    expression: &Expression,
    owner: crate::expr::FilterLink<'e>,
    lift: crate::governor::lift::DetachedLift,
    child: Evaluated<D::Id>,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<UnfoldStart<'e, D::Id>, EvalError> {
    let GraphPattern::Unfold {
        inner,
        element,
        companion,
        ..
    } = node
    else {
        return Err(EvalError::internal("UNFOLD continuation lost its node"));
    };
    let mut lift = lift.attach(node);
    let Some(seq) = lift.absorb(0, child) else {
        let mut schema = match lift.absorbed_schema() {
            Some(schema) => (*schema).clone(),
            None => (*crate::eval::syntactic_schema_admitted(inner, &ctx.growth)?).clone(),
        };
        schema.push_admitted(element.clone(), &ctx.growth)?;
        if let Some(companion) = companion {
            schema.push_admitted(companion.clone(), &ctx.growth)?;
        }
        return Ok(UnfoldStart::Complete(
            lift.finish(SolutionSeq::empty(schema.shared_admitted(&ctx.growth)?)),
        ));
    };
    let checkpoint = crate::row_checkpoint::RowCheckpoint::for_rows(
        ctx,
        crate::governor::ChargePoint::RowExpressionEvaluation,
        false,
        seq.rows.len(),
    );
    let mut schema = (*seq.schema).clone();
    let element_col = schema.push_admitted(element.clone(), &ctx.growth)?;
    let companion_col = companion
        .as_ref()
        .map(|companion| schema.push_admitted(companion.clone(), &ctx.growth))
        .transpose()?;
    let schema = schema.shared_admitted(&ctx.growth)?;
    let ingest = GovernedRowIngest::new(ctx, schema.len(), None);
    let program = crate::vm::program_at(ctx, node, expression)?;
    let linked = match owner {
        crate::expr::FilterLink::Borrowed(original) => {
            crate::vm::Linked::link_admitted(program, original, &seq.schema, ctx)?
        }
        crate::expr::FilterLink::Copied(pattern) => {
            crate::vm::Linked::link_copied_filter_owner_admitted(
                program,
                expression,
                pattern,
                &seq.schema,
                ctx,
            )?
        }
    };
    Ok(UnfoldStart::Sequence(UnfoldSequence {
        input_schema: seq.schema,
        out_schema: schema,
        input: Some(seq.rows.into_iter()),
        output: crate::solution::RowsBuilder::new(&ctx.growth),
        checkpoint,
        ingest,
        linked: Some(linked),
        active: None,
        lift: lift.detach(),
        element_col,
        companion_col,
        next_row: 0,
        tripped: None,
    }))
}

impl<'e, I: purrdf_core::ViewTermId> UnfoldSequence<'e, I> {
    pub(crate) fn step<'d, D: DatasetView<Id = I> + Sync>(
        mut self,
        node: &GraphPattern,
        ctx: &mut EvalCtx<'d, D>,
    ) -> Result<UnfoldStep<'e, 'd, I>, EvalError> {
        loop {
            let mut frame = if let Some(frame) = self.active.take() {
                frame
            } else {
                let Some(row) = self.input.as_mut().and_then(Iterator::next) else {
                    break;
                };
                if let Err(tripped) = self.checkpoint.pass(ctx) {
                    self.tripped = Some(tripped);
                    break;
                }
                ctx.current_row = self.next_row;
                self.next_row = self
                    .next_row
                    .checked_add(1)
                    .ok_or(EvalError::WorkspaceBoundOverflow)?;
                crate::vm::RowExpressionFrame::begin(
                    self.linked.take().expect("one UNFOLD link"),
                    row,
                    ctx,
                )?
            };
            match frame.poll(ctx)? {
                crate::vm::VmStep::Suspend(request) => {
                    self.active = Some(frame);
                    return Ok(UnfoldStep::Suspend(self, request));
                }
                crate::vm::VmStep::Value(value) => {
                    let (linked, row, value, _evidence) = frame.finish_term(value)?;
                    self.linked = Some(linked);
                    if !self.commit(&row, value, ctx)? {
                        break;
                    }
                }
            }
        }
        drop(self.input.take());
        let width = self.out_schema.len();
        if self.tripped.is_none() {
            self.tripped = ctx.observe_cells(self.output.len(), width).err();
        }
        if let Some(tripped) = ctx.expression_barrier.observed() {
            return Ok(UnfoldStep::Complete(Evaluated::Truncated(
                Truncation::barred_at(node, tripped, self.out_schema),
            )));
        }
        let seq = SolutionSeq {
            schema: self.out_schema,
            rows: self.output.finish()?,
        };
        let lift = self.lift.attach(node);
        Ok(UnfoldStep::Complete(match self.tripped {
            Some(tripped) if !lift.is_truncated() => {
                Evaluated::Truncated(Truncation::origin(seq, tripped))
            }
            _ => lift.finish(seq),
        }))
    }
    fn commit<D: DatasetView<Id = I> + Sync>(
        &mut self,
        mu: &crate::solution::RetainedRow<I>,
        term: Option<crate::scratch::SolutionTerm<I>>,
        ctx: &mut EvalCtx<'_, D>,
    ) -> Result<bool, EvalError> {
        let width = self.out_schema.len();
        let value = composite_term(term, ctx)?;
        let Some(value) = value else {
            if let Some(governor) = ctx.stop_check() {
                self.tripped = Some(governor);
                return Ok(false);
            }
            if let IngestVerdict::Abandoned(governor) = self.ingest.admit(ctx, self.output.len()) {
                self.tripped = governor;
                return Ok(false);
            }
            self.output.push_cells(
                width,
                (0..width).map(|column| mu.get(column).copied().flatten()),
            )?;
            return Ok(true);
        };
        let count = match value.contents() {
            CdtContents::List(items) => items.len(),
            CdtContents::Map(entries) => entries.len(),
        };
        for position in 0..count {
            if let Some(governor) = ctx.stop_check() {
                self.tripped = Some(governor);
                return Ok(false);
            }
            if let IngestVerdict::Abandoned(governor) = self.ingest.admit(ctx, self.output.len()) {
                self.tripped = governor;
                return Ok(false);
            }
            let growth = ctx.growth.clone();
            let (element_term, companion_term) = expansion_at(&value, position, &growth)?;
            let mut compatible = true;
            let row = crate::solution::RetainedRow::try_build(width, &growth, |row| {
                row[..self.input_schema.len()].copy_from_slice(mu);
                compatible = bind_admitted(row, Some(self.element_col), element_term, ctx)?
                    && bind_admitted(row, self.companion_col, companion_term, ctx)?;
                Ok(())
            })?;
            if compatible {
                self.output.push_row(row)?;
            }
        }
        Ok(true)
    }
}

fn composite_term<D: DatasetView + Sync>(
    term: Option<crate::scratch::SolutionTerm<D::Id>>,
    ctx: &EvalCtx<'_, D>,
) -> Result<Option<crate::composite_value::CompositeValue>, EvalError> {
    let Some(term) = term else {
        return Ok(None);
    };
    let value = crate::expr::owned_value_of(ctx, term)?;
    let TermValue::Literal {
        lexical_form,
        datatype,
        language: None,
        ..
    } = &*value
    else {
        return Ok(None);
    };
    crate::composite_value::CompositeValue::parse(lexical_form, datatype, &ctx.growth)
}

fn expansion_at(
    value: &CdtValue,
    position: usize,
    workspace: &crate::WorkspaceCapability,
) -> Result<(Option<crate::WorkspaceTerm>, Option<crate::WorkspaceTerm>), EvalError> {
    match value.contents() {
        CdtContents::List(items) => Ok((
            crate::cdt_fn::from_cdt_term_admitted(&items[position], workspace)?,
            Some(workspace.literal(position + 1, purrdf_xsd::datatype::XSD_INTEGER)?),
        )),
        CdtContents::Map(entries) => {
            let entry = &entries[position];
            let key = match &entry.key {
                purrdf_cdt::CdtKey::Iri(iri) => workspace.iri(iri)?,
                purrdf_cdt::CdtKey::Literal(literal) => {
                    crate::cdt_fn::cdt_literal_admitted(literal, workspace)?
                }
            };
            Ok((
                Some(key),
                crate::cdt_fn::from_cdt_term_admitted(&entry.value, workspace)?,
            ))
        }
    }
}

/// Write one produced binding into `row`, reporting whether the row survives.
///
/// `column` is `None` for the one-variable form's absent companion, and `value`
/// is `None` for a `null` position: neither produces a binding, and both leave
/// whatever the row already held in place. A produced binding is JOINED against
/// an existing one — see the module docs' note on a pre-bound target — so the
/// answer is `false` exactly when the two disagree.
///
/// "Exactly when" is load-bearing: `false` drops the whole row, so a produced
/// binding that simply cannot be made — a member whose language tag the grammar
/// refuses — is `true` with the column left unbound, the same answer a `null`
/// gets. See the comment at the [`ScratchInterner::intern_checked`] call.
///
/// [`ScratchInterner::intern_checked`]: crate::scratch::ScratchInterner::intern_checked
fn bind_admitted<D: DatasetView + Sync>(
    row: &mut Solution<D::Id>,
    column: Option<usize>,
    value: Option<crate::WorkspaceTerm>,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<bool, EvalError> {
    let (Some(column), Some(value)) = (column, value) else {
        return Ok(true);
    };
    // A produced binding the interner refuses is a binding to something that is
    // not an RDF term — an `UNFOLD` member lifted out of a composite literal's
    // lexical form carrying an ungrammatical language tag.
    //
    // It costs its own binding and NOTHING else, which is why this is `true` and
    // not `false`. `false` drops the whole row at the call site, and in the map
    // form `expansion` yields `(key, value)` through two calls to this function:
    // an ungrammatical tag in the VALUE would have taken the perfectly good KEY
    // binding down with it, which is the silent-drop bug wearing a refusal's
    // clothes. The neighbour two lines up is the precedent and it is exact — a
    // `null` map value is "this entry has no value", returns `true`, and leaves
    // the companion column unbound while the row survives. An unbound column is
    // also what SPARQL 1.1 §17.2 specifies for an expression error, and it is
    // what `crate::expr::eval_str_lang` already returns for the same garbage.
    //
    // `false` remains reserved for its one meaning: a produced binding that
    // DISAGREES with one the row already holds, below — a genuine non-match.
    // The door judges the member's language tags over a work list and then walks the
    // term — looks it up, hashes it, compares it — over work lists too, so a member
    // nested to any depth costs no machine stack. A nested composite comes back as a
    // single `cdt:`-typed literal, so the only nesting a member carries is that of the
    // triple terms inside it; `purrdf-cdt` bounds that depth by its byte bound alone.
    let Some(term) = ctx.intern_workspace_term(value)? else {
        return Ok(true);
    };
    Ok(match row[column] {
        None => {
            row[column] = Some(term);
            true
        }
        Some(existing) => existing == term,
    })
}

#[cfg(test)]
fn bind<D: DatasetView + Sync>(
    row: &mut Solution<D::Id>,
    column: Option<usize>,
    value: Option<&TermValue>,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<bool, EvalError> {
    let value = value
        .map(|value| ctx.growth.clone_term(value))
        .transpose()?;
    bind_admitted(row, column, value, ctx)
}

#[cfg(test)]
mod tests {
    use purrdf_core::RdfDatasetBuilder;

    use super::*;

    /// `"purr"@tag`, as a value the interner has to judge.
    fn tagged(tag: &str) -> TermValue {
        TermValue::Literal {
            lexical_form: "purr".to_owned(),
            datatype: "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString".to_owned(),
            language: Some(tag.to_owned()),
            direction: None,
        }
    }

    /// A refused member costs its own binding and nothing else.
    ///
    /// Driven at [`bind`] rather than through query text because that is the only
    /// place the distinction exists. Upstream, `purrdf-cdt`'s own `LANGTAG` scan
    /// judges on the same profile, so a bad tag in a composite's lexical form
    /// makes the composite ill-formed and never reaches here (pinned by
    /// `tests/cdt_fold_unfold_e2e.rs`). This is therefore an INVARIANT test, not
    /// a reproduction: `bind`'s `false` is documented to mean "the produced
    /// binding disagrees with one the row already holds", and returning it for a
    /// refused tag broke that — in the map form `expansion` yields `(key, value)`
    /// through two calls, so a refused VALUE erased a perfectly good KEY.
    #[test]
    fn a_refused_tag_costs_its_own_binding_and_nothing_else() {
        let dataset = RdfDatasetBuilder::new().freeze().expect("freeze");
        let mut ctx = EvalCtx::new(&*dataset);
        let mut row: Solution<_> = purrdf_core::smallvec![None; 2];

        assert!(
            bind(
                &mut row,
                Some(0),
                Some(&TermValue::iri("https://example.org/k")),
                &mut ctx
            )
            .unwrap()
        );
        assert!(
            bind(&mut row, Some(1), Some(&tagged("en us")), &mut ctx).unwrap(),
            "a refused tag must not drop the row — `false` here erases the key"
        );
        assert!(row[0].is_some(), "the key keeps its binding");
        assert!(row[1].is_none(), "only the refused column is unbound");
    }

    /// The over-refusal mirror, at the same door: a tag real data carries binds.
    #[test]
    fn every_tag_the_grammar_admits_still_binds() {
        let dataset = RdfDatasetBuilder::new().freeze().expect("freeze");
        let mut ctx = EvalCtx::new(&*dataset);
        for tag in [
            "en",
            "en-US",
            "zh-Hans-CN",
            "de-CH-x-phonebk",
            "i-enochian",
            "x-purrdf-afrikaans",
            "x-gmeow-english",
            "en-fr-jura",
            "fr-be-fbcl",
            "abcdefgh",
            "en-x-cantbethislong",
        ] {
            let mut row: Solution<_> = purrdf_core::smallvec![None; 1];
            assert!(bind(&mut row, Some(0), Some(&tagged(tag)), &mut ctx).unwrap());
            assert!(row[0].is_some(), "{tag} must still bind");
        }
    }

    /// And `false` keeps its one meaning: a produced binding that DISAGREES with
    /// one the row already holds. Without this the fix above would read as
    /// "`bind` never fails", which is not what changed.
    #[test]
    fn a_disagreeing_pre_bound_target_is_still_a_non_match() {
        let dataset = RdfDatasetBuilder::new().freeze().expect("freeze");
        let mut ctx = EvalCtx::new(&*dataset);
        let mut row: Solution<_> = purrdf_core::smallvec![None; 1];
        assert!(
            bind(
                &mut row,
                Some(0),
                Some(&TermValue::iri("https://example.org/a")),
                &mut ctx
            )
            .unwrap()
        );
        assert!(
            !bind(
                &mut row,
                Some(0),
                Some(&TermValue::iri("https://example.org/b")),
                &mut ctx
            )
            .unwrap(),
            "two disagreeing bindings for one column is a non-match"
        );
    }
}

crate::vm::row_continuation!(UnfoldSequence, input_schema);

// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SPARQL expression evaluation (FILTER / BIND / EXISTS), plus the `Filter` and
//! `Extend` graph-pattern nodes that drive it.
//!
//! [`eval_expr`] maps an [`Expression`] over one solution to
//! `Ok(Some(term))` (a value), `Ok(None)` (a SPARQL **error / unbound** — the
//! third truth value), or `Err` (a hard [`EvalError::Unsupported`] for a construct
//! outside the current S6 scope). The `Ok(None)` vs `Err` split is load-bearing: a
//! type error is normal three-valued logic (it makes a FILTER drop the row), while
//! an unimplemented builtin is a hard failure (never a wrong answer).
//!
//! ## Scope (S6)
//!
//! Implemented: logical `&&`/`||`/`!` (Kleene three-valued), comparisons and
//! `sameTerm`, `BOUND`, `IN`, `IF`, `COALESCE`, `EXISTS`, the string/type/RDF
//! built-ins the corpus uses, **numeric arithmetic** (`+ - * /`, unary sign),
//! **`ABS`/`CEIL`/`FLOOR`/`ROUND`**, and (Gap 4) **`ENCODE_FOR_URI`**,
//! **`NOW`**, **`YEAR`/`MONTH`/`DAY`/`HOURS`/`MINUTES`/`SECONDS`**,
//! **`TIMEZONE`/`TZ`/`ADJUST`**, **`MD5`/`SHA1`/`SHA256`/`SHA384`/`SHA512`**,
//! **`RAND`**, and **`UUID`/`STRUUID`**. Unsupported (`Unsupported`):
//! `SERVICE`, property paths, and `Function::Custom`.

use std::cmp::Ordering;
use std::sync::Arc;

use purrdf_core::{
    BlankScope, DatasetView, GraphMatch, RdfTextDirection, TermRef, TermValue, ViewTermId,
};
use purrdf_sparql_algebra::{Expression, Function, GraphPattern, PurrdfFn, Variable};
use purrdf_xsd::{
    XsdDatatype, XsdValue, effective_boolean_value, numeric_abs, numeric_ceil, numeric_floor,
    numeric_round, numeric_unary_plus, parse_by_iri, parse_xsd10, value_add, value_cmp, value_div,
    value_equal, value_mul, value_sub, value_unary_minus,
};
use sha2::Digest; // brings the Digest trait in scope for all RustCrypto hash calls

use crate::DetHashSet;
use crate::error::EvalError;
use crate::eval::{EvalCtx, eval_evaluated};
use crate::governor::lift::{Evaluated, Lift, Truncation};
use crate::scratch::SolutionTerm;
use crate::solution::{SolutionSeq, VarSchema};

const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";
const RDF_DIR_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString";

/// Evaluate an expression over a solution. See the [module docs](self) for the
/// `Ok(Some)` / `Ok(None)` / `Err` contract.
pub(crate) fn eval_expr<D: DatasetView + Sync>(
    expr: &Expression,
    row: &[Option<SolutionTerm<D::Id>>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    match expr {
        // ---- atoms ---------------------------------------------------------
        Expression::NamedNode(n) => Ok(Some(const_atom(ctx, expr, || {
            TermValue::Iri(n.as_str().to_owned())
        }))),
        Expression::Literal(l) => Ok(Some(const_atom(ctx, expr, || {
            crate::convert::literal_to_value(l)
        }))),
        Expression::Variable(v) => Ok(lookup(v, row, schema)),
        Expression::Bound(v) => Ok(Some(bool_term(ctx, lookup(v, row, schema).is_some()))),

        // ---- logical (Kleene three-valued) --------------------------------
        Expression::Or(a, b) => {
            let va = ebv_of(a, row, schema, ctx)?;
            let vb = ebv_of(b, row, schema, ctx)?;
            let r = match (va, vb) {
                (Some(true), _) | (_, Some(true)) => Some(true),
                (Some(false), Some(false)) => Some(false),
                _ => None,
            };
            Ok(r.map(|b| bool_term(ctx, b)))
        }
        Expression::And(a, b) => {
            let va = ebv_of(a, row, schema, ctx)?;
            let vb = ebv_of(b, row, schema, ctx)?;
            let r = match (va, vb) {
                (Some(false), _) | (_, Some(false)) => Some(false),
                (Some(true), Some(true)) => Some(true),
                _ => None,
            };
            Ok(r.map(|b| bool_term(ctx, b)))
        }
        Expression::Not(a) => {
            let v = ebv_of(a, row, schema, ctx)?;
            Ok(v.map(|b| bool_term(ctx, !b)))
        }

        // ---- comparisons ---------------------------------------------------
        // `=` is RDFterm-equality, NOT an ordering test: distinct IRIs/blank nodes
        // are *unequal* (`false`), not a type error. Routing `=` through the
        // ordering `compare` (which returns a type error for un-orderable IRI pairs)
        // would make `?a = ?b` — and therefore the desugared `?a != ?b` — evaluate
        // to an error (and so filter the row out) whenever the two IRIs differ. The
        // dedicated `equal` path applies the value-equality semantics of `rdf_equal`.
        Expression::Equal(a, b) => equal(a, b, row, schema, ctx),
        Expression::Greater(a, b) => compare(a, b, row, schema, ctx, |c| c == Ordering::Greater),
        Expression::GreaterOrEqual(a, b) => {
            compare(a, b, row, schema, ctx, |c| c != Ordering::Less)
        }
        Expression::Less(a, b) => compare(a, b, row, schema, ctx, |c| c == Ordering::Less),
        Expression::LessOrEqual(a, b) => {
            compare(a, b, row, schema, ctx, |c| c != Ordering::Greater)
        }
        Expression::SameTerm(a, b) => {
            let ta = eval_expr(a, row, schema, ctx)?;
            let tb = eval_expr(b, row, schema, ctx)?;
            Ok(match (ta, tb) {
                (Some(x), Some(y)) => Some(bool_term(ctx, x == y)),
                _ => None,
            })
        }

        // ---- conditionals --------------------------------------------------
        Expression::If(c, t, e) => match ebv_of(c, row, schema, ctx)? {
            Some(true) => eval_expr(t, row, schema, ctx),
            Some(false) => eval_expr(e, row, schema, ctx),
            None => Ok(None),
        },
        Expression::Coalesce(items) => {
            for item in items {
                if let Some(term) = eval_expr(item, row, schema, ctx)? {
                    return Ok(Some(term));
                }
            }
            Ok(None)
        }
        Expression::In(needle, haystack) => eval_in(needle, haystack, row, schema, ctx),

        // ---- EXISTS --------------------------------------------------------
        Expression::Exists(pattern) => {
            let found = exists(pattern, row, schema, ctx)?;
            Ok(Some(bool_term(ctx, found)))
        }

        // ---- arithmetic ---------------------------------------------------
        // SPARQL three-valued contract: type errors (non-numeric/non-temporal
        // operands, overflow, divide-by-zero, and the indeterminate-timezone
        // instant-difference case) → Ok(None), NOT Err. A hard EvalError would
        // propagate out of FILTER and break the query; Ok(None) just drops the row.
        Expression::Add(a, b) => binary_value(a, b, row, schema, ctx, value_add),
        Expression::Subtract(a, b) => binary_value(a, b, row, schema, ctx, value_sub),
        Expression::Multiply(a, b) => binary_value(a, b, row, schema, ctx, value_mul),
        Expression::Divide(a, b) => binary_value(a, b, row, schema, ctx, value_div),
        Expression::UnaryPlus(a) => unary_numeric(a, row, schema, ctx, numeric_unary_plus),
        Expression::UnaryMinus(a) => unary_numeric(a, row, schema, ctx, value_unary_minus),

        // ---- functions -----------------------------------------------------
        Expression::FunctionCall(function, args) => eval_function(function, args, row, schema, ctx),
    }
}

/// Evaluate `expr` and reduce it to an effective boolean value (`Ok(None)` =
/// error/unbound).
pub(crate) fn eval_ebv<D: DatasetView + Sync>(
    expr: &Expression,
    row: &[Option<SolutionTerm<D::Id>>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Option<bool>, EvalError> {
    ebv_of(expr, row, schema, ctx)
}

/// `Filter(expr, inner)`: keep solutions whose `expr` has effective boolean value
/// `true`; an error/unbound (or `false`) drops the row.
///
/// [`crate::parallel::is_parallel_safe`] gates the strategy: an expression that
/// can reach a stateful builtin (`RAND`/`UUID`/`STRUUID`/`BNODE`, the PurRDF list
/// constructors) MUST run on the real `ctx` sequentially, so its per-query
/// counter/RNG state advances exactly as it would without this parallel path — a
/// forked child would advance a throwaway copy instead, silently diverging from
/// the sequential result. A safe expression only decides keep/drop; the
/// surviving rows are the ORIGINAL rows (never a value derived from the child's
/// scratch), so each forked child's scratch is discarded after use — nothing to
/// re-intern via [`crate::parallel::reintern_minted_row`].
///
/// # Under a truncated child
///
/// A `FILTER`'s own data child is prefix-monotone: the predicate is evaluated in full
/// over every row that reaches it and no surviving row moves, so filtering a prefix
/// yields a prefix. A truncation **inside an `EXISTS` in the predicate** is a different
/// matter — it is an opaque edge, because a truncated `EXISTS` inner bag drops rows the
/// true query keeps and a truncated `NOT EXISTS` inner bag fabricates rows outright — so
/// the whole output is withheld and only the barrier crosses.
pub(crate) fn eval_filter<D: DatasetView + Sync>(
    node: &GraphPattern,
    expr: &Expression,
    inner: &GraphPattern,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    let mut lift = Lift::at(node);
    let Some(mut seq) = lift.absorb(0, eval_evaluated(inner, ctx)?) else {
        return Ok(lift.withheld());
    };
    let schema = seq.schema.clone();
    // The `row-expression-evaluation` charge point, charged **per row** rather than per
    // sub-expression so that the cost of a `FILTER` is a property of the data it sees
    // and not of how the planner happened to shape the predicate. Charged before the
    // predicate runs and against the rows in source order, so the admitted set is a
    // positional prefix and the refused rows are never evaluated at all — a governor
    // that only reported the cost after paying it would bound nothing. The trip itself
    // is latched in the governor state, which is where `eval` reads it to certify this
    // node's output.
    let _ = ctx.admit_rows(
        &mut seq.rows,
        crate::governor::ChargePoint::RowExpressionEvaluation,
    );
    let rows = if ctx.may_fork_row_loop(expr) {
        crate::parallel::par_chunk_try_map_init(
            &seq.rows,
            || ctx.fork_for_worker(),
            |child, acc, row| {
                if eval_ebv(expr, row, &schema, child)? == Some(true) {
                    acc.push(row.clone());
                }
                Ok(())
            },
        )?
    } else {
        let mut rows = Vec::new();
        for row in seq.rows {
            if eval_ebv(expr, &row, &schema, ctx)? == Some(true) {
                rows.push(row);
            }
        }
        rows
    };
    if let Some(tripped) = ctx.expression_barrier.observed() {
        return Ok(Evaluated::Truncated(Truncation::barred_at(
            node,
            tripped,
            schema.clone(),
        )));
    }
    Ok(lift.finish(SolutionSeq { schema, rows }))
}

/// `Extend(inner, var, expr)` (BIND): add `var` bound to `expr`'s value for each
/// solution. An error/unbound value leaves `var` unbound (the row is NOT dropped).
///
/// Gated on [`crate::parallel::is_parallel_safe`] like `eval_filter`: an unsafe
/// `expr` MUST run on the real `ctx` sequentially. A safe `expr` mints a NEW
/// `Computed` term that escapes into the output row (unlike FILTER's read-only
/// predicate), so each worker's forked child materializes its bound row via
/// [`crate::parallel::portable_row`] while its scratch is still alive, and this
/// function re-interns each portable row against `ctx.scratch` afterwards, in
/// source-index order, via [`crate::parallel::reintern_portable_row`].
pub(crate) fn eval_extend<D: DatasetView + Sync>(
    node: &GraphPattern,
    inner: &GraphPattern,
    var: &Variable,
    expr: &Expression,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Evaluated<D::Id>, EvalError> {
    let mut lift = Lift::at(node);
    let Some(mut seq) = lift.absorb(0, eval_evaluated(inner, ctx)?) else {
        let mut schema = lift.absorbed_schema().map_or_else(
            || (*crate::eval::syntactic_schema(inner)).clone(),
            |s| (*s).clone(),
        );
        schema.push(var.clone());
        return Ok(lift.finish(SolutionSeq::empty(Arc::new(schema))));
    };
    // The `row-expression-evaluation` charge point; see `eval_filter` for why it is per
    // row rather than per sub-expression, and why the refused rows are cut before the
    // expression runs rather than after.
    let _ = ctx.admit_rows(
        &mut seq.rows,
        crate::governor::ChargePoint::RowExpressionEvaluation,
    );
    let mut schema = (*seq.schema).clone();
    let col = schema.push(var.clone());
    let width = schema.len();
    let schema = Arc::new(schema);

    let rows = if ctx.may_fork_row_loop(expr) {
        // Parallel path: `is_parallel_safe` excludes `BNODE` (every arity), so the
        // per-solution `BNODE(strExpr)` memo (`ctx.current_row`/`ctx.bnode_memo`) is
        // never observed here — no per-row `current_row` bookkeeping is needed.
        let base = ctx.scratch.computed_count();
        let minted = crate::parallel::par_chunk_try_map_init(
            &seq.rows,
            || ctx.fork_for_worker(),
            |child, acc, in_row| {
                let mut row = in_row.clone();
                row.resize(width, None);
                let value = eval_expr(expr, &row, &schema, child)?;
                row[col] = value;
                acc.push(crate::parallel::minted_row(&child.scratch, base, row));
                Ok(())
            },
        )?;
        minted
            .into_iter()
            .map(|row| crate::parallel::reintern_minted_row(&mut ctx.scratch, ctx.dataset, row))
            .collect()
    } else {
        let mut rows = Vec::with_capacity(seq.rows.len());
        for (idx, mut row) in seq.rows.into_iter().enumerate() {
            row.resize(width, None);
            // §17.4.2.2: BNODE(strExpr) memoizes per solution — see `ctx.current_row`'s
            // doc. This Extend maps `seq`'s rows 1:1 in order, so the row's position
            // here matches its position in every other Extend of the same chain. A
            // BNODE-bearing `expr` is exactly what forces this sequential branch.
            ctx.current_row = idx as u64;
            let value = eval_expr(expr, &row, &schema, ctx)?;
            row[col] = value;
            rows.push(row);
        }
        rows
    };
    // An `EXISTS` inside the bound expression is an opaque edge; see `eval_filter`.
    if let Some(tripped) = ctx.expression_barrier.observed() {
        return Ok(Evaluated::Truncated(Truncation::barred_at(
            node,
            tripped,
            schema.clone(),
        )));
    }
    Ok(lift.finish(SolutionSeq { schema, rows }))
}

// ---------------------------------------------------------------------------
// internals
// ---------------------------------------------------------------------------

/// Look up a variable's binding in a solution.
fn lookup<I: ViewTermId>(
    var: &Variable,
    row: &[Option<SolutionTerm<I>>],
    schema: &VarSchema,
) -> Option<SolutionTerm<I>> {
    schema.index_of(var).and_then(|c| row[c])
}

/// Intern a value to a solution term (promoting to an existing dataset id).
fn intern<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    value: TermValue,
) -> SolutionTerm<D::Id> {
    ctx.scratch.intern(ctx.dataset, value)
}

/// Intern a constant atom (`NamedNode`/`Literal`), memoized per query by the
/// node's AST address (see [`EvalCtx::const_atom_cache`]). `build` — which owns
/// the `TermValue` allocation — runs only on a cache miss, so a FILTER/BIND over
/// N rows pays the `to_owned()` + intern probe once, not N times.
fn const_atom<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    expr: &Expression,
    build: impl FnOnce() -> TermValue,
) -> SolutionTerm<D::Id> {
    // Address-keyed memoization is unsound over a per-row substituted-EXISTS
    // temporary (see `EvalCtx::in_substituted_exists`): the node's address can
    // be a dropped-and-reused allocation from an earlier outer row, so a hit
    // here would silently return a stale, wrong-row constant. Bypass the cache
    // entirely for the duration of that window.
    if ctx.in_substituted_exists {
        return intern(ctx, build());
    }
    let key = std::ptr::from_ref::<Expression>(expr) as usize;
    if let Some(term) = ctx.const_atom_cache.get(&key) {
        return *term;
    }
    let term = intern(ctx, build());
    ctx.const_atom_cache.insert(key, term);
    term
}

/// Materialize a solution term to an owned value.
fn value_of<D: DatasetView + Sync>(ctx: &EvalCtx<'_, D>, term: SolutionTerm<D::Id>) -> TermValue {
    ctx.scratch.value_of(ctx.dataset, term)
}

/// Intern an `xsd:boolean` literal.
///
/// The two boolean terms are resolved **once per [`EvalCtx`]** (lazily) and then
/// served from `cached_bool_terms`: a FILTER over N rows pays the value-hash
/// intern probe once, not N times. The cache is exact — interning is
/// deterministic for the context's pinned dataset and dedup-by-value scratch, so
/// the cached term is the same `SolutionTerm` a fresh intern would produce.
fn bool_term<D: DatasetView + Sync>(ctx: &mut EvalCtx<'_, D>, b: bool) -> SolutionTerm<D::Id> {
    let slot = usize::from(b);
    if let Some(term) = ctx.cached_bool_terms[slot] {
        return term;
    }
    let term = intern(ctx, typed(if b { "true" } else { "false" }, XSD_BOOLEAN));
    ctx.cached_bool_terms[slot] = Some(term);
    term
}

/// Intern an `xsd:string` literal.
fn string_term<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    lexical: &str,
) -> SolutionTerm<D::Id> {
    intern(ctx, typed(lexical, XSD_STRING))
}

/// Intern an `xsd:integer` literal.
fn integer_term<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    value: i64,
) -> SolutionTerm<D::Id> {
    intern(ctx, typed(&value.to_string(), XSD_INTEGER))
}

/// Build a typed (no-language) literal value.
fn typed(lexical: &str, datatype: &str) -> TermValue {
    TermValue::Literal {
        lexical_form: lexical.to_owned(),
        datatype: datatype.to_owned(),
        language: None,
        direction: None,
    }
}

/// The XSD value of a term, if it is an XSD-typed literal; `None` otherwise
/// (non-literal, unknown datatype, or malformed lexical form).
pub(crate) fn xsd_of(value: &TermValue) -> Option<XsdValue> {
    if let TermValue::Literal {
        lexical_form,
        datatype,
        ..
    } = value
    {
        parse_by_iri(lexical_form, datatype).ok().flatten()
    } else {
        None
    }
}

/// The effective boolean value of an evaluated expression (`Ok(None)` = error).
fn ebv_of<D: DatasetView + Sync>(
    expr: &Expression,
    row: &[Option<SolutionTerm<D::Id>>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Option<bool>, EvalError> {
    match eval_expr(expr, row, schema, ctx)? {
        Some(term) => Ok(ebv_term(ctx, term)),
        None => Ok(None),
    }
}

/// The effective boolean value of a concrete term (`None` = type error).
fn ebv_term<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    term: SolutionTerm<D::Id>,
) -> Option<bool> {
    // A language-tagged string (rdf:langString / rdf:dirLangString) has no effective
    // boolean value — EBV covers only xsd:string, xsd:boolean, and the numeric types.
    let is_lang_tagged = match term {
        SolutionTerm::Existing(id) => {
            matches!(
                ctx.dataset.resolve(id),
                TermRef::Literal {
                    language: Some(_),
                    ..
                }
            )
        }
        SolutionTerm::Computed(sid) => matches!(
            ctx.scratch.computed_value(sid),
            TermValue::Literal {
                language: Some(_),
                ..
            }
        ),
    };
    if is_lang_tagged {
        return None;
    }
    match xsd_of_term(ctx, term) {
        Some(xv) => effective_boolean_value(&xv),
        None => None,
    }
}

/// The XSD value of a solution term, resolved through **borrowed** views — a
/// [`TermRef`] for dataset terms, the scratch table for computed ones — so the
/// per-row comparison hot path parses without materializing an owned
/// [`TermValue`]. Semantically identical to `xsd_of(&value_of(ctx, term))`.
///
/// Dataset (`Existing`) parses are memoized per query by `TermId` (see
/// [`EvalCtx::xsd_parse_cache`]): the lexical form and datatype are immutable for a
/// fixed id, so a comparison/`FILTER` over N rows parses each distinct literal once
/// instead of once per row. Computed scratch values are ephemeral and stay on the
/// direct borrowed-view path.
fn xsd_of_term<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    term: SolutionTerm<D::Id>,
) -> Option<XsdValue> {
    match term {
        SolutionTerm::Existing(id) => {
            if let Some(cached) = ctx.xsd_parse_cache.get(&id) {
                return cached.clone();
            }
            let parsed = match ctx.dataset.resolve(id) {
                TermRef::Literal {
                    lexical, datatype, ..
                } => match ctx.dataset.resolve(datatype) {
                    TermRef::Iri(iri) => parse_by_iri(lexical, iri).ok().flatten(),
                    // A literal's datatype is always an interned IRI (C0.1).
                    _ => unreachable!("literal datatype must be an IRI"),
                },
                _ => None,
            };
            ctx.xsd_parse_cache.insert(id, parsed.clone());
            parsed
        }
        SolutionTerm::Computed(sid) => xsd_of(ctx.scratch.computed_value(sid)),
    }
}

/// Whether a solution term is a literal, checked on the borrowed view (no
/// materialization).
fn term_is_literal<D: DatasetView + Sync>(ctx: &EvalCtx<'_, D>, term: SolutionTerm<D::Id>) -> bool {
    match term {
        SolutionTerm::Existing(id) => {
            matches!(ctx.dataset.resolve(id), TermRef::Literal { .. })
        }
        SolutionTerm::Computed(sid) => {
            matches!(ctx.scratch.computed_value(sid), TermValue::Literal { .. })
        }
    }
}

/// Whether a solution term is a triple term, checked on the borrowed view (no
/// materialization) — mirrors [`term_is_literal`].
fn term_is_triple<D: DatasetView + Sync>(ctx: &EvalCtx<'_, D>, term: SolutionTerm<D::Id>) -> bool {
    match term {
        SolutionTerm::Existing(id) => {
            matches!(ctx.dataset.resolve(id), TermRef::Triple { .. })
        }
        SolutionTerm::Computed(sid) => {
            matches!(ctx.scratch.computed_value(sid), TermValue::Triple { .. })
        }
    }
}

/// Evaluate a comparison: both operands to values, compare in the XSD value space,
/// and test the resulting [`Ordering`] with `keep`. `None` (error/unbound operand
/// or incomparable values) propagates.
fn compare<D: DatasetView + Sync>(
    a: &Expression,
    b: &Expression,
    row: &[Option<SolutionTerm<D::Id>>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_, D>,
    keep: impl Fn(Ordering) -> bool,
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    let ta = eval_expr(a, row, schema, ctx)?;
    let tb = eval_expr(b, row, schema, ctx)?;
    let (Some(ta), Some(tb)) = (ta, tb) else {
        return Ok(None);
    };
    // sameTerm short-circuit: identical terms are equal regardless of value space.
    if ta == tb {
        return Ok(Some(bool_term(ctx, keep(Ordering::Equal))));
    }
    // Value-space comparison over borrowed term views (no owned TermValue
    // clones). Distinct non-value terms (IRIs/blanks) or incomparable value
    // spaces are a type error (`None`), exactly as before. Each side is parsed
    // through the per-query id→XSD memo; the two calls are sequenced (not a tuple
    // literal) because each takes `&mut ctx`.
    let ax = xsd_of_term(ctx, ta);
    let bx = xsd_of_term(ctx, tb);
    let ord = match (ax, bx) {
        (Some(ax), Some(bx)) => value_cmp(&ax, &bx),
        _ => None,
    };
    Ok(ord.map(|ord| bool_term(ctx, keep(ord))))
}

/// Evaluate `a = b` under SPARQL RDF-term equality (SPARQL 1.2 §17.4.2.2
/// `sameValue`, which "replaces `RDFterm-equal` from SPARQL 1.1" — same
/// question, current name): both operands resolve to a term, identical terms
/// are equal, value-comparable literals compare in the XSD value space
/// ([`sparql_value_eq`], including the `sameValue`-only cross-type NaN
/// carve-out its docs explain), distinct terms where at least one is a
/// non-literal (IRI/blank) are **unequal** (`false`, NOT a type error), and two
/// incomparable literals are a type error (`None`). This is the equality companion to
/// the ordering [`compare`]; using `compare` for `=` would wrongly turn a distinct
/// IRI pair into an error. Note that `sameValue` "cannot be used directly in
/// expressions" (its own spec text) — it names the semantics `=` embeds, not a
/// callable SPARQL function, so there is no `Function::SameValue` parser/algebra
/// arm to add; see `crate::basic_profile`'s module docs for where that
/// distinction is recorded against the Basic-profile survey.
fn equal<D: DatasetView + Sync>(
    a: &Expression,
    b: &Expression,
    row: &[Option<SolutionTerm<D::Id>>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    let ta = eval_expr(a, row, schema, ctx)?;
    let tb = eval_expr(b, row, schema, ctx)?;
    let (Some(ta), Some(tb)) = (ta, tb) else {
        return Ok(None);
    };
    // sameTerm short-circuit: identical terms are equal regardless of value space.
    if ta == tb {
        return Ok(Some(bool_term(ctx, true)));
    }
    // Distinct `SolutionTerm`s are distinct RDF terms BY CONSTRUCTION: the dataset
    // builder interns terms by value (one id per value, table kept as-is at
    // freeze), the scratch interner dedups by value, and the promotion rule makes
    // an Existing/Computed cross-pair unequal in value. So `rdf_equal`'s
    // structural `a == b` fallback can never fire once `ta != tb`; only the
    // value-space comparison, the triple-term recursion, and the literal/
    // non-literal split remain — evaluated here on borrowed views (no owned
    // `TermValue` clones) EXCEPT for the triple-term case, which materializes
    // both sides to recurse componentwise (RDF 1.2 `op-2`: triple terms compare
    // structurally under `=`, not sameTerm-or-unequal).
    if term_is_triple(ctx, ta) && term_is_triple(ctx, tb) {
        let av = value_of(ctx, ta);
        let bv = value_of(ctx, tb);
        return Ok(rdf_equal(&av, &bv).map(|eq| bool_term(ctx, eq)));
    }
    let ax = xsd_of_term(ctx, ta);
    let bx = xsd_of_term(ctx, tb);
    let eq = match (ax, bx) {
        (Some(ax), Some(bx)) => sparql_value_eq(&ax, &bx),
        _ => {
            if term_is_literal(ctx, ta) && term_is_literal(ctx, tb) {
                // Two different literals neither side could value-compare.
                None
            } else {
                // Distinct terms of (at least one) non-literal kind: known unequal.
                Some(false)
            }
        }
    };
    Ok(eq.map(|eq| bool_term(ctx, eq)))
}

/// `expr IN (list)`: true if equal (value semantics) to any list entry; an error in
/// the list propagates only if no `true` is found (SPARQL §17.4.1.9).
fn eval_in<D: DatasetView + Sync>(
    needle: &Expression,
    haystack: &[Expression],
    row: &[Option<SolutionTerm<D::Id>>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    let Some(target) = eval_expr(needle, row, schema, ctx)? else {
        return Ok(None);
    };
    let tv = value_of(ctx, target);
    let mut saw_error = false;
    for item in haystack {
        match eval_expr(item, row, schema, ctx)? {
            Some(candidate) => {
                if target == candidate {
                    return Ok(Some(bool_term(ctx, true)));
                }
                let cv = value_of(ctx, candidate);
                match rdf_equal(&tv, &cv) {
                    Some(true) => return Ok(Some(bool_term(ctx, true))),
                    Some(false) => {}
                    None => saw_error = true,
                }
            }
            None => saw_error = true,
        }
    }
    if saw_error {
        Ok(None)
    } else {
        Ok(Some(bool_term(ctx, false)))
    }
}

/// RDF term value-equality (`=`). `None` = type error (two literals not comparable).
fn rdf_equal(a: &TermValue, b: &TermValue) -> Option<bool> {
    // RDF 1.2 triple terms compare *structurally*, componentwise, under this SAME
    // `=` relation (recursively) — not by identity. Checked before the XSD/literal
    // path so a triple-term pair never falls through to the "distinct non-literal
    // kind ⇒ unequal" default, which would wrongly treat e.g. `<<(:a :b 123)>>` and
    // `<<(:a :b 123.0)>>` as unequal even though `123 = 123.0` in the XSD value
    // space (W3C `eval-triple-terms` `op-2`).
    if let (
        TermValue::Triple {
            s: s1,
            p: p1,
            o: o1,
        },
        TermValue::Triple {
            s: s2,
            p: p2,
            o: o2,
        },
    ) = (a, b)
    {
        return triple_equal(s1, p1, o1, s2, p2, o2);
    }
    match (xsd_of(a), xsd_of(b)) {
        (Some(ax), Some(bx)) => sparql_value_eq(&ax, &bx),
        _ => {
            if a == b {
                Some(true)
            } else if is_literal(a) && is_literal(b) {
                // Two different literals neither side could value-compare.
                None
            } else {
                // Distinct terms of (at least one) non-literal kind: known unequal.
                Some(false)
            }
        }
    }
}

/// Whether `x` is the `xsd:double`/`xsd:float` NaN value.
fn is_xsd_nan(x: &XsdValue) -> bool {
    matches!(x, XsdValue::Double(d) if d.is_nan()) || matches!(x, XsdValue::Float(f) if f.is_nan())
}

/// `=` / `sameValue` equality between two already-typed XSD values (SPARQL 1.2
/// §17.4.2.2 `sameValue`, which "replaces `RDFterm-equal` from SPARQL 1.1"):
/// [`value_cmp`]'s value-space comparison, EXCEPT for one carve-out `sameValue`
/// states explicitly and `value_cmp` cannot: *"`NaN`^^xsd:double and
/// `NaN`^^xsd:float are considered to represent the same value. If term1 and
/// term2 are both `NaN` for either xsd:double or xsd:float, then return TRUE."*
/// This fires even ACROSS the two types — `"NaN"^^xsd:double = "NaN"^^xsd:float`
/// is `true` — which the ordinary numeric-tower promotion in [`value_cmp`]
/// cannot answer on its own, since `f64::partial_cmp` (and its `f32` sibling)
/// treats NaN as unordered by IEEE 754 design, exactly as `value_cmp` should
/// keep doing for `<`/`>`/`ORDER BY`: the carve-out is `sameValue`'s alone, so
/// it lives here rather than in `value_cmp` itself. `same-type` NaN pairs
/// (`double`/`double` or `float`/`float`) already answer `true` one level up,
/// via [`equal`]'s/[`rdf_equal`]'s identical-RDF-term short-circuit — NaN's
/// canonical lexical form is always `"NaN"`, so two same-typed NaN literals ARE
/// the same RDF term before this function is ever reached (`sameValue` step 1)
/// — this function is what the CROSS-type pair needs, since two literals with
/// different datatype IRIs are never the same RDF term regardless of value.
pub(crate) fn sparql_value_eq(ax: &XsdValue, bx: &XsdValue) -> Option<bool> {
    if is_xsd_nan(ax) && is_xsd_nan(bx) {
        return Some(true);
    }
    value_equal(ax, bx)
}

/// Componentwise `=` over two triple terms (SPARQL §17.4.1.7 extended to RDF 1.2
/// triple terms): equal iff subject, predicate, and object are pairwise `=`-equal.
/// A component that is definitely unequal short-circuits the whole comparison to
/// `false` (even if another component errored); otherwise any component error
/// propagates as an error (`None`).
fn triple_equal(
    s1: &TermValue,
    p1: &TermValue,
    o1: &TermValue,
    s2: &TermValue,
    p2: &TermValue,
    o2: &TermValue,
) -> Option<bool> {
    let rs = rdf_equal(s1, s2);
    let rp = rdf_equal(p1, p2);
    let ro = rdf_equal(o1, o2);
    if rs == Some(false) || rp == Some(false) || ro == Some(false) {
        Some(false)
    } else if rs.is_none() || rp.is_none() || ro.is_none() {
        None
    } else {
        Some(true)
    }
}

fn is_literal(v: &TermValue) -> bool {
    matches!(v, TermValue::Literal { .. })
}

/// Collect all [`Variable`]s referenced inside expression positions within `expr`.
/// This is a pure syntactic walk of the [`Expression`] tree; it returns every
/// variable that appears in a position where it is *evaluated* (not just matched
/// as a triple-pattern term).
fn expr_vars(expr: &Expression, out: &mut DetHashSet<Variable>) {
    match expr {
        Expression::Variable(v) | Expression::Bound(v) => {
            out.insert(v.clone());
        }
        Expression::NamedNode(_) | Expression::Literal(_) => {}
        Expression::Or(a, b)
        | Expression::And(a, b)
        | Expression::Equal(a, b)
        | Expression::SameTerm(a, b)
        | Expression::Greater(a, b)
        | Expression::GreaterOrEqual(a, b)
        | Expression::Less(a, b)
        | Expression::LessOrEqual(a, b)
        | Expression::Add(a, b)
        | Expression::Subtract(a, b)
        | Expression::Multiply(a, b)
        | Expression::Divide(a, b) => {
            expr_vars(a, out);
            expr_vars(b, out);
        }
        Expression::UnaryPlus(a) | Expression::UnaryMinus(a) | Expression::Not(a) => {
            expr_vars(a, out);
        }
        Expression::If(c, t, e) => {
            expr_vars(c, out);
            expr_vars(t, out);
            expr_vars(e, out);
        }
        Expression::In(needle, haystack) => {
            expr_vars(needle, out);
            for h in haystack {
                expr_vars(h, out);
            }
        }
        Expression::Coalesce(items) => {
            for item in items {
                expr_vars(item, out);
            }
        }
        Expression::FunctionCall(_, args) => {
            for a in args {
                expr_vars(a, out);
            }
        }
        // Nested EXISTS: walk the expression positions inside its inner pattern too.
        Expression::Exists(inner_pat) => {
            pattern_expr_vars(inner_pat, out);
        }
    }
}

/// Collect the variables a term pattern mentions, descending into a quoted triple's
/// own component positions.
fn term_pattern_vars(term: &purrdf_sparql_algebra::TermPattern, out: &mut DetHashSet<Variable>) {
    use purrdf_sparql_algebra::{NamedNodePattern, TermPattern};

    match term {
        TermPattern::Variable(variable) => {
            out.insert(variable.clone());
        }
        TermPattern::Triple(triple) => {
            term_pattern_vars(&triple.subject, out);
            if let NamedNodePattern::Variable(variable) = &triple.predicate {
                out.insert(variable.clone());
            }
            term_pattern_vars(&triple.object, out);
        }
        TermPattern::NamedNode(_) | TermPattern::BlankNode(_) | TermPattern::Literal(_) => {}
    }
}

/// Collect all variables referenced in *expression* positions within `pattern`.
///
/// Expression positions are: `Filter` conditions, `Extend`/BIND expressions,
/// `LeftJoin` inline filter conditions, `OrderBy` sort-key expressions, `Group`
/// grouping-key expressions and aggregate sub-expressions. Variables that appear
/// only as triple-pattern terms (subject/predicate/object) are NOT included here
/// because they are constrained by the standard join, not by expression evaluation.
fn pattern_expr_vars(pattern: &GraphPattern, out: &mut DetHashSet<Variable>) {
    match pattern {
        // Leaf nodes with no expression positions.
        GraphPattern::Bgp { .. } | GraphPattern::Path { .. } | GraphPattern::Values { .. } => {}

        // A property function's arguments are INVOCATION INPUTS: the relation is
        // called with whatever the row binds them to, and it produces rows from that.
        // They are therefore expression-like, NOT join-constrained the way a triple
        // term is — so every argument variable is reported here. Under-reporting would
        // send a correlated `EXISTS` whose inner pattern calls a relation with an
        // outer-bound argument down the UNCORRELATED path, where the inner result is
        // evaluated once and reused across outer rows: the relation would be invoked
        // with the first row's arguments and every later row would read that answer.
        GraphPattern::PropertyFunction(call) => {
            for term in call.subject_args.iter().chain(&call.object_args) {
                term_pattern_vars(term, out);
            }
        }

        // Single-child wrappers with no expressions of their own.
        GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. }
        | GraphPattern::Project { inner, .. }
        | GraphPattern::Graph { inner, .. }
        | GraphPattern::Service { inner, .. } => {
            pattern_expr_vars(inner, out);
        }

        // Two-child operators with no expressions of their own.
        GraphPattern::Join { left, right }
        | GraphPattern::Union { left, right }
        | GraphPattern::Minus { left, right }
        | GraphPattern::Lateral { left, right } => {
            pattern_expr_vars(left, out);
            pattern_expr_vars(right, out);
        }

        // Filter: the condition is an expression — walk it, then recurse into inner.
        GraphPattern::Filter { expr, inner } => {
            expr_vars(expr, out);
            pattern_expr_vars(inner, out);
        }

        // Extend / BIND: the bound expression is evaluated.
        GraphPattern::Extend {
            inner, expression, ..
        } => {
            expr_vars(expression, out);
            pattern_expr_vars(inner, out);
        }

        // LeftJoin: the optional inline filter condition is evaluated.
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => {
            if let Some(e) = expression {
                expr_vars(e, out);
            }
            pattern_expr_vars(left, out);
            pattern_expr_vars(right, out);
        }

        // OrderBy: sort keys are expressions.
        GraphPattern::OrderBy { inner, expression } => {
            for ord_expr in expression {
                match ord_expr {
                    purrdf_sparql_algebra::OrderExpression::Asc(e)
                    | purrdf_sparql_algebra::OrderExpression::Desc(e) => expr_vars(e, out),
                }
            }
            pattern_expr_vars(inner, out);
        }

        // Group: grouping-key expressions and aggregate sub-expressions are evaluated.
        GraphPattern::Group {
            inner,
            variables: _,
            aggregates,
        } => {
            for (_, agg) in aggregates {
                for arg in agg.args() {
                    expr_vars(arg, out);
                }
            }
            pattern_expr_vars(inner, out);
        }
    }
}

/// Whether `pattern` contains, anywhere in its tree, a node whose EVALUATED
/// ANSWER depends on more than the unconstrained relation it computes over —
/// `Lateral`, `Slice`, `Distinct`, `Reduced`, `Minus`, and `Group` all qualify:
/// each can give a DIFFERENT answer depending on which specific outer row
/// drove the evaluation that produced its input (a `LIMIT`, an aggregate, or
/// `MINUS`'s own domain-disjointness test all read the shape of the row, not
/// just whether it exists). `Bgp`/`Path`/`Values`/an ordinary `Join`/`Union`
/// are NOT row-sensitive: evaluating them once, unconstrained, and probing the
/// result for compatibility (the memoized fast path in [`exists`]) is sound
/// for them regardless of which outer row is being tested.
///
/// This is what makes SEP-0006's `LATERAL` reachable inside `FILTER EXISTS`
/// through a bare TRIPLE position (no expression position at all) a
/// correctness hazard the fast path cannot see on its own:
/// [`pattern_expr_vars`] only reports variables occurring in expression
/// positions, so a `LATERAL` correlated solely through a shared triple-pattern
/// variable was invisible to the OLD correlation test — this predicate, paired
/// with [`pattern_all_vars`], is the widened test. A bare `Project` (a plain
/// sub-`SELECT` with no `LIMIT`/`DISTINCT`/`GROUP BY` of its own) is a pure
/// column restriction and is NOT independently row-sensitive; wherever a
/// `Project` sits beneath one of the six listed constructs, that construct's
/// own arm already reports `true` for the whole subtree.
fn is_row_sensitive(pattern: &GraphPattern) -> bool {
    match pattern {
        GraphPattern::Lateral { .. }
        | GraphPattern::Slice { .. }
        | GraphPattern::Distinct { .. }
        | GraphPattern::Reduced { .. }
        | GraphPattern::Minus { .. }
        | GraphPattern::Group { .. } => true,
        GraphPattern::Bgp { .. }
        | GraphPattern::Path { .. }
        | GraphPattern::Values { .. }
        | GraphPattern::PropertyFunction(_) => false,
        GraphPattern::Join { left, right } | GraphPattern::Union { left, right } => {
            is_row_sensitive(left) || is_row_sensitive(right)
        }
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => {
            is_row_sensitive(left)
                || is_row_sensitive(right)
                || expression.as_ref().is_some_and(expr_is_row_sensitive)
        }
        GraphPattern::Filter { expr, inner } => {
            is_row_sensitive(inner) || expr_is_row_sensitive(expr)
        }
        GraphPattern::Extend {
            inner, expression, ..
        } => is_row_sensitive(inner) || expr_is_row_sensitive(expression),
        GraphPattern::OrderBy { inner, expression } => {
            is_row_sensitive(inner)
                || expression.iter().any(|oe| match oe {
                    purrdf_sparql_algebra::OrderExpression::Asc(e)
                    | purrdf_sparql_algebra::OrderExpression::Desc(e) => expr_is_row_sensitive(e),
                })
        }
        GraphPattern::Project { inner, .. }
        | GraphPattern::Graph { inner, .. }
        | GraphPattern::Service { inner, .. } => is_row_sensitive(inner),
    }
}

/// Whether any [`Expression::Exists`] reachable within `expr` (at any nesting
/// depth through the boolean/arithmetic combinators) wraps an inner pattern
/// [`is_row_sensitive`] itself judges row-sensitive.
///
/// Mirrors [`expr_vars`]'s recursion structure exactly (same combinators, same
/// `Expression::Exists` seam) so this walk and the variable walk never diverge
/// on which expression positions exist. Needed because [`is_row_sensitive`]'s
/// pattern walk mirrors [`pattern_all_vars`]'s node coverage (`Filter`,
/// `Extend`, `OrderBy`, `Group` aggregates, `LeftJoin`'s condition) — a
/// row-sensitive node (`Lateral`+`LIMIT`, etc.) reachable ONLY inside a nested
/// `EXISTS`/`NOT EXISTS` expression must still mark its enclosing pattern
/// row-sensitive, or a correlated outer `EXISTS` wrongly takes the
/// evaluate-once-and-probe fast path.
fn expr_is_row_sensitive(expr: &Expression) -> bool {
    match expr {
        Expression::Variable(_) | Expression::Bound(_) => false,
        Expression::NamedNode(_) | Expression::Literal(_) => false,
        Expression::Or(a, b)
        | Expression::And(a, b)
        | Expression::Equal(a, b)
        | Expression::SameTerm(a, b)
        | Expression::Greater(a, b)
        | Expression::GreaterOrEqual(a, b)
        | Expression::Less(a, b)
        | Expression::LessOrEqual(a, b)
        | Expression::Add(a, b)
        | Expression::Subtract(a, b)
        | Expression::Multiply(a, b)
        | Expression::Divide(a, b) => expr_is_row_sensitive(a) || expr_is_row_sensitive(b),
        Expression::UnaryPlus(a) | Expression::UnaryMinus(a) | Expression::Not(a) => {
            expr_is_row_sensitive(a)
        }
        Expression::If(c, t, e) => {
            expr_is_row_sensitive(c) || expr_is_row_sensitive(t) || expr_is_row_sensitive(e)
        }
        Expression::In(needle, haystack) => {
            expr_is_row_sensitive(needle) || haystack.iter().any(expr_is_row_sensitive)
        }
        Expression::Coalesce(items) => items.iter().any(expr_is_row_sensitive),
        Expression::FunctionCall(_, args) => args.iter().any(expr_is_row_sensitive),
        // The seam this walk exists for: a nested EXISTS's inner pattern can itself
        // be row-sensitive (or carry a further-nested EXISTS that is).
        Expression::Exists(inner_pat) => is_row_sensitive(inner_pat),
    }
}

/// Collect EVERY variable `pattern` mentions anywhere — triple/path terms,
/// `VALUES` columns, `GRAPH`/`SERVICE` names, property-function arguments, and
/// every expression position [`pattern_expr_vars`] already covers — plus every
/// variable a construct itself INTRODUCES (`BIND`/`GROUP BY`/a projection
/// list), which is a safe over-approximation for this predicate's one use:
/// widened `EXISTS` correlation detection ([`is_row_sensitive`]'s doc). Used
/// ONLY for a row-sensitive inner pattern, where an outer-bound variable
/// occurring in ANY position — not just an expression one — makes the fast
/// (evaluate-once, probe-per-row) path unsound, so the substituted per-row
/// path must run instead.
fn pattern_all_vars(pattern: &GraphPattern, out: &mut DetHashSet<Variable>) {
    use purrdf_sparql_algebra::NamedNodePattern;

    match pattern {
        GraphPattern::Bgp { patterns } => {
            for tp in patterns {
                term_pattern_vars(&tp.subject, out);
                if let NamedNodePattern::Variable(v) = &tp.predicate {
                    out.insert(v.clone());
                }
                term_pattern_vars(&tp.object, out);
            }
        }
        GraphPattern::Path {
            subject, object, ..
        } => {
            term_pattern_vars(subject, out);
            term_pattern_vars(object, out);
        }
        GraphPattern::Values { variables, .. } => {
            out.extend(variables.iter().cloned());
        }
        GraphPattern::PropertyFunction(call) => {
            for term in call.subject_args.iter().chain(&call.object_args) {
                term_pattern_vars(term, out);
            }
        }
        GraphPattern::Graph { name, inner } => {
            if let NamedNodePattern::Variable(v) = name {
                out.insert(v.clone());
            }
            pattern_all_vars(inner, out);
        }
        GraphPattern::Service { name, inner, .. } => {
            if let NamedNodePattern::Variable(v) = name {
                out.insert(v.clone());
            }
            pattern_all_vars(inner, out);
        }
        GraphPattern::Join { left, right }
        | GraphPattern::Union { left, right }
        | GraphPattern::Minus { left, right }
        | GraphPattern::Lateral { left, right } => {
            pattern_all_vars(left, out);
            pattern_all_vars(right, out);
        }
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => {
            if let Some(e) = expression {
                expr_vars(e, out);
            }
            pattern_all_vars(left, out);
            pattern_all_vars(right, out);
        }
        GraphPattern::Filter { expr, inner } => {
            expr_vars(expr, out);
            pattern_all_vars(inner, out);
        }
        GraphPattern::Extend {
            inner,
            variable,
            expression,
        } => {
            out.insert(variable.clone());
            expr_vars(expression, out);
            pattern_all_vars(inner, out);
        }
        GraphPattern::OrderBy { inner, expression } => {
            for oe in expression {
                match oe {
                    purrdf_sparql_algebra::OrderExpression::Asc(e)
                    | purrdf_sparql_algebra::OrderExpression::Desc(e) => expr_vars(e, out),
                }
            }
            pattern_all_vars(inner, out);
        }
        GraphPattern::Group {
            inner,
            variables,
            aggregates,
        } => {
            out.extend(variables.iter().cloned());
            for (v, agg) in aggregates {
                out.insert(v.clone());
                for arg in agg.args() {
                    expr_vars(arg, out);
                }
            }
            pattern_all_vars(inner, out);
        }
        GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. } => pattern_all_vars(inner, out),
        GraphPattern::Project { inner, variables } => {
            out.extend(variables.iter().cloned());
            pattern_all_vars(inner, out);
        }
    }
}

/// Evaluate `EXISTS { pattern }` for the current solution.
///
/// Two evaluation paths are used depending on whether any outer-bound variable
/// appears in an expression position (FILTER condition, BIND expression, etc.)
/// inside the inner pattern:
///
/// **Uncorrelated path** (fast): the inner pattern result is independent of which
/// outer row is being tested, so it can be evaluated once and cached. The outer
/// row's bindings are substituted via a seed-join with the memoized inner result.
/// This is the common case and preserves the performance win of evaluating the
/// inner pattern once per EXISTS site rather than once per outer row.
///
/// **Expression-correlated path** (correct per-row): when an outer-bound variable
/// is referenced inside an expression context in the inner pattern (e.g. a FILTER
/// that references an outer variable), evaluating the inner pattern unconstrained
/// would leave that variable unbound, causing the expression to error and drop
/// rows incorrectly. In this case the inner pattern is evaluated with the outer
/// row's bound variables pre-seeded as a VALUES-like leading input, so they are
/// visible as bound during expression evaluation. This result is NOT memoized
/// because it depends on the specific outer row.
fn exists<D: DatasetView + Sync>(
    pattern: &GraphPattern,
    row: &[Option<SolutionTerm<D::Id>>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<bool, EvalError> {
    // Build the set of outer-bound variables (those with a concrete binding in
    // the current row), then check if any of them are referenced in expression
    // positions inside the inner pattern.
    let outer_bound: DetHashSet<Variable> = schema
        .vars()
        .iter()
        .enumerate()
        .filter_map(|(i, v)| {
            if row[i].is_some() {
                Some(v.clone())
            } else {
                None
            }
        })
        .collect();

    // `pattern`'s address is a sound cache key only for the static query algebra.
    // While evaluating a per-row substituted-EXISTS temporary
    // (`ctx.in_substituted_exists`), `pattern` is itself such a temporary — its
    // address can be a dropped-and-reused allocation from an earlier outer row —
    // so skip both the cache read and the write and compute the var set fresh.
    let inner_expr_vars = if ctx.in_substituted_exists {
        let mut vars = DetHashSet::default();
        pattern_expr_vars(pattern, &mut vars);
        Arc::new(vars)
    } else {
        let pattern_key = std::ptr::from_ref::<GraphPattern>(pattern) as usize;
        ctx.exists_expr_vars_cache
            .entry(pattern_key)
            .or_insert_with(|| {
                let mut vars = DetHashSet::default();
                pattern_expr_vars(pattern, &mut vars);
                Arc::new(vars)
            })
            .clone()
    };

    // Widened per SEP-0006 reachability (see `is_row_sensitive`'s doc): an
    // outer-bound variable occurring ONLY in a triple position (never an
    // expression position) still needs the per-row substituted path whenever
    // the inner pattern is row-sensitive — evaluating a `Lateral`/`Slice`/
    // `Distinct`/`Reduced`/`Minus`/`Group` once, unconstrained, and probing
    // per outer row is sound only when the inner's answer does not itself
    // depend on which row drove it, and each of those constructs' answer CAN.
    // `LATERAL`'s parser is what first makes such an inner reachable through a
    // bare triple position inside `EXISTS` (previously unwritable surface
    // syntax). The row-sensitivity/all-vars walk is deliberately NOT cached
    // (unlike `inner_expr_vars` above): row-sensitive inners are the rare
    // case, so the memoized fast path below is what carries the common-case
    // performance, not this check.
    let is_expression_correlated = inner_expr_vars.iter().any(|v| outer_bound.contains(v))
        || (is_row_sensitive(pattern) && {
            let mut all_vars = DetHashSet::default();
            pattern_all_vars(pattern, &mut all_vars);
            all_vars.iter().any(|v| outer_bound.contains(v))
        });

    if is_expression_correlated {
        // Correct per-row path: substitute the outer row's bound variable values
        // into the inner pattern before evaluating (expression positions per
        // SPARQL §18.6; triple/leaf positions via Values Insertion — see
        // `crate::binop::eval_correlated`, this crate's one substitution seam).
        //
        // After substitution, the resulting pattern's result is specific to this
        // outer row; it is NOT memoized.
        let inner = crate::binop::eval_correlated(pattern, row, schema, ctx)?;
        if let Evaluated::Truncated(truncation) = &inner {
            // A truncated `EXISTS` inner bag can only turn its boolean from true to
            // false, and a truncated `NOT EXISTS` one from false to true — a fabricated
            // row. Neither is a bound, so the trip is reported to the enclosing operator,
            // which withholds its whole output. The boolean returned here is therefore
            // never observed in any row that reaches a caller.
            ctx.expression_barrier.record(truncation.tripped());
            return Ok(false);
        }
        Ok(!inner.rows().is_empty())
    } else {
        // Fast memoized path: the inner pattern result is independent of the outer
        // row's values in expression contexts, so evaluate it — and build the probe
        // index over it — ONCE per site (keyed by inner-pattern, graph, and outer
        // schema), then existence-probe each outer row against the reused index. This
        // replaces the former per-row seed-join, whose `join_seqs` rebuilt the inner
        // hash index on every outer row (O(rows × |inner|)).
        // As above: `pattern`'s address is a sound cache key only for the static
        // query algebra. A doubly-nested EXISTS reached while
        // `ctx.in_substituted_exists` is already set means `pattern` is itself
        // part of an outer per-row substituted temporary, so skip both the cache
        // get and the insert and build the entry fresh, unshared.
        let key = (
            std::ptr::from_ref::<GraphPattern>(pattern) as usize,
            ctx.graph_key(),
            crate::eval::schema_fingerprint(schema),
        );
        let cached = if ctx.options.exists_memo && !ctx.in_substituted_exists {
            ctx.exists_inner_cache.get(&key).cloned()
        } else {
            None
        };
        let entry = match cached {
            Some(entry) => entry,
            None => {
                let evaluated = eval_evaluated(pattern, ctx)?;
                if let Evaluated::Truncated(truncation) = &evaluated {
                    // Never memoized: a truncated inner bag cached under this site's key
                    // would make every subsequent probe — including one under a larger budget
                    // in the same query — read a bag that is not the inner pattern's
                    // answer. See the correlated branch above for why the boolean is
                    // irrelevant once the barrier is recorded.
                    ctx.expression_barrier.record(truncation.tripped());
                    return Ok(false);
                }
                let inner = Arc::new(evaluated.into_complete().unwrap_or_else(|_| {
                    unreachable!("a non-truncated result is complete by construction")
                }));
                // `shared` is computed against the FULL outer schema (not just the
                // row's bound vars), so one index serves every row: an outer var
                // unbound in a given row is `None` in the probe and matches anything
                // via `compatible`, exactly as the prior bound-only seed-join did.
                let shared = schema.shared_columns(&inner.schema);
                let (keyed, wild) = crate::binop::build_index(&inner, &shared);
                let entry = Arc::new(crate::eval::ExistsInner {
                    inner,
                    shared,
                    keyed,
                    wild,
                });
                if ctx.options.exists_memo && !ctx.in_substituted_exists {
                    ctx.exists_inner_cache.insert(key, entry.clone());
                }
                entry
            }
        };

        // Existence-only probe of the full outer row against the reused index.
        Ok(crate::binop::probe_has_match(
            row,
            &entry.shared,
            &entry.keyed,
            &entry.wild,
            &entry.inner.rows,
        ))
    }
}

/// The per-row μ used by pattern substitution — SEP-0007's `Replace` mechanism,
/// PurRDF's evaluation-side construction of it (see [`substitute_pattern`]).
///
/// The outer solution's bound variables are carried in the two forms
/// substitution needs, built together (once per outer row) by
/// [`outer_bindings_for_substitution`]:
///
/// * `expr` — IRI/literal-representable bindings for **expression positions**
///   (`substitute_expr`, unchanged since before this construct): `Expression`
///   has no constant spelling for a blank node or a quoted triple, so those two
///   term kinds are absent here. SPARQL §18.6's EXISTS substitution rewrites a
///   FILTER/BIND's syntax, and neither term kind has one.
/// * `term` — EVERY bound variable, IRI/literal/blank-node/quoted-triple alike,
///   as a [`purrdf_sparql_algebra::GroundTerm`]: the row Values Insertion joins
///   onto a `Bgp`/`Path` leaf (see [`substitute_pattern`]'s doc), total over
///   every term kind a syntactic term-rewrite could never place.
///
/// # Allocation shape (AGENTS.md's per-term-`String` hot-path rule)
///
/// Values Insertion means `term`/`expr` fan out to MANY sites per outer row:
/// once per `Bgp`/`Path` leaf ([`join_leaf_with_values`]), once per
/// expression-bearing node that needs a term-only var
/// ([`wrap_with_expr_term_only_values`]), and the whole row is cloned again at
/// every `Project` boundary ([`narrow_to`](Self::narrow_to)). Naively that is
/// O(leaves + wrappers + `Project` boundaries) DEEP clones of every bound
/// var's text per row on top of the O(bound vars) text this struct's own
/// construction already pays once
/// ([`outer_bindings_for_substitution`]/[`ground_term_from_term_value`]).
///
/// [`Variable`], [`purrdf_sparql_algebra::NamedNode`], and
/// [`purrdf_sparql_algebra::Literal`] store their text behind `Arc<str>` (see
/// those types' docs), so every one of those fan-out `.clone()` calls —
/// `Variable`, `GroundTerm::NamedNode`/`Literal`/`BlankNode`, `Expression::
/// NamedNode`/`Literal` alike — is a refcount bump, not a text
/// allocation/copy. The allocation count per row is therefore O(bound vars)
/// TEXT allocations, paid exactly once by construction; every fan-out site
/// after that is O(1) refcount traffic. The one residual exception is
/// `GroundTerm::Triple`: its `Box<GroundTriple>` is a unique owner (quoted
/// triples are rare in `VALUES` cells and not text-length-dependent), so a
/// leaf/wrapper clone of a bound quoted-triple term pays one small,
/// fixed-size `Box` allocation — not a `String`, and not one that grows with
/// leaf/wrapper count times term length.
pub(crate) struct SubstitutionRow {
    pub(crate) expr: Vec<(Variable, Expression)>,
    pub(crate) term: Vec<(Variable, purrdf_sparql_algebra::GroundTerm)>,
}

impl SubstitutionRow {
    /// Restrict μ to exactly `variables`, preserving each side's original
    /// (schema) order — the walk's response to a `Project` boundary, the ONLY
    /// scope boundary the surface language has (SEP-0006's rule, restated in
    /// [`substitute_pattern`]'s doc). The walk already deep-clones the tree
    /// once per outer row, so the narrowed copy here is noise against that
    /// cost, not a new per-row allocation class (AGENTS.md's hot-path rule) —
    /// see [`SubstitutionRow`]'s doc: every element clone here is an `Arc`
    /// refcount bump, not text allocation.
    fn narrow_to(&self, variables: &[Variable]) -> Self {
        Self {
            expr: self
                .expr
                .iter()
                .filter(|(v, _)| variables.contains(v))
                .cloned()
                .collect(),
            term: self
                .term
                .iter()
                .filter(|(v, _)| variables.contains(v))
                .cloned()
                .collect(),
        }
    }
}

/// Substitute outer-bound variables into a graph pattern for a correlated
/// per-row evaluation (a `LATERAL` right operand or an expression-correlated
/// `EXISTS` inner — see [`crate::binop::eval_correlated`], this walk's sole
/// caller).
///
/// # Values Insertion (SEP-0007's `Replace` mechanism)
///
/// A `Bgp` or `Path` leaf whose variables intersect `row.term` is wrapped as
/// `Join(leaf, Values { vars ∩ leaf-vars, [row] })` instead of being
/// term-rewritten. This is uniform over every term kind a `GroundTerm` can
/// carry — IRI, literal, blank node, quoted triple — because the row reaches
/// the dataset through the SAME evaluation path real `VALUES` data uses
/// (`crate::modifier::eval_values` → interning), not through a syntactic
/// rewrite that can only synthesize the term kinds a `TriplePattern` constant
/// slot admits *textually* reachable from an `Expression`. It also fixes the
/// MINUS disjoint-domain flip: because the leaf keeps its variable (only
/// gaining a joined `VALUES` row below it), `MINUS`'s both sides keep the
/// SAME schema column for that variable, so §18.5's domain-disjointness test
/// reads the truth instead of being fooled by a rewrite that erased the
/// column into a baked-in constant.
///
/// `Graph`/`Service` names keep their pre-existing treatment (a `Service`
/// variable endpoint resolves directly — the sole reason `LATERAL` ever needs
/// per-row re-evaluation at all, since a joined `VALUES` row cannot supply the
/// constant a federated call must be dispatched with; a `Graph` variable name
/// is left unresolved, exactly as before — an unconstrained `GRAPH ?g`
/// enumeration is already sound because the caller's own compatibility check
/// against μ, run after this substituted pattern is evaluated, filters it).
///
/// # The theorem
///
/// This walk and the parser's `LATERAL` scope-conflict check
/// (`purrdf_sparql_algebra::parser`) satisfy one theorem together: the parser
/// rejects exactly those programs in which injecting bindings across the
/// RHS's top scope level would be observable as a rebinding; this walk's
/// injection never crosses a `Project` boundary the projection does not
/// carry (`SubstitutionRow::narrow_to`, applied when descending through
/// `GraphPattern::Project`). Together: a parsed `LATERAL` evaluates per
/// SEP-0006's `Lateral(Ω,P) = ⋃ eval(inject(P, μ))` with `inject` this walk.
///
/// # Expression positions
///
/// `substitute_expr` still rewrites `Expression::Variable`/`Bound` occurrences
/// directly (SPARQL §18.6) for the IRI/literal fast path, including through a
/// nested `EXISTS`. An outer binding with no `Expression` constant form — a
/// blank node or a quoted triple — that occurs ONLY in an expression position
/// (no leaf occurrence Values Insertion above already covers) is instead
/// carried by Values Insertion applied to the expression's own owning pattern
/// node: `Filter`/`Extend`/`LeftJoin`/`OrderBy`/`Group`'s arms below join the
/// pattern the expression evaluates against with a one-row `VALUES` table
/// carrying exactly that variable (`wrap_with_expr_term_only_values`), so the
/// expression sees it bound like any other row cell rather than needing a
/// syntactic rewrite that term kind has no syntax for.
///
/// # Property-function arguments
///
/// Unchanged: still IRI-only value substitution via `substitute_term_pattern`
/// — the fusion contract a joined `VALUES` row cannot satisfy (see that
/// function's doc).
pub(crate) fn substitute_pattern(
    pattern: &GraphPattern,
    row: &SubstitutionRow,
) -> Box<GraphPattern> {
    let mut map: Option<&mut SubstitutionSourceMap> = None;
    substitute_pattern_impl(pattern, row, &mut map)
}

/// One [`SubstitutionSourceMap`] entry: which real plan node a substituted-tree node is
/// doing work on behalf of, and whether ITS OWN committed-output rows ARE that node's true
/// output for this row.
///
/// The two charge kinds this walk's synthetic nodes generate are attributed differently on
/// purpose:
///
/// - **Fuel** (and every other non-row charge point) is attributed to `source`
///   unconditionally, from every synthesized node — the leaf's own raw re-scan, the
///   one-row `VALUES` table, and the `Join` that narrows them — because that work really
///   happened and `source` is the only real node it can be charged against (see this
///   type's field-less predecessor's doc, preserved below, for why folding it into the
///   nearest enclosing REAL node instead would be worse). Fuel's sum-to-total invariant
///   depends on this: every unit charged during the substituted evaluation must land
///   somewhere, and nothing but `source` has a claim on it.
/// - **Committed rows/cells** are attributed to `source` ONLY from the node whose output
///   IS `source`'s true output for this row — `counts_rows`. A `Bgp`/`Path` leaf Values
///   Insertion wraps is evaluated UNCONSTRAINED (its own triple patterns are never
///   rewritten to ground terms — see [`substitute_pattern`]'s doc), so its own
///   committed rows are a re-scan cost, not `source`'s output; the one-row `VALUES` table
///   is bookkeeping, never `source`'s output either. Only the wrapping `Join`'s narrowed
///   result is. Counting all three would make `source`'s reported `rows`/`cells` a
///   composite of three different nodes' outputs rendered beside ONE estimate that
///   predicts only `source`'s own — the miscalibration this type exists to prevent.
///   `counts_rows` is `true` for a plain 1:1 copy (no wrapper needed) and for the
///   wrapper itself when one is added, `false` for the scaffolding a wrapper wraps.
///
/// A node the walk **synthesizes** — the `Values` Values Insertion adds, and the `Join`
/// that joins it in (see [`substitute_pattern`]'s doc) — carries no source identity of its
/// own, so it is mapped to the SAME source as the already-mapped copy it wraps
/// ([`join_leaf_with_values`]/[`wrap_with_expr_term_only_values`]). That, not leaving it
/// unmapped, is what keeps a bare `LATERAL { ?s :p ?o }` (no `Filter`/`Extend` between the
/// `LATERAL` and the wrapped leaf) from folding the wrapper's own charges into the
/// `LATERAL` node itself: the wrapper is very often the SUBSTITUTED TREE'S OWN ROOT in
/// that shape, so "fold into the nearest enclosing mapped node" would mean "fold into
/// `LATERAL`", corrupting exactly the row count this whole map exists to keep honest.
/// Mapping the wrapper to its child's source instead means every node in a wrapped
/// subtree — real or synthetic — resolves to the ONE real node it is doing work on behalf
/// of, and nothing but that node's own true output ever lands on an ancestor's row column.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SubstitutionSource {
    /// The real plan node's own address, in the UNSUBSTITUTED plan tree.
    pub(crate) source: usize,
    /// Whether this node's own committed rows/cells are `source`'s true output for this
    /// row — see this type's doc.
    pub(crate) counts_rows: bool,
}

/// Substituted-node address → [`SubstitutionSource`], one entry per node
/// [`substitute_pattern_tracked`]'s walk produces from the plan it is substituting.
///
/// Keyed by a per-row heap-temporary address. That address is internal evaluator
/// bookkeeping only: [`crate::governor::ledger::QueryExplanation::render`] emits ordinals
/// and labels, never an address, so this map's keys never reach anything the ledger
/// prints — see that function's determinism note.
pub(crate) type SubstitutionSourceMap = crate::DetHashMap<usize, SubstitutionSource>;

/// [`substitute_pattern`], additionally populating `map` (see [`SubstitutionSourceMap`]) —
/// the seam [`crate::binop::eval_correlated`] uses for a `LATERAL` right operand, whose
/// nodes ARE plan nodes and must keep their own ledger identity across the per-row
/// substituted copy. A correlated-`EXISTS` inner (also reached through
/// `eval_correlated`, via [`substitute_pattern`] without a map) has no plan identity to
/// preserve — see the ledger module doc's "Attribution of work that is not a plan node" —
/// so it is deliberately never tracked.
pub(crate) fn substitute_pattern_tracked(
    pattern: &GraphPattern,
    row: &SubstitutionRow,
    tracked: &mut SubstitutionSourceMap,
) -> Box<GraphPattern> {
    let mut map = Some(tracked);
    substitute_pattern_impl(pattern, row, &mut map)
}

/// The substitution walk itself. Every arm ends by boxing the node it built and — through
/// [`boxed_and_mapped`] — recording, IF `map` is engaged, that box's address (its
/// permanent location; a `Box`'s pointee never moves again once allocated) against
/// `pattern`'s address, so a later [`resolve_ledger_ordinal`](crate::eval::EvalCtx::resolve_ledger_ordinal)
/// lookup on the returned tree resolves back to the exact plan node this call
/// substituted. [`GraphPattern::Bgp`]/[`GraphPattern::Path`] and the expression-bearing
/// arms additionally hand [`join_leaf_with_values`]/[`wrap_with_expr_term_only_values`]
/// the SAME `pattern`/`map`, so a synthetic `Join`/`Values` wrapper those helpers add gets
/// mapped to the identical source its wrapped child already resolves to — see
/// [`SubstitutionSourceMap`]'s doc for why that, not leaving the wrapper unmapped, is the
/// correct choice.
fn substitute_pattern_impl(
    pattern: &GraphPattern,
    row: &SubstitutionRow,
    map: &mut Option<&mut SubstitutionSourceMap>,
) -> Box<GraphPattern> {
    match pattern {
        GraphPattern::Bgp { patterns } => {
            let mut vars = DetHashSet::default();
            for tp in patterns {
                term_pattern_vars(&tp.subject, &mut vars);
                if let purrdf_sparql_algebra::NamedNodePattern::Variable(v) = &tp.predicate {
                    vars.insert(v.clone());
                }
                term_pattern_vars(&tp.object, &mut vars);
            }
            let leaf = boxed_and_mapped(
                GraphPattern::Bgp {
                    patterns: patterns.clone(),
                },
                pattern,
                map,
            );
            join_leaf_with_values(leaf, &vars, &row.term, pattern, map)
        }
        GraphPattern::Path {
            subject,
            path,
            object,
        } => {
            let mut vars = DetHashSet::default();
            term_pattern_vars(subject, &mut vars);
            term_pattern_vars(object, &mut vars);
            let leaf = boxed_and_mapped(
                GraphPattern::Path {
                    subject: subject.clone(),
                    path: path.clone(),
                    object: object.clone(),
                },
                pattern,
                map,
            );
            join_leaf_with_values(leaf, &vars, &row.term, pattern, map)
        }
        // A property function's argument vectors are term positions, but — unlike a
        // `Bgp`/`Path` leaf, which now receives its row via a joined `VALUES` above —
        // the relation's invocation contract needs the argument itself constant, not
        // merely join-compatible with one (`substitute_term_pattern`'s fusion-contract
        // doc). This is that helper's one remaining client.
        GraphPattern::PropertyFunction(call) => boxed_and_mapped(
            GraphPattern::PropertyFunction(purrdf_sparql_algebra::PropertyFunctionCall {
                iri: call.iri.clone(),
                subject_args: call
                    .subject_args
                    .iter()
                    .map(|term| substitute_term_pattern(term, &row.expr))
                    .collect(),
                object_args: call
                    .object_args
                    .iter()
                    .map(|term| substitute_term_pattern(term, &row.expr))
                    .collect(),
            }),
            pattern,
            map,
        ),
        // `wrap_with_expr_term_only_values` closes the gap `substitute_expr` alone
        // leaves open: a blank-node/quoted-triple outer binding referenced ONLY by
        // `expr` (no leaf occurrence elsewhere in `inner`) has no `Expression`
        // constant `substitute_expr` could rewrite it to, so `inner` is instead
        // joined against a one-row `VALUES` carrying it — `expr` then evaluates
        // against a row where the variable is bound like any other.
        GraphPattern::Filter { expr, inner } => {
            let mut free = DetHashSet::default();
            expr_vars(expr, &mut free);
            let inner_sub = substitute_pattern_impl(inner, row, map);
            let inner_final = wrap_with_expr_term_only_values(inner_sub, &free, row, inner, map);
            boxed_and_mapped(
                GraphPattern::Filter {
                    expr: substitute_expr(expr, row),
                    inner: inner_final,
                },
                pattern,
                map,
            )
        }
        GraphPattern::Extend {
            inner,
            variable,
            expression,
        } => {
            let mut free = DetHashSet::default();
            expr_vars(expression, &mut free);
            let inner_sub = substitute_pattern_impl(inner, row, map);
            let inner_final = wrap_with_expr_term_only_values(inner_sub, &free, row, inner, map);
            boxed_and_mapped(
                GraphPattern::Extend {
                    inner: inner_final,
                    variable: variable.clone(),
                    expression: substitute_expr(expression, row),
                },
                pattern,
                map,
            )
        }
        GraphPattern::Join { left, right } => {
            let left_sub = substitute_pattern_impl(left, row, map);
            let right_sub = substitute_pattern_impl(right, row, map);
            boxed_and_mapped(
                GraphPattern::Join {
                    left: left_sub,
                    right: right_sub,
                },
                pattern,
                map,
            )
        }
        GraphPattern::Union { left, right } => {
            let left_sub = substitute_pattern_impl(left, row, map);
            let right_sub = substitute_pattern_impl(right, row, map);
            boxed_and_mapped(
                GraphPattern::Union {
                    left: left_sub,
                    right: right_sub,
                },
                pattern,
                map,
            )
        }
        // The optional inline filter evaluates against the (already merged) joined
        // row, so a term-only variable it needs is injected on `left`: an outer
        // binding is never a `right`-side join key (the parser forbids `right`
        // rebinding it), so it survives into every merged row regardless of
        // whether `right` finds a match, exactly where the filter needs it.
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => {
            let mut free = DetHashSet::default();
            if let Some(e) = expression {
                expr_vars(e, &mut free);
            }
            let left_sub = substitute_pattern_impl(left, row, map);
            let left_final = wrap_with_expr_term_only_values(left_sub, &free, row, left, map);
            let right_sub = substitute_pattern_impl(right, row, map);
            boxed_and_mapped(
                GraphPattern::LeftJoin {
                    left: left_final,
                    right: right_sub,
                    expression: expression.as_ref().map(|e| substitute_expr(e, row)),
                },
                pattern,
                map,
            )
        }
        // Both sides receive the SAME row (see this function's doc on the MINUS
        // disjoint-domain fix): the right side is NOT excluded from substitution here,
        // only from the PARSER's LATERAL scope check (§18.2.1 governs scope, not
        // evaluation-time injection).
        GraphPattern::Minus { left, right } => {
            let left_sub = substitute_pattern_impl(left, row, map);
            let right_sub = substitute_pattern_impl(right, row, map);
            boxed_and_mapped(
                GraphPattern::Minus {
                    left: left_sub,
                    right: right_sub,
                },
                pattern,
                map,
            )
        }
        GraphPattern::Lateral { left, right } => {
            let left_sub = substitute_pattern_impl(left, row, map);
            let right_sub = substitute_pattern_impl(right, row, map);
            boxed_and_mapped(
                GraphPattern::Lateral {
                    left: left_sub,
                    right: right_sub,
                },
                pattern,
                map,
            )
        }
        // The graph name is left unresolved even when it is a bound variable — see this
        // function's doc for why that is sound (the caller's post-hoc compatibility
        // check against μ is what makes it sound, not a substitution here).
        GraphPattern::Graph { name, inner } => {
            let inner_sub = substitute_pattern_impl(inner, row, map);
            boxed_and_mapped(
                GraphPattern::Graph {
                    name: name.clone(),
                    inner: inner_sub,
                },
                pattern,
                map,
            )
        }
        GraphPattern::Service {
            name,
            inner,
            silent,
        } => {
            // A variable endpoint (`SERVICE ?g`) bound to an IRI by the enclosing
            // solution becomes a concrete named endpoint, so the substituted
            // pattern federates against the resolved IRI (the LATERAL seam) — a
            // joined `VALUES` row cannot supply the constant a dispatch needs.
            let resolved_name = match name {
                purrdf_sparql_algebra::NamedNodePattern::Variable(v) => row
                    .expr
                    .iter()
                    .find(|(bv, _)| bv == v)
                    .and_then(|(_, e)| match e {
                        Expression::NamedNode(n) => Some(
                            purrdf_sparql_algebra::NamedNodePattern::NamedNode(n.clone()),
                        ),
                        _ => None,
                    })
                    .unwrap_or_else(|| name.clone()),
                purrdf_sparql_algebra::NamedNodePattern::NamedNode(_) => name.clone(),
            };
            let inner_sub = substitute_pattern_impl(inner, row, map);
            boxed_and_mapped(
                GraphPattern::Service {
                    name: resolved_name,
                    inner: inner_sub,
                    silent: *silent,
                },
                pattern,
                map,
            )
        }
        GraphPattern::OrderBy { inner, expression } => {
            let mut free = DetHashSet::default();
            for oe in expression {
                match oe {
                    purrdf_sparql_algebra::OrderExpression::Asc(e)
                    | purrdf_sparql_algebra::OrderExpression::Desc(e) => expr_vars(e, &mut free),
                }
            }
            let inner_sub = substitute_pattern_impl(inner, row, map);
            let inner_final = wrap_with_expr_term_only_values(inner_sub, &free, row, inner, map);
            boxed_and_mapped(
                GraphPattern::OrderBy {
                    inner: inner_final,
                    expression: expression
                        .iter()
                        .map(|oe| match oe {
                            purrdf_sparql_algebra::OrderExpression::Asc(e) => {
                                purrdf_sparql_algebra::OrderExpression::Asc(substitute_expr(e, row))
                            }
                            purrdf_sparql_algebra::OrderExpression::Desc(e) => {
                                purrdf_sparql_algebra::OrderExpression::Desc(substitute_expr(
                                    e, row,
                                ))
                            }
                        })
                        .collect(),
                },
                pattern,
                map,
            )
        }
        GraphPattern::Group {
            inner,
            variables,
            aggregates,
        } => {
            let mut free = DetHashSet::default();
            for (_, agg) in aggregates {
                for arg in agg.args() {
                    expr_vars(arg, &mut free);
                }
            }
            let inner_sub = substitute_pattern_impl(inner, row, map);
            let inner_final = wrap_with_expr_term_only_values(inner_sub, &free, row, inner, map);
            boxed_and_mapped(
                GraphPattern::Group {
                    inner: inner_final,
                    variables: variables.clone(),
                    aggregates: aggregates
                        .iter()
                        .map(|(v, agg)| {
                            let args = agg.args().iter().map(|e| substitute_expr(e, row)).collect();
                            // `substitute_expr` rewrites each argument in place and never
                            // changes the argument COUNT, so this can never turn a valid
                            // `agg` into an invalid one.
                            let new_agg = purrdf_sparql_algebra::AggregateExpression::new(
                                agg.function().clone(),
                                args,
                                agg.scalarvals().to_vec(),
                                agg.distinct,
                            )
                            .expect("substitution preserves argument count, so arity stays valid");
                            (v.clone(), new_agg)
                        })
                        .collect(),
                },
                pattern,
                map,
            )
        }
        // Leaf patterns that need no substitution.
        GraphPattern::Distinct { inner } => {
            let inner_sub = substitute_pattern_impl(inner, row, map);
            boxed_and_mapped(GraphPattern::Distinct { inner: inner_sub }, pattern, map)
        }
        GraphPattern::Reduced { inner } => {
            let inner_sub = substitute_pattern_impl(inner, row, map);
            boxed_and_mapped(GraphPattern::Reduced { inner: inner_sub }, pattern, map)
        }
        GraphPattern::Slice {
            inner,
            start,
            length,
        } => {
            let inner_sub = substitute_pattern_impl(inner, row, map);
            boxed_and_mapped(
                GraphPattern::Slice {
                    inner: inner_sub,
                    start: *start,
                    length: *length,
                },
                pattern,
                map,
            )
        }
        // The Project-boundary narrowing this walk's theorem depends on: only the
        // projected variables survive to the inner pattern's substitution.
        GraphPattern::Project { inner, variables } => {
            let narrowed = row.narrow_to(variables);
            let inner_sub = substitute_pattern_impl(inner, &narrowed, map);
            boxed_and_mapped(
                GraphPattern::Project {
                    inner: inner_sub,
                    variables: variables.clone(),
                },
                pattern,
                map,
            )
        }
        GraphPattern::Values { .. } => boxed_and_mapped(pattern.clone(), pattern, map),
    }
}

/// Box `built` (its final, permanent address — a `Box`'s pointee never moves again) and,
/// if `map` is engaged, record that address against `source`'s, `counts_rows: true` — the
/// default for a node that has not (yet) been wrapped by
/// [`join_leaf_with_values`]/[`wrap_with_expr_term_only_values`]. The one place
/// [`substitute_pattern_impl`] pairs a `Box::new` with a [`SubstitutionSourceMap`] entry,
/// so every entry is keyed by an address the node actually ends up at rather than one it
/// only transiently held on the stack before being moved into its `Box`.
fn boxed_and_mapped(
    built: GraphPattern,
    source: &GraphPattern,
    map: &mut Option<&mut SubstitutionSourceMap>,
) -> Box<GraphPattern> {
    boxed_and_mapped_as(built, source, map, true)
}

/// [`boxed_and_mapped`], but for Values-Insertion SCAFFOLDING — the one-row `VALUES` table
/// a wrapper joins in — whose own committed rows are never `source`'s true output (see
/// [`SubstitutionSource`]'s doc) and must therefore not count toward it.
fn boxed_and_mapped_scaffolding(
    built: GraphPattern,
    source: &GraphPattern,
    map: &mut Option<&mut SubstitutionSourceMap>,
) -> Box<GraphPattern> {
    boxed_and_mapped_as(built, source, map, false)
}

/// Shared implementation of [`boxed_and_mapped`]/[`boxed_and_mapped_scaffolding`].
fn boxed_and_mapped_as(
    built: GraphPattern,
    source: &GraphPattern,
    map: &mut Option<&mut SubstitutionSourceMap>,
    counts_rows: bool,
) -> Box<GraphPattern> {
    let boxed = Box::new(built);
    if let Some(m) = map.as_deref_mut() {
        m.insert(
            std::ptr::from_ref(boxed.as_ref()) as usize,
            SubstitutionSource {
                source: std::ptr::from_ref(source) as usize,
                counts_rows,
            },
        );
    }
    boxed
}

/// Demote an already-mapped node (inserted `counts_rows: true` by [`boxed_and_mapped`]) to
/// scaffolding, because a wrapper is being added over it: the wrapper's own narrowed output
/// becomes `source`'s true output for this row, so the wrapped node underneath must stop
/// counting rows/cells — its fuel keeps accruing to `source` unaffected (see
/// [`SubstitutionSource`]'s doc). A no-op when `map` is disengaged.
fn demote_to_scaffolding(map: &mut Option<&mut SubstitutionSourceMap>, address: usize) {
    if let Some(m) = map.as_deref_mut()
        && let Some(entry) = m.get_mut(&address)
    {
        entry.counts_rows = false;
    }
}

/// Wrap a `Bgp`/`Path` leaf in `Join(leaf, Values { .. })` when its variables
/// intersect the injected row (Values Insertion — see [`substitute_pattern`]'s
/// doc). The joined `VALUES` carries exactly `leaf_vars ∩ row-vars`, in the
/// row's own (schema) order — a single row, so the join can only narrow, never
/// multiply, the leaf's solutions. A leaf sharing nothing with the row is
/// returned unchanged: no join is worth adding, and the pattern stays
/// leaf-shaped for every operator that inspects it structurally (e.g. the
/// property-function fusion check in `crate::property_fn_eval`).
///
/// `leaf` arrives already boxed — at its permanent address — and, when a
/// [`SubstitutionSourceMap`] is engaged, already mapped to `source` by the caller
/// ([`substitute_pattern_impl`]'s `Bgp`/`Path` arms, which pass their own `pattern` as
/// `source`), `counts_rows: true`. When this function DOES add the synthetic
/// `Join`/`Values` wrapper, the wrapper is mapped to that SAME `source` — see
/// [`SubstitutionSourceMap`]'s doc for why a wrapper that inherited from its enclosing node
/// instead would, for a `LATERAL` whose right operand is directly a `Bgp`/`Path` (no
/// `Filter` between the `LATERAL` and the wrapped leaf), fold straight into the `LATERAL`
/// node — but the wrapper, not `leaf`, becomes the row-counting entry
/// ([`SubstitutionSource::counts_rows`]): `leaf` is demoted to scaffolding, because its own
/// committed rows are the UNCONSTRAINED scan (Values Insertion never rewrites the leaf's
/// own triple patterns to ground terms), not `source`'s output for this row, and the
/// `Values` table is bookkeeping that carries no output of its own either. Only the
/// `Join`'s narrowed result is `source`'s true output.
///
/// This runs once per `Bgp`/`Path` leaf in the substituted pattern, so a query with
/// several leaves sharing bound vars clones each `(Variable, GroundTerm)` pair once per
/// leaf that needs it — see [`SubstitutionRow`]'s doc: that per-leaf fan-out is `Arc`
/// refcount traffic (both types store their text behind `Arc<str>`), not the per-term
/// `String` allocation AGENTS.md's hot-path rule forbids.
fn join_leaf_with_values(
    leaf: Box<GraphPattern>,
    leaf_vars: &DetHashSet<Variable>,
    term_row: &[(Variable, purrdf_sparql_algebra::GroundTerm)],
    source: &GraphPattern,
    map: &mut Option<&mut SubstitutionSourceMap>,
) -> Box<GraphPattern> {
    let restricted: Vec<&(Variable, purrdf_sparql_algebra::GroundTerm)> = term_row
        .iter()
        .filter(|(v, _)| leaf_vars.contains(v))
        .collect();
    if restricted.is_empty() {
        return leaf;
    }
    let mut variables = Vec::with_capacity(restricted.len());
    let mut values_row = Vec::with_capacity(restricted.len());
    for (v, g) in restricted {
        variables.push(v.clone());
        values_row.push(Some(g.clone()));
    }
    demote_to_scaffolding(map, std::ptr::from_ref(leaf.as_ref()) as usize);
    let values = boxed_and_mapped_scaffolding(
        GraphPattern::Values {
            variables,
            bindings: vec![values_row],
        },
        source,
        map,
    );
    boxed_and_mapped(
        GraphPattern::Join {
            left: leaf,
            right: values,
        },
        source,
        map,
    )
}

/// Wrap `node` in `Join(node, Values { .. })` for every variable `expr_free_vars`
/// mentions that is bound in `row.term` **but has no `Expression` constant form**
/// (a blank node or a quoted triple — see [`outer_bindings_for_substitution`]'s
/// doc on why `row.expr` omits them). This is Values Insertion applied to an
/// expression-only occurrence: `substitute_expr` cannot rewrite
/// `Expression::Variable(v)`/`Expression::Bound(v)` into a constant for such a
/// `v` (no SPARQL expression syntax spells a bare blank node or a quoted
/// triple), so instead the enclosing pattern node — `node`, already the
/// recursively-substituted `inner`/`left` a `Filter`/`Extend`/`LeftJoin`/
/// `OrderBy`/`Group` evaluates its expression against — is joined against a
/// one-row `VALUES` table carrying exactly those bindings. The variable then
/// arrives at expression evaluation BOUND, through the ordinary row the
/// expression evaluator already reads for every other bound variable: no new
/// evaluation path, `BOUND`/`sameTerm`/every builtin sees it like any other
/// solution cell.
///
/// A variable already covered by a leaf's own Values Insertion below `node`
/// (or already carried in `row.expr`) is excluded here, so the common
/// IRI/literal case pays no extra join — `substitute_expr`'s direct constant
/// rewrite stays the fast path. Joining a variable the leaf ALSO already
/// binds (to the same term — the parser's scope-conflict check forbids the
/// RHS from rebinding an outer variable) is compatible and therefore
/// harmless if it ever occurs; this function does not attempt to detect that
/// case, only the term-kind one that make it necessary.
///
/// `node` arrives already boxed and, when a [`SubstitutionSourceMap`] is engaged, already
/// mapped to `source` by the caller, `counts_rows: true` — exactly like
/// [`join_leaf_with_values`]'s `leaf`, and for the same reason: when this function DOES add
/// the synthetic `Join`/`Values` wrapper, the wrapper is mapped to that SAME `source`
/// rather than left to inherit from whatever encloses it — see [`SubstitutionSourceMap`]'s
/// doc — and BECOMES the row-counting entry for `source`, demoting `node` (whose own
/// committed rows are pre-narrowing, not `source`'s output for this row) to scaffolding,
/// same as there.
///
/// Same allocation shape as [`join_leaf_with_values`]: this runs once per
/// expression-bearing node that needs a term-only var, and the `(Variable, GroundTerm)`
/// clone at each such site is `Arc` refcount traffic (see [`SubstitutionRow`]'s doc), not
/// a `String` allocation.
fn wrap_with_expr_term_only_values(
    node: Box<GraphPattern>,
    expr_free_vars: &DetHashSet<Variable>,
    row: &SubstitutionRow,
    source: &GraphPattern,
    map: &mut Option<&mut SubstitutionSourceMap>,
) -> Box<GraphPattern> {
    if expr_free_vars.is_empty() {
        return node;
    }
    let needed: Vec<&(Variable, purrdf_sparql_algebra::GroundTerm)> = row
        .term
        .iter()
        .filter(|(v, _)| expr_free_vars.contains(v) && !row.expr.iter().any(|(ev, _)| ev == v))
        .collect();
    if needed.is_empty() {
        return node;
    }
    let mut variables = Vec::with_capacity(needed.len());
    let mut values_row = Vec::with_capacity(needed.len());
    for (v, g) in needed {
        variables.push(v.clone());
        values_row.push(Some(g.clone()));
    }
    demote_to_scaffolding(map, std::ptr::from_ref(node.as_ref()) as usize);
    let values = boxed_and_mapped_scaffolding(
        GraphPattern::Values {
            variables,
            bindings: vec![values_row],
        },
        source,
        map,
    );
    boxed_and_mapped(
        GraphPattern::Join {
            left: node,
            right: values,
        },
        source,
        map,
    )
}

/// Substitute outer-bound variables into a property-function argument term —
/// the ONLY remaining client of this substitution style, since Values
/// Insertion (`substitute_pattern`'s `Bgp`/`Path` arms) took over triple-pattern
/// positions.
///
/// IRI-valued bindings only, by the fusion contract: a property-function
/// argument is an INVOCATION INPUT the relation reads directly from the row
/// (`crate::property_fn_eval`), not a join key a `VALUES` row could supply —
/// joining a literal/blank-node/quoted-triple binding in would hand the
/// relation a FREE argument position it may refuse to be invoked with, where a
/// rewritten IRI constant is a position the relation can read exactly as if
/// the caller had written it literally. A literal, blank-node, or quoted-triple
/// binding therefore leaves the argument as the original variable, and the
/// per-row argument read supplies the value instead — see
/// `property_function_arg_with_a_literal_binding_behaves_unchanged`.
fn substitute_term_pattern(
    term: &purrdf_sparql_algebra::TermPattern,
    bindings: &[(Variable, Expression)],
) -> purrdf_sparql_algebra::TermPattern {
    use purrdf_sparql_algebra::TermPattern;

    if let TermPattern::Variable(v) = term {
        for (bv, expr) in bindings {
            if bv == v
                && let Expression::NamedNode(n) = expr
            {
                return TermPattern::NamedNode(n.clone());
            }
        }
    }
    term.clone()
}

/// Substitute outer-bound variables in expression positions by replacing
/// `Expression::Variable(v)` with the corresponding constant expression
/// (`row.expr`) — the IRI/literal fast path. A nested `EXISTS`'s inner pattern
/// gets the FULL row (`row`, not just `row.expr`), so Values Insertion reaches
/// its leaves too.
///
/// This function alone is NOT total over `Expression::Variable`: a blank-node
/// or quoted-triple binding has no `row.expr` entry (no expression syntax
/// spells one) and so is left as the unresolved `Expression::Variable(v)`
/// here — total coverage for that case is `wrap_with_expr_term_only_values`,
/// applied by `substitute_pattern`'s expression-bearing arms to the pattern
/// node the expression evaluates against, so `v` arrives already bound in the
/// row by the time this leftover `Expression::Variable(v)` is evaluated.
/// `Expression::Bound`, by contrast, IS total here: it only needs to know
/// THAT `v` is bound, which `row.term` (not `row.expr`) answers for every
/// term kind.
fn substitute_expr(expr: &Expression, row: &SubstitutionRow) -> Expression {
    let bindings = &row.expr;
    match expr {
        Expression::Variable(v) => {
            for (bv, replacement) in bindings {
                if bv == v {
                    return replacement.clone();
                }
            }
            expr.clone()
        }
        Expression::Bound(v) => {
            // BOUND(?v) where ?v is outer-bound → always true (the variable IS
            // bound). Checked against `row.term`, not `bindings` (`row.expr`):
            // `row.term` is total over every outer-bound variable, including a
            // blank node or a quoted-triple binding `row.expr` has no constant
            // form for (see `outer_bindings_for_substitution`'s doc) — `BOUND`
            // only needs to know THAT ?v is bound, never a term to substitute,
            // so the totality gap that forces `wrap_with_expr_term_only_values`
            // for `Expression::Variable` does not apply here.
            if row.term.iter().any(|(bv, _)| bv == v) {
                return Expression::Literal(purrdf_sparql_algebra::Literal::new_typed(
                    "true",
                    purrdf_sparql_algebra::NamedNode::new_unchecked(XSD_BOOLEAN),
                ));
            }
            expr.clone()
        }
        Expression::NamedNode(_) | Expression::Literal(_) => expr.clone(),
        Expression::Or(a, b) => Expression::Or(
            Box::new(substitute_expr(a, row)),
            Box::new(substitute_expr(b, row)),
        ),
        Expression::And(a, b) => Expression::And(
            Box::new(substitute_expr(a, row)),
            Box::new(substitute_expr(b, row)),
        ),
        Expression::Equal(a, b) => Expression::Equal(
            Box::new(substitute_expr(a, row)),
            Box::new(substitute_expr(b, row)),
        ),
        Expression::SameTerm(a, b) => Expression::SameTerm(
            Box::new(substitute_expr(a, row)),
            Box::new(substitute_expr(b, row)),
        ),
        Expression::Greater(a, b) => Expression::Greater(
            Box::new(substitute_expr(a, row)),
            Box::new(substitute_expr(b, row)),
        ),
        Expression::GreaterOrEqual(a, b) => Expression::GreaterOrEqual(
            Box::new(substitute_expr(a, row)),
            Box::new(substitute_expr(b, row)),
        ),
        Expression::Less(a, b) => Expression::Less(
            Box::new(substitute_expr(a, row)),
            Box::new(substitute_expr(b, row)),
        ),
        Expression::LessOrEqual(a, b) => Expression::LessOrEqual(
            Box::new(substitute_expr(a, row)),
            Box::new(substitute_expr(b, row)),
        ),
        Expression::Add(a, b) => Expression::Add(
            Box::new(substitute_expr(a, row)),
            Box::new(substitute_expr(b, row)),
        ),
        Expression::Subtract(a, b) => Expression::Subtract(
            Box::new(substitute_expr(a, row)),
            Box::new(substitute_expr(b, row)),
        ),
        Expression::Multiply(a, b) => Expression::Multiply(
            Box::new(substitute_expr(a, row)),
            Box::new(substitute_expr(b, row)),
        ),
        Expression::Divide(a, b) => Expression::Divide(
            Box::new(substitute_expr(a, row)),
            Box::new(substitute_expr(b, row)),
        ),
        Expression::UnaryPlus(a) => Expression::UnaryPlus(Box::new(substitute_expr(a, row))),
        Expression::UnaryMinus(a) => Expression::UnaryMinus(Box::new(substitute_expr(a, row))),
        Expression::Not(a) => Expression::Not(Box::new(substitute_expr(a, row))),
        Expression::If(c, t, e) => Expression::If(
            Box::new(substitute_expr(c, row)),
            Box::new(substitute_expr(t, row)),
            Box::new(substitute_expr(e, row)),
        ),
        Expression::In(needle, haystack) => Expression::In(
            Box::new(substitute_expr(needle, row)),
            haystack.iter().map(|h| substitute_expr(h, row)).collect(),
        ),
        Expression::Coalesce(items) => {
            Expression::Coalesce(items.iter().map(|i| substitute_expr(i, row)).collect())
        }
        Expression::FunctionCall(f, args) => Expression::FunctionCall(
            f.clone(),
            args.iter().map(|a| substitute_expr(a, row)).collect(),
        ),
        // Untracked (`map = None`): a nested `EXISTS`'s inner pattern is not walked by
        // `walk_spine` even in its un-substituted form (see the ledger module doc's
        // "Attribution of work that is not a plan node"), so it has no plan identity for
        // any map to preserve — its charges fold into the enclosing node exactly as
        // before.
        Expression::Exists(inner_pat) => Expression::Exists(substitute_pattern(inner_pat, row)),
    }
}

/// Build the [`SubstitutionRow`] μ from the outer row's bound variables,
/// materializing each `SolutionTerm` to both a constant `Expression` (when
/// representable — IRI/literal) and a total `GroundTerm` (every term kind).
///
/// Runs once per outer row (this function's sole caller,
/// `crate::binop::eval_correlated`, calls it once per left row before
/// walking the pattern). For an IRI/literal-representable var this builds an
/// owned [`purrdf_sparql_algebra::NamedNode`]/[`purrdf_sparql_algebra::Literal`]
/// (the ONE text materialization from the dataset's interned form — genuinely
/// new text, so genuinely a `String`-shaped cost, not avoidable here) and
/// then clones it a second time into `expr`'s `Expression`: that second clone
/// is an `Arc` refcount bump (see [`SubstitutionRow`]'s doc), not a second
/// text allocation, because `NamedNode`/`Literal` carry their text behind
/// `Arc<str>`.
pub(crate) fn outer_bindings_for_substitution<D: DatasetView + Sync>(
    row: &[Option<SolutionTerm<D::Id>>],
    schema: &VarSchema,
    ctx: &EvalCtx<'_, D>,
) -> SubstitutionRow {
    use purrdf_sparql_algebra::GroundTerm;

    let mut expr = Vec::new();
    let mut term = Vec::new();
    for (i, var) in schema.vars().iter().enumerate() {
        if let Some(t) = row[i] {
            let value = ctx.scratch.value_of(ctx.dataset, t);
            // `None` means `value` is a quoted triple with a non-IRI predicate — see
            // `ground_term_from_term_value`'s doc. That term kind has no `GroundTerm`
            // representation, so the variable is left OUT of both `expr` and `term`:
            // the same "uninjected" treatment `wrap_with_expr_term_only_values` and
            // `join_leaf_with_values` already give any variable absent from
            // `row.term` (they simply don't build a `VALUES` cell for it). The
            // substituted plan then evaluates with that one variable unbound rather
            // than panicking on a foreign `DatasetView`'s malformed data.
            let Some(ground) = ground_term_from_term_value(&value) else {
                continue;
            };
            match &ground {
                GroundTerm::NamedNode(n) => {
                    expr.push((var.clone(), Expression::NamedNode(n.clone())));
                }
                GroundTerm::Literal(l) => expr.push((var.clone(), Expression::Literal(l.clone()))),
                // Blank nodes and quoted triples have no `Expression` constant
                // form; the leaf join (`row.term`, populated below regardless)
                // is what carries them.
                GroundTerm::BlankNode(_) | GroundTerm::Triple(_) => {}
            }
            term.push((var.clone(), ground));
        }
    }
    SubstitutionRow { expr, term }
}

/// Convert an interned term's dataset-independent value to the algebra's
/// [`purrdf_sparql_algebra::GroundTerm`] — the constant a `VALUES` row cell
/// carries. `None` means `value` has no `GroundTerm` representation; today the
/// only such case is a quoted triple whose predicate is not an IRI (see the
/// `TermValue::Triple` arm below) — the caller,
/// [`outer_bindings_for_substitution`], leaves that variable uninjected rather
/// than treating the whole substitution as failed.
///
/// IRI/literal construction below (`NamedNode::new_unchecked`) stays
/// unconditionally safe: every IRI/datatype IRI reaching this function was
/// already validated when the engine interned it (this function only ever
/// sees a value read back out of `ctx.scratch`), matching
/// `outer_bindings_for_substitution`'s sibling `Expression` construction.
fn ground_term_from_term_value(value: &TermValue) -> Option<purrdf_sparql_algebra::GroundTerm> {
    use purrdf_sparql_algebra::{
        BaseDirection, BlankNode, GroundTerm, GroundTriple, Literal, NamedNode,
    };

    Some(match value {
        TermValue::Iri(iri) => GroundTerm::NamedNode(NamedNode::new_unchecked(iri)),
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => {
            let lit = if let Some(lang) = language {
                // The algebra's `BaseDirection` and the IR's `RdfTextDirection` are
                // the same two-value RDF 1.2 base-direction enum in two crates;
                // `crate::convert::map_direction` maps the OTHER way (algebra → IR,
                // for a query-authored literal reaching the dataset lookup key), so
                // this direction gets the mirrored match inline.
                let dir = direction.map(|d| match d {
                    RdfTextDirection::Ltr => BaseDirection::Ltr,
                    RdfTextDirection::Rtl => BaseDirection::Rtl,
                });
                Literal::new_lang(lexical_form, lang, dir)
            } else {
                Literal::new_typed(lexical_form, NamedNode::new_unchecked(datatype))
            };
            GroundTerm::Literal(lit)
        }
        // The algebra's blank-node slot carries a scope-qualified spelling (the
        // same single-slot convention `GroundTerm::BlankNode`'s own doc and
        // `crate::convert::ground_term_to_value` use), so a blank bound by an
        // earlier evaluation round-trips to that SAME node through the `VALUES`
        // join rather than to a fresh, unrelated one.
        TermValue::Blank { label, scope } => {
            GroundTerm::BlankNode(BlankNode::new(scope.qualify_label(label).into_owned()))
        }
        TermValue::Triple { s, p, o } => {
            let subject = ground_term_from_term_value(s)?;
            // A quoted triple's predicate MUST be an IRI (RDF 1.2 C0 positional
            // constraint). For PurRDF's own data this is enforced once, structurally,
            // at `RdfDatasetBuilder::freeze` (`crates/rdf-core/src/ir/validate.rs`,
            // `require_iri_predicate`, reached for every interned triple term via
            // `validate_triple_terms`) — every `TermId` this evaluator resolves out
            // of a purrdf-built `RdfDataset` already cleared that gate, so this arm
            // is unreachable in practice for `RdfDataset`.
            //
            // That gate, however, sits on `RdfDatasetBuilder::freeze`, NOT on the
            // `DatasetView` trait (`crates/rdf-core/src/dataset_view.rs`) this
            // function is generic over. `DatasetView` is public and UNSEALED: a
            // third-party implementation can hand back a `TermRef` for a triple's
            // predicate id that resolves to anything at all, with no freeze-time
            // validation ever run. `value_of`/`term_id_to_value` (`crate::scratch`)
            // copy exactly what such a view's `resolve` returns into this
            // `TermValue`, so a malformed quoted triple from a foreign dataset
            // reaches here as real, unvalidated input on a LATERAL/EXISTS
            // substitution path. Because the invariant is NOT enforced at a boundary
            // every `DatasetView` implementor is forced through, a `debug_assert!`
            // here would panic on exactly that foreign input in every debug/test
            // build — the same crash this arm exists to remove. So: a total `None`,
            // no assertion, not `unreachable!`.
            let TermValue::Iri(p_iri) = p.as_ref() else {
                return None;
            };
            let predicate = NamedNode::new_unchecked(p_iri);
            let object = ground_term_from_term_value(o)?;
            GroundTerm::Triple(Box::new(GroundTriple {
                subject,
                predicate,
                object,
            }))
        }
    })
}

/// Dispatch a built-in (or custom) function call.
fn eval_function<D: DatasetView + Sync>(
    function: &Function,
    args: &[Expression],
    row: &[Option<SolutionTerm<D::Id>>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    match function {
        Function::Contains => {
            return eval_string_pred_expr(args, row, schema, ctx, |h, n| h.contains(n));
        }
        Function::StrStarts => {
            return eval_string_pred_expr(args, row, schema, ctx, |h, n| h.starts_with(n));
        }
        Function::StrEnds => {
            return eval_string_pred_expr(args, row, schema, ctx, |h, n| h.ends_with(n));
        }
        Function::Regex => return eval_regex_expr(args, row, schema, ctx),
        Function::LangMatches => return eval_lang_matches_expr(args, row, schema, ctx),
        _ => {}
    }

    // Evaluate all arguments first (a missing/unbound argument is a per-function
    // concern handled below; most functions are strict and error on it).
    let mut vals: Vec<Option<TermValue>> = Vec::with_capacity(args.len());
    for a in args {
        vals.push(eval_expr(a, row, schema, ctx)?.map(|t| value_of(ctx, t)));
    }

    match function {
        // ---- type tests (total: never a type error) -----------------------
        Function::IsIri | Function::IsUri => Ok(Some(bool_term(
            ctx,
            matches!(vals.first(), Some(Some(TermValue::Iri(_)))),
        ))),
        Function::IsBlank => Ok(Some(bool_term(
            ctx,
            matches!(vals.first(), Some(Some(TermValue::Blank { .. }))),
        ))),
        Function::IsLiteral => Ok(Some(bool_term(
            ctx,
            matches!(vals.first(), Some(Some(TermValue::Literal { .. }))),
        ))),
        Function::IsNumeric => {
            let numeric =
                matches!(arg(&vals, 0), Some(v) if xsd_of(v).is_some_and(|xv| is_numeric(&xv)));
            Ok(Some(bool_term(ctx, numeric)))
        }
        Function::IsTriple => Ok(Some(bool_term(
            ctx,
            matches!(vals.first(), Some(Some(TermValue::Triple { .. }))),
        ))),

        // ---- term accessors ------------------------------------------------
        Function::Str => match arg(&vals, 0) {
            Some(TermValue::Literal { lexical_form, .. }) => {
                Ok(Some(string_term(ctx, lexical_form)))
            }
            Some(TermValue::Iri(iri)) => Ok(Some(string_term(ctx, iri))),
            _ => Ok(None),
        },
        Function::Lang => match arg(&vals, 0) {
            Some(TermValue::Literal { language, .. }) => Ok(Some(string_term(
                ctx,
                language.as_deref().unwrap_or_default(),
            ))),
            _ => Ok(None),
        },
        // RDF 1.2 base-direction accessors/tests.
        Function::LangDir => match arg(&vals, 0) {
            Some(TermValue::Literal { direction, .. }) => {
                Ok(Some(string_term(ctx, direction.map_or("", |d| d.as_str()))))
            }
            _ => Ok(None),
        },
        // `hasLANG`/`hasLANGDIR` are total over a bound term: false for any term
        // that is not a language-tagged / directional literal (only an unbound
        // argument yields unbound).
        Function::HasLang => match arg(&vals, 0) {
            None => Ok(None),
            Some(TermValue::Literal { language, .. }) => {
                Ok(Some(bool_term(ctx, language.is_some())))
            }
            Some(_) => Ok(Some(bool_term(ctx, false))),
        },
        Function::HasLangDir => match arg(&vals, 0) {
            None => Ok(None),
            Some(TermValue::Literal { direction, .. }) => {
                Ok(Some(bool_term(ctx, direction.is_some())))
            }
            Some(_) => Ok(Some(bool_term(ctx, false))),
        },
        Function::Datatype => match arg(&vals, 0) {
            Some(TermValue::Literal { datatype, .. }) => {
                Ok(Some(intern(ctx, TermValue::Iri(datatype.clone()))))
            }
            _ => Ok(None),
        },

        // ---- string functions ---------------------------------------------
        Function::StrLen => match string_arg(&vals, 0) {
            Some((s, _)) => Ok(Some(integer_term(ctx, s.chars().count() as i64))),
            None => Ok(None),
        },
        Function::UCase => map_string(ctx, &vals, str::to_uppercase),
        Function::LCase => map_string(ctx, &vals, str::to_lowercase),
        Function::Contains => string_pred(ctx, &vals, |h, n| h.contains(n)),
        Function::StrStarts => string_pred(ctx, &vals, |h, n| h.starts_with(n)),
        Function::StrEnds => string_pred(ctx, &vals, |h, n| h.ends_with(n)),
        Function::Concat => eval_concat(ctx, &vals),
        Function::SubStr => eval_substr(ctx, &vals),
        Function::StrBefore => eval_str_before_after(ctx, &vals, true),
        Function::StrAfter => eval_str_before_after(ctx, &vals, false),
        Function::Replace => eval_replace(ctx, &vals),
        Function::Regex => eval_regex(ctx, &vals),
        Function::LangMatches => eval_lang_matches(ctx, &vals),

        // ---- term constructors --------------------------------------------
        Function::Iri | Function::Uri => match arg(&vals, 0) {
            Some(TermValue::Iri(iri)) => Ok(Some(intern(ctx, TermValue::Iri(iri.clone())))),
            Some(TermValue::Literal { lexical_form, .. }) => {
                match resolve_against_base(ctx.base_iri.as_deref(), lexical_form) {
                    Some(resolved) => Ok(Some(intern(ctx, TermValue::Iri(resolved)))),
                    // Relative reference with no base to resolve against — a SPARQL
                    // expression error (unbound), not a silent identity pass-through.
                    None => Ok(None),
                }
            }
            _ => Ok(None),
        },
        Function::StrLang => eval_str_lang(ctx, &vals),
        Function::StrLangDir => eval_str_lang_dir(ctx, &vals),
        Function::StrDt => eval_str_dt(ctx, &vals),
        // BNODE(): always mints a fresh blank node, even called twice in the same
        // solution (contrast BNODE(strExpr) below — SPARQL 1.1 §17.4.2.2).
        Function::BNode if vals.is_empty() => Ok(Some(mint_bnode(ctx))),
        // BNODE(strExpr): the SAME argument string within the SAME query solution
        // (§17.4.2.2) reuses the previously-minted blank; see `ctx.bnode_memo`'s
        // doc for the row-identity mechanism and its scope.
        Function::BNode => {
            let Some((s, _)) = string_arg(&vals, 0) else {
                return Ok(None);
            };
            let key = (ctx.current_row, s);
            if let Some(existing) = ctx.bnode_memo.get(&key) {
                return Ok(Some(*existing));
            }
            let term = mint_bnode(ctx);
            ctx.bnode_memo.insert(key, term);
            Ok(Some(term))
        }

        // ---- RDF 1.2 triple-term functions --------------------------------
        Function::Triple => eval_triple_ctor(ctx, &vals),
        Function::Subject => triple_part(ctx, &vals, |s, _, _| s),
        Function::Predicate => triple_part(ctx, &vals, |_, p, _| p),
        Function::Object => triple_part(ctx, &vals, |_, _, o| o),

        // ---- numeric math functions (ABS/CEIL/FLOOR/ROUND) ----------------
        // All four are strict in one numeric argument; type errors → Ok(None).
        Function::Abs => unary_numeric_fn(ctx, &vals, numeric_abs),
        Function::Ceil => unary_numeric_fn(ctx, &vals, numeric_ceil),
        Function::Floor => unary_numeric_fn(ctx, &vals, numeric_floor),
        Function::Round => unary_numeric_fn(ctx, &vals, numeric_round),

        // ---- ENCODE_FOR_URI -----------------------------------------------
        Function::EncodeForUri => match string_arg(&vals, 0) {
            Some((s, _)) => Ok(Some(string_term(ctx, &encode_for_uri(&s)))),
            None => Ok(None),
        },

        // ---- NOW() --------------------------------------------------------
        Function::Now => Ok(Some(xsd_to_term(ctx, &ctx.now.clone()))),

        // ---- Date/time component extraction --------------------------------
        Function::Year => match arg(&vals, 0).and_then(xsd_of) {
            Some(XsdValue::DateTime(dt)) => Ok(Some(integer_term(ctx, dt.year()))),
            Some(XsdValue::Date(d)) => Ok(Some(integer_term(ctx, d.year()))),
            _ => Ok(None),
        },
        Function::Month => match arg(&vals, 0).and_then(xsd_of) {
            Some(XsdValue::DateTime(dt)) => Ok(Some(integer_term(ctx, i64::from(dt.month())))),
            Some(XsdValue::Date(d)) => Ok(Some(integer_term(ctx, i64::from(d.month())))),
            _ => Ok(None),
        },
        Function::Day => match arg(&vals, 0).and_then(xsd_of) {
            Some(XsdValue::DateTime(dt)) => Ok(Some(integer_term(ctx, i64::from(dt.day())))),
            Some(XsdValue::Date(d)) => Ok(Some(integer_term(ctx, i64::from(d.day())))),
            _ => Ok(None),
        },
        Function::Hours => match arg(&vals, 0).and_then(xsd_of) {
            Some(XsdValue::DateTime(dt)) => Ok(Some(integer_term(ctx, i64::from(dt.hour())))),
            Some(XsdValue::Time(t)) => Ok(Some(integer_term(ctx, i64::from(t.hour())))),
            _ => Ok(None),
        },
        Function::Minutes => match arg(&vals, 0).and_then(xsd_of) {
            Some(XsdValue::DateTime(dt)) => Ok(Some(integer_term(ctx, i64::from(dt.minute())))),
            Some(XsdValue::Time(t)) => Ok(Some(integer_term(ctx, i64::from(t.minute())))),
            _ => Ok(None),
        },
        Function::Seconds => match arg(&vals, 0).and_then(xsd_of) {
            Some(XsdValue::DateTime(dt)) => {
                Ok(Some(xsd_to_term(ctx, &XsdValue::Decimal(dt.second()))))
            }
            Some(XsdValue::Time(t)) => Ok(Some(xsd_to_term(ctx, &XsdValue::Decimal(t.second())))),
            _ => Ok(None),
        },
        Function::Timezone => match arg(&vals, 0).and_then(xsd_of) {
            Some(XsdValue::DateTime(dt)) => match dt.timezone_minutes() {
                Some(off_min) => Ok(Some(intern(
                    ctx,
                    typed(
                        &format_daytime_duration(off_min),
                        "http://www.w3.org/2001/XMLSchema#dayTimeDuration",
                    ),
                ))),
                None => Ok(None), // SPARQL §17.4.5.7: no timezone → error
            },
            Some(XsdValue::Date(d)) => match d.timezone_minutes() {
                Some(off_min) => Ok(Some(intern(
                    ctx,
                    typed(
                        &format_daytime_duration(off_min),
                        "http://www.w3.org/2001/XMLSchema#dayTimeDuration",
                    ),
                ))),
                None => Ok(None),
            },
            Some(XsdValue::Time(t)) => match t.timezone_minutes() {
                Some(off_min) => Ok(Some(intern(
                    ctx,
                    typed(
                        &format_daytime_duration(off_min),
                        "http://www.w3.org/2001/XMLSchema#dayTimeDuration",
                    ),
                ))),
                None => Ok(None),
            },
            _ => Ok(None),
        },
        Function::Tz => match arg(&vals, 0).and_then(xsd_of) {
            Some(XsdValue::DateTime(dt)) => Ok(Some(string_term(
                ctx,
                &format_tz_string(dt.timezone_minutes()),
            ))),
            Some(XsdValue::Date(d)) => Ok(Some(string_term(
                ctx,
                &format_tz_string(d.timezone_minutes()),
            ))),
            Some(XsdValue::Time(t)) => Ok(Some(string_term(
                ctx,
                &format_tz_string(t.timezone_minutes()),
            ))),
            _ => Ok(None),
        },

        // ---- ADJUST(value, timezone) ----------------------------------------
        // The SPARQL 1.2 Query specification's Functions on Dates and Times
        // table (SEP-0002 "Add Support Durations, Dates, and Times") maps
        // ADJUST onto `fn:adjust-dateTime-to-timezone` / `-date-` / `-time-
        // to-timezone` (XPath and XQuery Functions and Operators §9.6); see
        // `Function::Adjust`'s rustdoc and [`adjust_timezone_arg`] for the
        // exact source trail and the SPARQL empty-sequence encoding. Domain
        // errors from the `purrdf-xsd` call (non-whole-minute or out-of-range
        // ±14:00 timezone) fold into `Ok(None)`, same as every other SPARQL
        // expression error.
        Function::Adjust => {
            let value = arg(&vals, 0).and_then(xsd_of);
            let timezone = arg(&vals, 1)
                .and_then(xsd_of)
                .and_then(|v| adjust_timezone_arg(&v))
                .map(AdjustTimezone::into_seconds);
            match (value, timezone) {
                (Some(XsdValue::DateTime(dt)), Some(tz)) => {
                    match purrdf_xsd::temporal::adjust_datetime_to_timezone(&dt, tz) {
                        Ok(result) => Ok(Some(xsd_to_term(ctx, &XsdValue::DateTime(result)))),
                        Err(_) => Ok(None),
                    }
                }
                (Some(XsdValue::Date(d)), Some(tz)) => {
                    match purrdf_xsd::temporal::adjust_date_to_timezone(&d, tz) {
                        Ok(result) => Ok(Some(xsd_to_term(ctx, &XsdValue::Date(result)))),
                        Err(_) => Ok(None),
                    }
                }
                (Some(XsdValue::Time(t)), Some(tz)) => {
                    match purrdf_xsd::temporal::adjust_time_to_timezone(&t, tz) {
                        Ok(result) => Ok(Some(xsd_to_term(ctx, &XsdValue::Time(result)))),
                        Err(_) => Ok(None),
                    }
                }
                _ => Ok(None),
            }
        }

        // ---- hash functions ------------------------------------------------
        Function::Md5 => match string_arg(&vals, 0) {
            Some((s, _)) => {
                let digest = md5::Md5::digest(s.as_bytes());
                Ok(Some(string_term(ctx, &hex_lower(&digest))))
            }
            None => Ok(None),
        },
        Function::Sha1 => match string_arg(&vals, 0) {
            Some((s, _)) => {
                let digest = sha1::Sha1::digest(s.as_bytes());
                Ok(Some(string_term(ctx, &hex_lower(&digest))))
            }
            None => Ok(None),
        },
        Function::Sha256 => match string_arg(&vals, 0) {
            Some((s, _)) => {
                let digest = sha2::Sha256::digest(s.as_bytes());
                Ok(Some(string_term(ctx, &hex_lower(&digest))))
            }
            None => Ok(None),
        },
        Function::Sha384 => match string_arg(&vals, 0) {
            Some((s, _)) => {
                let digest = sha2::Sha384::digest(s.as_bytes());
                Ok(Some(string_term(ctx, &hex_lower(&digest))))
            }
            None => Ok(None),
        },
        Function::Sha512 => match string_arg(&vals, 0) {
            Some((s, _)) => {
                let digest = sha2::Sha512::digest(s.as_bytes());
                Ok(Some(string_term(ctx, &hex_lower(&digest))))
            }
            None => Ok(None),
        },

        // ---- RAND() --------------------------------------------------------
        Function::Rand => {
            let bits = next_u64(ctx);
            // Map to [0,1) double by using the 52 mantissa bits of IEEE 754.
            // Pattern: set exponent to 1023 (1.0), OR in 52 random bits, subtract 1.0.
            let f = f64::from_bits((bits >> 12) | 0x3FF0_0000_0000_0000) - 1.0;
            Ok(Some(xsd_to_term(ctx, &XsdValue::Double(f))))
        }

        // ---- UUID() / STRUUID() -------------------------------------------
        Function::Uuid => {
            let (uuid_iri, _) = make_uuid(ctx);
            let iri_val = format!("urn:uuid:{uuid_iri}");
            Ok(Some(intern(ctx, TermValue::Iri(iri_val))))
        }
        Function::StrUuid => {
            let (uuid_str, _) = make_uuid(ctx);
            Ok(Some(string_term(ctx, &uuid_str)))
        }

        // ---- extension functions (CLOSED, exhaustive) -----------------------
        // Dispatch on the parse-time-resolved kind; the original call IRI in the
        // node is serialization-only.
        Function::Purrdf(call) => match call.fn_kind {
            PurrdfFn::HeldIn => eval_held_in(&vals, ctx),
            // The six `rdf:List` functions (`listLength`, …) — every other
            // extension function is a list function, so this arm is total over
            // the registry.
            list_func => crate::list_fn::dispatch(list_func, &vals, ctx),
        },

        // ---- XSD constructor casts (SPARQL 1.1 §17.1) ---------------------
        // An IRI in call position whose IRI is an XSD value-space datatype is the
        // standard cast constructor (`xsd:decimal(?x)`, `xsd:integer(?x)`, …), NOT an
        // unknown custom function. It builds a target-typed literal from the argument's
        // lexical form (an IRI argument casts to `xsd:string`). A lexical form that is
        // not valid for the target type is a SPARQL expression error (`Ok(None)`).
        Function::Custom(iri) => {
            // A caller-injected SHACL-AF function (`sh:SPARQLFunction`) resolved at
            // eval time — the open counterpart of the closed, parse-time `PurrdfFn`
            // set. `ctx.user_functions` is a `Copy` borrow tied to the dataset
            // lifetime, so reading it out does not borrow `ctx`, leaving `&mut ctx`
            // free for the executor. Checked before the XSD-cast path so a function
            // IRI never collides with a datatype IRI. An EMPTY registry resolves
            // nothing, so this falls through exactly as an absent registry used to.
            if let Some(func) = ctx.user_functions.resolve(iri.as_str()) {
                let result = crate::user_fn::eval_user_function(func, iri.as_str(), &vals, ctx)?;
                return Ok(result.map(|value| intern(ctx, value)));
            }
            // A caller-injected native (host-Rust closure) function, resolved from
            // the same registry's second table. Checked after the SPARQL-bodied
            // path (so a same-registry cross-kind collision can never arise — the
            // registry's collision guard already makes that unrepresentable) and
            // before the XSD-cast fallback, so a function IRI never collides with a
            // datatype IRI.
            if let Some(native) = ctx.user_functions.resolve_native(iri.as_str()) {
                let result = crate::user_fn::eval_native_function(native, iri.as_str(), &vals)?;
                return Ok(result.map(|value| intern(ctx, value)));
            }
            if let Some(target) = XsdDatatype::from_iri(iri.as_str()) {
                return Ok(eval_xsd_cast(ctx, target, arg(&vals, 0)));
            }
            Err(EvalError::unsupported_deferred(
                crate::error::UnsupportedKind::CustomFunction,
                format!("custom SPARQL function <{}>", iri.as_str()),
            ))
        }
    }
}

/// Evaluate an XSD constructor cast: parse the source literal's lexical form against
/// the `target` datatype (an IRI source casts to `xsd:string`), returning the
/// target-typed literal in canonical form, or `None` on a type/lexical error.
///
/// Numeric→numeric casts are value-space, not lexical-space (SPARQL 1.1 §17.1 / the
/// XPath casting rules): `xsd:decimal("5.355e1"^^xsd:double)` is the decimal value
/// `53.55`, NOT a re-parse of the scientific-notation lexical (which is not a valid
/// `xsd:decimal` lexical). The direct lexical parse handles same-representation casts
/// (and string/boolean/temporal targets); when it fails, a numeric-or-boolean source is
/// cast by VALUE through [`cast_numeric_value`] (this also covers `xsd:boolean` as
/// EITHER the source or the target of a numeric cast, per XPath's boolean/numeric
/// casting rules).
///
/// `xsd:string(x)` is handled BEFORE the generic lexical-copy path: casting a
/// `boolean`/numeric source to `xsd:string` is a VALUE-space operation with its own
/// XPath-mandated string form ([`numeric_or_bool_to_xpath_string`]) that is generally
/// NOT the source's own lexical form (e.g. `xsd:string("0"^^xsd:boolean)` is
/// `"false"`, not `"0"`) — only a source with no numeric/boolean value (a plain
/// string, an already-`xsd:string` literal, an unrecognized datatype, …) falls back to
/// copying its lexical form verbatim.
fn eval_xsd_cast<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    target: XsdDatatype,
    source: Option<&TermValue>,
) -> Option<SolutionTerm<D::Id>> {
    let source = source?;
    if target == XsdDatatype::String {
        if let TermValue::Iri(iri) = source {
            return Some(string_term(ctx, iri));
        }
        if let Some(s) = xsd_of(source)
            .as_ref()
            .and_then(numeric_or_bool_to_xpath_string)
        {
            return Some(string_term(ctx, &s));
        }
    }
    let lexical = match source {
        TermValue::Literal { lexical_form, .. } => lexical_form.clone(),
        _ => return None,
    };
    // The operand-mapping rules pin XSD 1.0, so a `xsd:float`/`xsd:double` constructor
    // rejects the XSD 1.1-only `+INF` spelling (only `INF`); other targets are
    // unaffected (`parse_xsd10` delegates to `parse`).
    if let Ok(value) = parse_xsd10(&lexical, target) {
        return Some(xsd_to_term(ctx, &value));
    }
    // The lexical is not directly valid for `target`. If both source and target are
    // numeric-or-boolean, convert by value (e.g. a `double`/`float` scientific-notation
    // lexical into the equivalent `decimal`/`integer`, or a numeric source into
    // `xsd:boolean`), matching the spec's casting tower.
    let value = cast_numeric_value(&xsd_of(source)?, target)?;
    Some(xsd_to_term(ctx, &value))
}

/// Cast a numeric-or-boolean [`XsdValue`] to a numeric-or-`xsd:boolean` `target`
/// datatype **by value** (the SPARQL §17.1 / XPath casting rules): the source's value
/// is re-expressed in the target's value space. Returns `None` when the source has no
/// numeric value, the target is not numeric/boolean, or the value is out of the
/// target's range (e.g. a non-integral double cast to integer truncates toward zero,
/// as XPath `xs:integer` mandates).
///
/// `xsd:boolean` participates on BOTH sides: a boolean source is `1.0`/`0.0` for a
/// numeric target, and a numeric target of `xsd:boolean` is XPath's numeric effective
/// boolean value (zero or `NaN` → `false`, else `true`) — the same rule SPARQL's own
/// effective boolean value uses for numerics ([`effective_boolean_value`]).
fn cast_numeric_value(source: &XsdValue, target: XsdDatatype) -> Option<XsdValue> {
    use purrdf_xsd::parse as xsd_parse;
    // The source's exact numeric value, as the widest faithful form available.
    let as_f64 = match source {
        XsdValue::Integer { value, .. } => *value as f64,
        XsdValue::Decimal(d) => d.to_f64(),
        XsdValue::Float(f) => f64::from(*f),
        XsdValue::Double(d) => *d,
        XsdValue::Boolean(b) => f64::from(u8::from(*b)),
        _ => return None,
    };
    match target {
        XsdDatatype::Double => Some(XsdValue::Double(as_f64)),
        XsdDatatype::Float => Some(XsdValue::Float(as_f64 as f32)),
        // Zero or NaN is false; every other numeric value (including negatives and
        // subnormals) is true — XPath's numeric-to-boolean casting rule.
        XsdDatatype::Boolean => Some(XsdValue::Boolean(as_f64 != 0.0 && !as_f64.is_nan())),
        XsdDatatype::Decimal => {
            // A non-finite double has no decimal value (a SPARQL expression error).
            if !as_f64.is_finite() {
                return None;
            }
            // Re-express the value as a plain (exponent-free) decimal lexical the
            // decimal parser accepts, bounded to the 18-digit scale it allows.
            xsd_parse(&format_plain_decimal(as_f64), XsdDatatype::Decimal).ok()
        }
        // An integer target truncates toward zero (XPath `xs:integer(double)`), within
        // the i128 range the integer value space supports.
        XsdDatatype::Integer
        | XsdDatatype::Long
        | XsdDatatype::Int
        | XsdDatatype::Short
        | XsdDatatype::Byte
        | XsdDatatype::UnsignedLong
        | XsdDatatype::UnsignedInt
        | XsdDatatype::UnsignedShort
        | XsdDatatype::UnsignedByte
        | XsdDatatype::NonNegativeInteger
        | XsdDatatype::PositiveInteger
        | XsdDatatype::NonPositiveInteger
        | XsdDatatype::NegativeInteger => {
            let truncated = as_f64.trunc();
            if !truncated.is_finite() {
                return None;
            }
            // Re-parse the integral lexical against the exact integer target so its
            // range constraints (e.g. `nonNegativeInteger >= 0`) are enforced.
            xsd_parse(&format!("{truncated:.0}"), target).ok()
        }
        _ => None,
    }
}

/// The XPath F&O §19 "Casting to `xs:string`" string form of a boolean or numeric
/// value — DISTINCT from the value's XSD canonical **literal** lexical mapping (which
/// [`XsdValue::canonical_lexical`] provides for writing an actual `xsd:double`/
/// `xsd:float` term). Returns `None` for a non-numeric, non-boolean value (the caller
/// then falls back to copying the source's own lexical form).
///
/// - `xsd:boolean` → `"true"`/`"false"`.
/// - `xsd:integer` (and derived) → the plain decimal digits (no fractional part ever).
/// - `xsd:decimal` → its XSD 1.1 canonical lexical form directly: an integer-valued
///   decimal already has no decimal point there (`1.0` → `"1"`), so the cast-to-string
///   and the literal serialization share ONE decimal-formatting path.
/// - `xsd:float`/`xsd:double` → [`xpath_double_to_xpath_string`] (plain decimal
///   notation in the "ordinary" magnitude range, scientific outside it).
fn numeric_or_bool_to_xpath_string(value: &XsdValue) -> Option<String> {
    match value {
        XsdValue::Boolean(b) => Some(if *b { "true" } else { "false" }.to_owned()),
        XsdValue::Integer { value, .. } => Some(value.to_string()),
        XsdValue::Decimal(d) => Some(d.canonical_lexical()),
        XsdValue::Float(f) => Some(xpath_double_to_xpath_string(f64::from(*f))),
        XsdValue::Double(d) => Some(xpath_double_to_xpath_string(*d)),
        _ => None,
    }
}

/// XPath F&O's number→`xs:string` casting algorithm for `xs:float`/`xs:double`: values
/// with an absolute magnitude in `[0.000001, 100000000)` are written in plain
/// (non-exponential) decimal notation; every other finite value uses scientific
/// notation (`mantissa Eexponent`, no padding). This is intentionally NOT the XSD
/// canonical literal mapping ([`purrdf_xsd::numeric::canonical_double`]), which always
/// uses mandatory exponential notation — this is the distinct, narrower rule XPath
/// specifies for the STRING VALUE of a numeric cast.
fn xpath_double_to_xpath_string(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_owned();
    }
    if value.is_infinite() {
        return if value > 0.0 { "INF" } else { "-INF" }.to_owned();
    }
    if value == 0.0 {
        return if value.is_sign_negative() { "-0" } else { "0" }.to_owned();
    }
    let abs = value.abs();
    if (1e-6..1e8).contains(&abs) {
        format_plain_decimal(value)
    } else {
        // Scientific notation, without the XSD-canonical mandatory ".0" mantissa pad
        // this cast rule doesn't require.
        let raw = format!("{value:e}");
        let (mantissa, exp) = raw.split_once('e').unwrap_or((raw.as_str(), "0"));
        format!("{mantissa}E{exp}")
    }
}

/// Format a finite `f64` as a plain, exponent-free decimal lexical with at most 18
/// fractional digits (the `xsd:decimal` scale bound), trimming trailing fractional
/// zeros. Used by the numeric value-space cast into `xsd:decimal`.
fn format_plain_decimal(value: f64) -> String {
    // `{:.18}` never emits scientific notation and caps the fraction at the decimal
    // scale bound; trim trailing zeros (and a bare trailing point) for a clean lexical.
    let raw = format!("{value:.18}");
    let trimmed = if raw.contains('.') {
        raw.trim_end_matches('0').trim_end_matches('.')
    } else {
        raw.as_str()
    };
    if trimmed.is_empty() || trimmed == "-" {
        "0".to_owned()
    } else {
        trimmed.to_owned()
    }
}

/// `heldIn(reifier, standpoint) -> xsd:boolean` — DIRECT, non-transitive
/// standpoint membership over the already-reasoned dataset.
///
/// The standpoint vocabulary is **caller configuration**, never an engine
/// constant: the `accordingTo`/`sharpens` predicates are domain terms from the
/// caller's ontology (gmeow's, …) read
/// from the caller's data, supplied as a
/// [`crate::eval::StandpointPredicates`] table. Evaluating `heldIn` with **no**
/// configured table is a hard [`EvalError`] — there is no fabricated default.
///
/// The reasoning authority is the entailment lane, not this builtin: it does NOT
/// walk/compute the `sharpens` transitive closure — it relies on the closure being
/// materialized upstream as direct edges. It returns
/// true iff some vantage standpoint `T` of the reifier (the objects of the reifier's
/// `accordingTo` annotations) either equals the queried standpoint or has a
/// direct `(T, sharpens, standpoint)` quad (T is more specific than the
/// queried standpoint, so a claim held in T counts as held in the broader one).
///
/// Three-valued: an unbound argument yields `Ok(None)` (a SPARQL error). An argument
/// absent from the dataset is a well-formed negative answer — `Ok(Some(false))`, not
/// `None`. Missing `accordingTo`/`sharpens` interning simply yields no
/// matches (→ false), which is correct.
fn eval_held_in<D: DatasetView + Sync>(
    vals: &[Option<TermValue>],
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    // The predicate table is mandatory configuration — fail loudly BEFORE looking at
    // the arguments, so a misconfigured deployment cannot get a quietly-wrong answer.
    let Some(predicates) = ctx.standpoint_predicates.clone() else {
        return Err(EvalError::unsupported_deferred(
            crate::error::UnsupportedKind::HeldInUnconfigured,
            "heldIn requires a standpoint predicate configuration: supply the \
             ontology's accordingTo/sharpens IRIs via \
             NativeSparqlEngine::with_standpoint_predicates (or \
             EvalCtx::with_standpoint_predicates); there is no built-in default",
        ));
    };

    // Strict in both args: an unbound/error argument is a SPARQL error (None).
    let (Some(reifier_val), Some(standpoint_val)) = (arg(vals, 0), arg(vals, 1)) else {
        return Ok(None);
    };

    // A term absent from the dataset cannot participate in any quad/annotation, so the
    // function is a clean, well-formed FALSE (not an error).
    let (Some(reifier_id), Some(standpoint_id)) = (
        ctx.dataset.term_id_by_value(reifier_val),
        ctx.dataset.term_id_by_value(standpoint_val),
    ) else {
        return Ok(Some(bool_term(ctx, false)));
    };

    let according_to_id = ctx
        .dataset
        .term_id_by_value(&TermValue::Iri(predicates.according_to));
    let sharpens_id = ctx
        .dataset
        .term_id_by_value(&TermValue::Iri(predicates.sharpens));

    // The reifier's vantage standpoint(s): annotation objects under the configured
    // `accordingTo`. If it was never interned, there are no vantage standpoints.
    let held = according_to_id.is_some_and(|atid| {
        ctx.dataset
            .annotations_of_with_graph(reifier_id)
            .filter(|(pred, _, _)| *pred == atid)
            .map(|(_, vantage, _)| vantage)
            .any(|vantage| {
                // Held directly in the queried standpoint, …
                vantage == standpoint_id
                    // … or in a standpoint that sharpens (is more specific than) it.
                    || sharpens_id.is_some_and(|spid| {
                        ctx.dataset
                            .quads_for_pattern(
                                Some(vantage),
                                Some(spid),
                                Some(standpoint_id),
                                GraphMatch::Default,
                            )
                            .next()
                            .is_some()
                    })
            })
    });

    Ok(Some(bool_term(ctx, held)))
}

/// The value at argument index `i`, if it was bound (not unbound/error).
fn arg(vals: &[Option<TermValue>], i: usize) -> Option<&TermValue> {
    vals.get(i).and_then(|v| v.as_ref())
}

fn eval_string_pred_expr<D: DatasetView + Sync>(
    args: &[Expression],
    row: &[Option<SolutionTerm<D::Id>>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_, D>,
    f: impl Fn(&str, &str) -> bool,
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    let result = {
        let (Some((h, _)), Some((n, _))) = (
            eval_string_arg_expr(args.first(), row, schema, ctx)?,
            eval_string_arg_expr(args.get(1), row, schema, ctx)?,
        ) else {
            return Ok(None);
        };
        f(&h, &n)
    };
    Ok(Some(bool_term(ctx, result)))
}

fn eval_regex_expr<D: DatasetView + Sync>(
    args: &[Expression],
    row: &[Option<SolutionTerm<D::Id>>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    let text = eval_string_arg_expr(args.first(), row, schema, ctx)?;
    let pattern = eval_string_arg_expr(args.get(1), row, schema, ctx)?;
    let flags = eval_string_arg_expr(args.get(2), row, schema, ctx)?;
    let (Some((text, _)), Some((pattern, _))) = (text, pattern) else {
        return Ok(None);
    };
    let flags = flags.map(|(f, _)| f).unwrap_or_default();
    match cached_regex(ctx, &pattern, &flags) {
        Some(re) => Ok(Some(bool_term(ctx, re.is_match(&text)))),
        None => Ok(None),
    }
}

fn eval_lang_matches_expr<D: DatasetView + Sync>(
    args: &[Expression],
    row: &[Option<SolutionTerm<D::Id>>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    let result = {
        let (Some((tag, _)), Some((range, _))) = (
            eval_string_arg_expr(args.first(), row, schema, ctx)?,
            eval_string_arg_expr(args.get(1), row, schema, ctx)?,
        ) else {
            return Ok(None);
        };
        let tag = tag.to_ascii_lowercase();
        let range = range.to_ascii_lowercase();
        range == "*" || tag == range || tag.starts_with(&(range + "-"))
    };
    Ok(Some(bool_term(ctx, result)))
}

fn eval_string_arg_expr<D: DatasetView + Sync>(
    expr: Option<&Expression>,
    row: &[Option<SolutionTerm<D::Id>>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Option<(String, Option<String>)>, EvalError> {
    let Some(expr) = expr else {
        return Ok(None);
    };
    match expr {
        Expression::Literal(lit)
            if lit.datatype().as_str() == XSD_STRING
                || lit.datatype().as_str() == RDF_LANG_STRING =>
        {
            Ok(Some((
                lit.value().to_owned(),
                lit.language().map(str::to_ascii_lowercase),
            )))
        }
        Expression::FunctionCall(Function::Str, inner) if inner.len() == 1 => {
            eval_str_lexical_expr(&inner[0], row, schema, ctx).map(|v| v.map(|s| (s, None)))
        }
        Expression::FunctionCall(Function::Lang, inner) if inner.len() == 1 => {
            eval_lang_lexical_expr(&inner[0], row, schema, ctx).map(|v| v.map(|s| (s, None)))
        }
        _ => {
            let Some(term) = eval_expr(expr, row, schema, ctx)? else {
                return Ok(None);
            };
            let value = value_of(ctx, term);
            Ok(string_arg_value(&value).map(|(s, l, _)| (s, l)))
        }
    }
}

fn eval_str_lexical_expr<D: DatasetView + Sync>(
    expr: &Expression,
    row: &[Option<SolutionTerm<D::Id>>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Option<String>, EvalError> {
    match expr {
        Expression::NamedNode(node) => Ok(Some(node.as_str().to_owned())),
        Expression::Literal(lit) => Ok(Some(lit.value().to_owned())),
        _ => {
            let Some(term) = eval_expr(expr, row, schema, ctx)? else {
                return Ok(None);
            };
            Ok(str_lexical_term(ctx, term))
        }
    }
}

fn eval_lang_lexical_expr<D: DatasetView + Sync>(
    expr: &Expression,
    row: &[Option<SolutionTerm<D::Id>>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Option<String>, EvalError> {
    match expr {
        Expression::Literal(lit) => Ok(Some(
            lit.language()
                .map(str::to_ascii_lowercase)
                .unwrap_or_default(),
        )),
        _ => {
            let Some(term) = eval_expr(expr, row, schema, ctx)? else {
                return Ok(None);
            };
            Ok(lang_lexical_term(ctx, term))
        }
    }
}

fn str_lexical_term<D: DatasetView + Sync>(
    ctx: &EvalCtx<'_, D>,
    term: SolutionTerm<D::Id>,
) -> Option<String> {
    match term {
        SolutionTerm::Existing(id) => match ctx.dataset.resolve(id) {
            TermRef::Iri(iri) => Some(iri.to_owned()),
            TermRef::Literal { lexical, .. } => Some(lexical.to_owned()),
            TermRef::Blank { .. } | TermRef::Triple { .. } => None,
        },
        SolutionTerm::Computed(_) => match value_of(ctx, term) {
            TermValue::Iri(iri) => Some(iri),
            TermValue::Literal { lexical_form, .. } => Some(lexical_form),
            TermValue::Blank { .. } | TermValue::Triple { .. } => None,
        },
    }
}

fn lang_lexical_term<D: DatasetView + Sync>(
    ctx: &EvalCtx<'_, D>,
    term: SolutionTerm<D::Id>,
) -> Option<String> {
    match term {
        SolutionTerm::Existing(id) => match ctx.dataset.resolve(id) {
            TermRef::Literal { language, .. } => Some(language.unwrap_or_default().to_owned()),
            _ => None,
        },
        SolutionTerm::Computed(_) => match value_of(ctx, term) {
            TermValue::Literal { language, .. } => Some(language.unwrap_or_default()),
            _ => None,
        },
    }
}

/// Whether an XSD value is in the numeric tower.
fn is_numeric(v: &XsdValue) -> bool {
    matches!(
        v,
        XsdValue::Integer { .. } | XsdValue::Decimal(_) | XsdValue::Float(_) | XsdValue::Double(_)
    )
}

/// Extract `(lexical, language)` from a plain/`xsd:string`/`rdf:langString` literal
/// argument. `None` for any other term (a string-function type error).
fn string_arg(vals: &[Option<TermValue>], i: usize) -> Option<(String, Option<String>)> {
    string_arg_value(arg(vals, i)?).map(|(s, l, _)| (s, l))
}

/// Extract the lexical form of a *plain* string argument — a simple literal or
/// an explicitly `xsd:string`-typed one — for built-ins whose first argument
/// must NOT already carry a language tag (`STRLANG`/`STRDT`, SPARQL 1.1
/// §17.4.2.4/§17.4.2.5). Unlike [`string_arg`], a `rdf:langString` (or RDF 1.2
/// `rdf:dirLangString`) argument is a type error here, not an accepted input
/// whose language would silently be discarded.
fn plain_string_arg(vals: &[Option<TermValue>], i: usize) -> Option<String> {
    match arg(vals, i)? {
        TermValue::Literal {
            lexical_form,
            datatype,
            ..
        } if datatype == XSD_STRING => Some(lexical_form.clone()),
        _ => None,
    }
}

/// Like [`string_arg`] but also returns the RDF 1.2 base direction (for functions
/// that must preserve or inspect it, e.g. `CONCAT`).
fn string_arg3(
    vals: &[Option<TermValue>],
    i: usize,
) -> Option<(String, Option<String>, Option<RdfTextDirection>)> {
    string_arg_value(arg(vals, i)?)
}

fn string_arg_value(
    value: &TermValue,
) -> Option<(String, Option<String>, Option<RdfTextDirection>)> {
    match value {
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } if datatype == XSD_STRING
            || datatype == RDF_LANG_STRING
            || datatype == RDF_DIR_LANG_STRING =>
        {
            Some((lexical_form.clone(), language.clone(), *direction))
        }
        _ => None,
    }
}

/// Apply a pure string transform to a single string argument, preserving its
/// language tag.
fn map_string<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    vals: &[Option<TermValue>],
    f: impl Fn(&str) -> String,
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    match string_arg(vals, 0) {
        Some((s, lang)) => Ok(Some(make_string(ctx, f(&s), lang))),
        None => Ok(None),
    }
}

/// A two-string boolean predicate (CONTAINS/STRSTARTS/STRENDS).
fn string_pred<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    vals: &[Option<TermValue>],
    f: impl Fn(&str, &str) -> bool,
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    match (string_arg(vals, 0), string_arg(vals, 1)) {
        (Some((h, _)), Some((n, _))) => Ok(Some(bool_term(ctx, f(&h, &n)))),
        _ => Ok(None),
    }
}

/// Intern a string literal, as `rdf:langString@lang` if a language is present, else
/// `xsd:string`.
fn make_string<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    lexical: String,
    lang: Option<String>,
) -> SolutionTerm<D::Id> {
    match lang {
        Some(l) => intern(
            ctx,
            TermValue::Literal {
                lexical_form: lexical,
                datatype: RDF_LANG_STRING.to_owned(),
                language: Some(l),
                direction: None,
            },
        ),
        None => string_term(ctx, &lexical),
    }
}

/// Intern a string literal keeping a language tag and (RDF 1.2) base direction:
/// `rdf:dirLangString` when both are present, `rdf:langString` when only a
/// language is, else `xsd:string`. A direction without a language is dropped.
fn make_string_dir<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    lexical: String,
    lang: Option<String>,
    dir: Option<RdfTextDirection>,
) -> SolutionTerm<D::Id> {
    match (lang, dir) {
        (Some(l), Some(d)) => intern(
            ctx,
            TermValue::Literal {
                lexical_form: lexical,
                datatype: RDF_DIR_LANG_STRING.to_owned(),
                language: Some(l),
                direction: Some(d),
            },
        ),
        (Some(l), None) => make_string(ctx, lexical, Some(l)),
        (None, _) => string_term(ctx, &lexical),
    }
}

/// `CONCAT(...)`: concatenate string arguments. The result keeps the language tag
/// **and** base direction iff *every* argument shares the same `(lang, dir)` pair;
/// if either facet differs across arguments the result is a plain `xsd:string`.
fn eval_concat<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    vals: &[Option<TermValue>],
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    let mut out = String::new();
    let mut common: Option<(Option<String>, Option<RdfTextDirection>)> = None;
    let mut consistent = true;
    for i in 0..vals.len() {
        let Some((s, lang, dir)) = string_arg3(vals, i) else {
            return Ok(None);
        };
        out.push_str(&s);
        match &common {
            None => common = Some((lang, dir)),
            Some((cl, cd)) if *cl == lang && *cd == dir => {}
            Some(_) => consistent = false,
        }
    }
    match common {
        Some((lang, dir)) if consistent => Ok(Some(make_string_dir(ctx, out, lang, dir))),
        _ => Ok(Some(string_term(ctx, &out))),
    }
}

/// `SUBSTR(str, start[, length])` with 1-based indexing over Unicode scalars.
fn eval_substr<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    vals: &[Option<TermValue>],
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    let Some((s, lang)) = string_arg(vals, 0) else {
        return Ok(None);
    };
    let Some(start) = arg(vals, 1).and_then(xsd_int_of) else {
        return Ok(None);
    };
    let chars: Vec<char> = s.chars().collect();
    // SPARQL substr is 1-based; clamp to the string bounds.
    let start0 = (start - 1).max(0) as usize;
    let end = match vals.get(2).and_then(|v| v.as_ref()) {
        Some(len_val) => {
            let Some(len) = xsd_int_of(len_val) else {
                return Ok(None);
            };
            ((start - 1).max(0) + len.max(0)) as usize
        }
        None => chars.len(),
    };
    let slice: String = chars
        .get(start0..end.min(chars.len()))
        .unwrap_or(&[])
        .iter()
        .collect();
    Ok(Some(make_string(ctx, slice, lang)))
}

/// SPARQL 1.1 §17.4.1.1 "argument compatibility": whether a string operand
/// tagged `arg1_lang` may be compared against one tagged `arg2_lang`.
/// Compatible when: both are simple/`xsd:string` (no language); both carry the
/// *same* language tag (compared case-insensitively per RFC 4646); or `arg1`
/// has a language tag and `arg2` is simple/`xsd:string`. NOT compatible the
/// other way around (`arg1` simple, `arg2` tagged) — a plain string cannot be
/// searched for a language-tagged pattern.
fn args_compatible(arg1_lang: Option<&str>, arg2_lang: Option<&str>) -> bool {
    match (arg1_lang, arg2_lang) {
        (None, None) => true,
        (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
        (Some(_), None) => true,
        (None, Some(_)) => false,
    }
}

/// `STRBEFORE`/`STRAFTER(haystack, needle)`.
fn eval_str_before_after<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    vals: &[Option<TermValue>],
    before: bool,
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    let (Some((h, lang)), Some((n, needle_lang))) = (string_arg(vals, 0), string_arg(vals, 1))
    else {
        return Ok(None);
    };
    // §17.4.1.1: the needle must be argument-compatible with the haystack, or
    // the call is a type error (unbound) — e.g. a `@cy`-tagged needle can never
    // match an untagged or `@en`-tagged haystack.
    if !args_compatible(lang.as_deref(), needle_lang.as_deref()) {
        return Ok(None);
    }
    // An empty needle matches at the start: STRBEFORE → "", STRAFTER → the haystack.
    let result = match h.find(&n) {
        Some(idx) => {
            if before {
                h[..idx].to_owned()
            } else {
                h[idx + n.len()..].to_owned()
            }
        }
        // No match → empty (typed xsd:string, no language).
        None => return Ok(Some(string_term(ctx, ""))),
    };
    Ok(Some(make_string(ctx, result, lang)))
}

/// `REPLACE(str, pattern, replacement[, flags])` via the regex engine.
fn eval_replace<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    vals: &[Option<TermValue>],
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    let Some((s, lang)) = string_arg(vals, 0) else {
        return Ok(None);
    };
    let (Some((pattern, _)), Some((replacement, _))) = (string_arg(vals, 1), string_arg(vals, 2))
    else {
        return Ok(None);
    };
    let flags = string_arg(vals, 3).map(|(f, _)| f).unwrap_or_default();
    let Some(re) = cached_regex(ctx, &pattern, &flags) else {
        return Ok(None);
    };
    // SPARQL uses $N for capture-group references — same as the regex crate.
    let replaced = re.replace_all(&s, replacement.as_str()).into_owned();
    Ok(Some(make_string(ctx, replaced, lang)))
}

/// `REGEX(text, pattern[, flags])`.
fn eval_regex<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    vals: &[Option<TermValue>],
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    let Some((text, _)) = string_arg(vals, 0) else {
        return Ok(None);
    };
    let Some((pattern, _)) = string_arg(vals, 1) else {
        return Ok(None);
    };
    let flags = string_arg(vals, 2).map(|(f, _)| f).unwrap_or_default();
    match cached_regex(ctx, &pattern, &flags) {
        Some(re) => Ok(Some(bool_term(ctx, re.is_match(&text)))),
        None => Ok(None),
    }
}

/// The compiled regex for `(pattern, flags)`, from the per-query cache.
///
/// The hit path probes with the **borrowed** strings (the two-level map avoids
/// allocating a `(String, String)` key per row) and returns an `Arc` clone — the
/// rows of one filter share a single compiled regex and therefore its lazy-DFA
/// cache pool, instead of each row cloning a fresh one. Compile failures are
/// cached as `None` (same errors, compiled once).
fn cached_regex<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    pattern: &str,
    flags: &str,
) -> Option<Arc<regex::Regex>> {
    if let Some(cached) = ctx
        .regex_cache
        .get(pattern)
        .and_then(|by_flags| by_flags.get(flags))
    {
        return cached.clone();
    }
    let compiled = build_regex(pattern, flags).map(Arc::new);
    ctx.regex_cache
        .entry(pattern.to_owned())
        .or_default()
        .insert(flags.to_owned(), compiled.clone());
    compiled
}

/// Build a regex from a SPARQL pattern + flag string (`i`, `s`, `m`, `x`).
fn build_regex(pattern: &str, flags: &str) -> Option<regex::Regex> {
    let mut builder = regex::RegexBuilder::new(pattern);
    for f in flags.chars() {
        match f {
            'i' => builder.case_insensitive(true),
            's' => builder.dot_matches_new_line(true),
            'm' => builder.multi_line(true),
            'x' => builder.ignore_whitespace(true),
            _ => return None,
        };
    }
    builder.build().ok()
}

/// `langMatches(tag, range)` — RFC 4647 basic filtering (`*` matches any tag).
fn eval_lang_matches<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    vals: &[Option<TermValue>],
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    let (Some((tag, _)), Some((range, _))) = (string_arg(vals, 0), string_arg(vals, 1)) else {
        return Ok(None);
    };
    let tag = tag.to_ascii_lowercase();
    let range = range.to_ascii_lowercase();
    let matches = if range == "*" {
        !tag.is_empty()
    } else {
        tag == range || tag.starts_with(&format!("{range}-"))
    };
    Ok(Some(bool_term(ctx, matches)))
}

/// `STRLANG(lexical, lang)`.
fn eval_str_lang<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    vals: &[Option<TermValue>],
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    // §17.4.2.5: the lexical-form argument must be a simple/`xsd:string` literal
    // — one that ALREADY carries a language tag (or RDF 1.2 base direction) is a
    // type error, not silently re-tagged.
    let (Some(lex), Some((lang, _))) = (plain_string_arg(vals, 0), string_arg(vals, 1)) else {
        return Ok(None);
    };
    if lang.is_empty() {
        return Ok(None); // an empty language tag is not a valid rdf:langString
    }
    Ok(Some(make_string(ctx, lex, Some(lang.to_ascii_lowercase()))))
}

/// `STRLANGDIR(lexical, lang, dir)` — RDF 1.2 directional-language-string
/// constructor. An empty `dir` yields a plain `rdf:langString`; `ltr`/`rtl`
/// (case-insensitive) yield an `rdf:dirLangString`; any other direction errors.
fn eval_str_lang_dir<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    vals: &[Option<TermValue>],
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    let (Some((lex, _)), Some((lang, _)), Some((dir, _))) = (
        string_arg(vals, 0),
        string_arg(vals, 1),
        string_arg(vals, 2),
    ) else {
        return Ok(None);
    };
    if lang.is_empty() {
        return Ok(None); // a directional language string needs a language
    }
    // The base direction must be exactly `ltr`/`rtl` (case-sensitive); anything
    // else, including an empty string, is a type error (unbound).
    let direction = match dir.as_str() {
        "ltr" => RdfTextDirection::Ltr,
        "rtl" => RdfTextDirection::Rtl,
        _ => return Ok(None),
    };
    Ok(Some(intern(
        ctx,
        TermValue::Literal {
            lexical_form: lex,
            datatype: RDF_DIR_LANG_STRING.to_owned(),
            language: Some(lang.to_ascii_lowercase()),
            direction: Some(direction),
        },
    )))
}

/// `STRDT(lexical, datatypeIri)`.
fn eval_str_dt<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    vals: &[Option<TermValue>],
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    // §17.4.2.4: the lexical-form argument must be a simple/`xsd:string` literal
    // — a language-tagged (or RDF 1.2 direction-tagged) argument is a type error.
    let Some(lex) = plain_string_arg(vals, 0) else {
        return Ok(None);
    };
    let Some(TermValue::Iri(dt)) = arg(vals, 1) else {
        return Ok(None);
    };
    Ok(Some(intern(ctx, typed(&lex, dt))))
}

/// `TRIPLE(s, p, o)` — RDF 1.2 triple-term constructor.
fn eval_triple_ctor<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    vals: &[Option<TermValue>],
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    let (Some(s), Some(p), Some(o)) = (arg(vals, 0), arg(vals, 1), arg(vals, 2)) else {
        return Ok(None);
    };
    // A triple term's subject must be an IRI or blank node and its predicate an
    // IRI. Under RDF 1.2 a triple term may nest only in *object* position, so a
    // triple term (or literal) in the subject/predicate slot is a type error, as
    // is a literal predicate — all of which yield an unbound result.
    if !matches!(s, TermValue::Iri(_) | TermValue::Blank { .. }) || !matches!(p, TermValue::Iri(_))
    {
        return Ok(None);
    }
    let triple = TermValue::Triple {
        s: Box::new(s.clone()),
        p: Box::new(p.clone()),
        o: Box::new(o.clone()),
    };
    Ok(Some(intern(ctx, triple)))
}

/// Extract a component of a triple term (`SUBJECT`/`PREDICATE`/`OBJECT`).
fn triple_part<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    vals: &[Option<TermValue>],
    pick: impl Fn(TermValue, TermValue, TermValue) -> TermValue,
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    match arg(vals, 0) {
        Some(TermValue::Triple { s, p, o }) => {
            let part = pick((**s).clone(), (**p).clone(), (**o).clone());
            Ok(Some(intern(ctx, part)))
        }
        _ => Ok(None),
    }
}

/// An `i64` from an XSD integer argument value.
fn xsd_int_of(v: &TermValue) -> Option<i64> {
    match xsd_of(v)? {
        XsdValue::Integer { value, .. } => i64::try_from(value).ok(),
        _ => None,
    }
}

/// Convert a computed [`XsdValue`] back into an interned [`SolutionTerm`] using the
/// canonical typed-literal form. The datatype IRI comes from `v.datatype().iri()`.
pub(crate) fn xsd_to_term<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    v: &XsdValue,
) -> SolutionTerm<D::Id> {
    intern(ctx, xsd_literal_value(v))
}

/// [`xsd_to_term`]'s value-only half: the canonical typed-literal [`TermValue`] for a
/// computed [`XsdValue`], with no interning. Shared with `crate::modifier`'s built-in
/// aggregate accumulators, whose [`crate::agg_fn::AggregateAccumulator::finish`] produces
/// a plain `TermValue` (interned once, by the caller, exactly as a custom aggregate's own
/// `finish` is) rather than an [`EvalCtx`]-bound [`SolutionTerm`].
pub(crate) fn xsd_literal_value(v: &XsdValue) -> TermValue {
    typed(&v.canonical_lexical(), v.datatype().iri())
}

/// Evaluate a binary value-space expression: resolve both operands to [`XsdValue`],
/// call `op`, and return `Ok(Some(term))` on success or `Ok(None)` on any error (type
/// error, overflow, divide-by-zero, indeterminate timezone mix — all SPARQL
/// expression errors).
fn binary_value<D: DatasetView + Sync>(
    a: &Expression,
    b: &Expression,
    row: &[Option<SolutionTerm<D::Id>>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_, D>,
    op: impl Fn(&XsdValue, &XsdValue) -> Result<XsdValue, purrdf_xsd::XsdError>,
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    let (Some(ta), Some(tb)) = (
        eval_expr(a, row, schema, ctx)?,
        eval_expr(b, row, schema, ctx)?,
    ) else {
        return Ok(None);
    };
    let (Some(xa), Some(xb)) = (xsd_of_term(ctx, ta), xsd_of_term(ctx, tb)) else {
        return Ok(None); // operand with no XSD value
    };
    match op(&xa, &xb) {
        Ok(result) => Ok(Some(xsd_to_term(ctx, &result))),
        Err(_) => Ok(None), // overflow / div-by-zero / type-mismatch → expression error
    }
}

/// Evaluate a unary numeric expression (`+` / `-`): resolve the operand, call `op`,
/// return `Ok(None)` on any error.
fn unary_numeric<D: DatasetView + Sync>(
    a: &Expression,
    row: &[Option<SolutionTerm<D::Id>>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_, D>,
    op: impl Fn(&XsdValue) -> Result<XsdValue, purrdf_xsd::XsdError>,
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    let Some(ta) = eval_expr(a, row, schema, ctx)? else {
        return Ok(None);
    };
    let Some(xa) = xsd_of_term(ctx, ta) else {
        return Ok(None);
    };
    match op(&xa) {
        Ok(result) => Ok(Some(xsd_to_term(ctx, &result))),
        Err(_) => Ok(None),
    }
}

/// Apply a unary numeric function from the `vals` pre-evaluated argument list.
/// Argument 0 must be a numeric literal; type errors → `Ok(None)`.
fn unary_numeric_fn<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    vals: &[Option<TermValue>],
    op: impl Fn(&XsdValue) -> Result<XsdValue, purrdf_xsd::XsdError>,
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    let Some(xa) = arg(vals, 0).and_then(xsd_of) else {
        return Ok(None);
    };
    match op(&xa) {
        Ok(result) => Ok(Some(xsd_to_term(ctx, &result))),
        Err(_) => Ok(None),
    }
}

// ---------------------------------------------------------------------------
// Gap 4 helper functions
// ---------------------------------------------------------------------------

/// Splitmix64 step: advance the PRNG state and return the next pseudo-random u64.
/// Algorithm: <https://prng.di.unimi.it/splitmix64.c>
fn next_u64<D: DatasetView + Sync>(ctx: &mut EvalCtx<'_, D>) -> u64 {
    ctx.rng_state = ctx.rng_state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = ctx.rng_state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// Mint a fresh blank node (`BNODE()`/`BNODE(strExpr)`'s cache-miss path).
/// Honors the context's deterministic [`EvalCtx::bnode_mint_prefix`], like every
/// other mint drawing on `bnode_counter`; with no prefix the label is exactly
/// `bnode{n}`, byte-identical to an unprefixed evaluation.
fn mint_bnode<D: DatasetView + Sync>(ctx: &mut EvalCtx<'_, D>) -> SolutionTerm<D::Id> {
    ctx.bnode_counter += 1;
    let label =
        crate::eval::minted_label(ctx.bnode_mint_prefix.as_deref(), "bnode", ctx.bnode_counter);
    intern(
        ctx,
        TermValue::Blank {
            label,
            scope: BlankScope::DEFAULT,
        },
    )
}

/// Resolve `reference` for the `IRI()`/`URI()` built-in (SPARQL 1.1 §17.4.2.6):
/// an already-absolute reference (has an RFC-3986 scheme) is returned unchanged;
/// a relative reference is resolved against `base` (the query's effective base
/// IRI). Returns `None` when `reference` is relative and there is no base (or
/// either string fails to parse as an IRI/IRI-reference) — a SPARQL expression
/// error, matching every other malformed-argument built-in in this module.
fn resolve_against_base(base: Option<&str>, reference: &str) -> Option<String> {
    if purrdf_iri::parse(reference).is_ok_and(|iri| iri.has_scheme()) {
        return Some(reference.to_owned());
    }
    let base = base?;
    let base_iri = purrdf_iri::parse(base).ok()?;
    Some(base_iri.resolve(reference).ok()?.as_str().to_owned())
}

/// Percent-encode every byte except unreserved characters (RFC 3986 §2.3:
/// `A-Za-z0-9 - _ . ~`). All other bytes become `%XX` in uppercase hex.
fn encode_for_uri(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for byte in s.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(char::from(*byte));
            }
            b => {
                out.push('%');
                out.push(
                    char::from_digit(u32::from(b >> 4), 16)
                        .unwrap()
                        .to_ascii_uppercase(),
                );
                out.push(
                    char::from_digit(u32::from(b & 0xf), 16)
                        .unwrap()
                        .to_ascii_uppercase(),
                );
            }
        }
    }
    out
}

/// Render a byte slice as lowercase hex.
fn hex_lower(bytes: &[u8]) -> String {
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
            s.push(char::from_digit(u32::from(*b >> 4), 16).unwrap());
            s.push(char::from_digit(u32::from(*b & 0xf), 16).unwrap());
            s
        })
}

/// Format a timezone offset in minutes as an `xsd:dayTimeDuration` string,
/// e.g. `+60` → `"PT1H"`, `0` → `"PT0S"`, `-330` → `"-PT5H30M"`.
fn format_daytime_duration(offset_minutes: i64) -> String {
    use core::fmt::Write as _;

    if offset_minutes == 0 {
        return "PT0S".to_owned();
    }
    let neg = offset_minutes < 0;
    let abs_min = offset_minutes.unsigned_abs();
    let hours = abs_min / 60;
    let mins = abs_min % 60;
    let mut s = if neg {
        "-PT".to_owned()
    } else {
        "PT".to_owned()
    };
    // Writing to a `String` is infallible, so the `write!` results are ignored.
    if hours > 0 {
        let _ = write!(s, "{hours}H");
    }
    if mins > 0 {
        let _ = write!(s, "{mins}M");
    }
    s
}

/// Format a timezone offset (minutes) as the SPARQL TZ() string:
/// `Some(0)` → `"Z"`, `Some(n)` → `"+HH:MM"` / `"-HH:MM"`, `None` → `""`.
fn format_tz_string(offset_minutes: Option<i64>) -> String {
    match offset_minutes {
        None => String::new(),
        Some(0) => "Z".to_owned(),
        Some(off) => {
            let sign = if off < 0 { '-' } else { '+' };
            let abs_min = off.unsigned_abs();
            format!("{sign}{:02}:{:02}", abs_min / 60, abs_min % 60)
        }
    }
}

/// A validated `ADJUST` second argument, pre-conversion to the `Option<i64>`
/// (seconds) shape `purrdf_xsd::temporal::adjust_*_to_timezone` expects. See
/// [`adjust_timezone_arg`].
enum AdjustTimezone {
    /// The empty-simple-literal `""` case: remove the timezone.
    Remove,
    /// A `dayTimeDuration`-valued shift/attach offset, in whole seconds.
    Offset(i64),
}

impl AdjustTimezone {
    /// Convert to the `Option<i64>` shape the `purrdf-xsd` adjust functions take.
    fn into_seconds(self) -> Option<i64> {
        match self {
            Self::Remove => None,
            Self::Offset(secs) => Some(secs),
        }
    }
}

/// Resolve `ADJUST`'s second argument to an [`AdjustTimezone`]. Returns `None`
/// for anything that is not a valid SPARQL `ADJUST` timezone argument — a type
/// error the caller folds into `Ok(None)`, same as every other
/// malformed-argument built-in in this module.
///
/// `fn:adjust-*-to-timezone` (XPath and XQuery Functions and Operators §9.6)
/// types `$timezone` as `xs:dayTimeDuration?`; an *empty sequence* there means
/// "remove the timezone". SPARQL has no empty-sequence value, so the SEP-0002
/// `ADJUST()` surface has no literal that types as "absent". Every known
/// SEP-0002 implementation (Apache Jena's `E_AdjustToTimezone` /
/// `XSDFuncOp.adjustToTimezone`) resolves this the same way: the empty simple
/// literal `""` stands in for the empty sequence, since it is the one
/// zero-information SPARQL value with no other meaning as a timezone. This
/// function mirrors that resolution.
///
/// A non-empty duration must be a **value-level** `dayTimeDuration` — `months()`
/// must be `0` (an `xsd:duration`/`xsd:yearMonthDuration` operand with a nonzero
/// month component is a type error) — and `seconds()` must be an exact whole
/// number of seconds (a fractional-second duration is a type error; the
/// whole-minute and ±14:00 domain checks happen inside the `purrdf-xsd` call).
fn adjust_timezone_arg(v: &XsdValue) -> Option<AdjustTimezone> {
    match v {
        XsdValue::String(s) if s.is_empty() => Some(AdjustTimezone::Remove),
        XsdValue::Duration(dur) => {
            if dur.months() != 0 {
                return None;
            }
            let seconds = dur.seconds();
            if !seconds.frac_part().is_zero() {
                return None; // non-integer seconds: not a whole-minute timezone
            }
            i64::try_from(seconds.whole_part())
                .ok()
                .map(AdjustTimezone::Offset)
        }
        _ => None,
    }
}

/// Mint a version-4 UUID from the PRNG state and return it as a
/// lowercase-hyphenated `8-4-4-4-12` string (without any `urn:uuid:` prefix).
fn make_uuid<D: DatasetView + Sync>(ctx: &mut EvalCtx<'_, D>) -> (String, [u8; 16]) {
    let hi = next_u64(ctx);
    let lo = next_u64(ctx);
    let mut bytes = [0u8; 16];
    bytes[..8].copy_from_slice(&hi.to_be_bytes());
    bytes[8..].copy_from_slice(&lo.to_be_bytes());
    // Set version 4 (bits 76–79 of octet 6): top nibble = 4.
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    // Set variant bits (RFC 4122 §4.1.1): top 2 bits of octet 8 = 10.
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex: Vec<String> = bytes.iter().map(|b| format!("{b:02x}")).collect();
    let uuid = format!(
        "{}-{}-{}-{}-{}",
        hex[0..4].join(""),
        hex[4..6].join(""),
        hex[6..8].join(""),
        hex[8..10].join(""),
        hex[10..16].join(""),
    );
    (uuid, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::eval;
    use purrdf_core::{RdfDataset, RdfDatasetBuilder};
    use purrdf_sparql_algebra::{Literal, NamedNode};

    fn empty_ds() -> Arc<RdfDataset> {
        RdfDatasetBuilder::new().freeze().expect("freeze")
    }

    fn lit(value: &str) -> Expression {
        Expression::Literal(Literal::new_simple(value))
    }
    fn typed_lit(value: &str, dt: &str) -> Expression {
        Expression::Literal(Literal::new_typed(value, NamedNode::new_unchecked(dt)))
    }
    fn iri(iri: &str) -> Expression {
        Expression::NamedNode(NamedNode::new_unchecked(iri))
    }

    /// Evaluate a constant expression (empty solution) and return the EBV.
    fn ebv(ds: &RdfDataset, expr: &Expression) -> Option<bool> {
        let mut ctx = EvalCtx::new(ds);
        let schema = VarSchema::new();
        eval_ebv(expr, &[], &schema, &mut ctx).expect("eval")
    }

    /// Evaluate a constant expression to a string lexical form, if it is a literal.
    fn lex(ds: &RdfDataset, expr: &Expression) -> Option<String> {
        let mut ctx = EvalCtx::new(ds);
        let schema = VarSchema::new();
        let term = eval_expr(expr, &[], &schema, &mut ctx).expect("eval")?;
        match value_of(&ctx, term) {
            TermValue::Literal { lexical_form, .. } => Some(lexical_form),
            TermValue::Iri(s) => Some(s),
            _ => None,
        }
    }

    const XINT: &str = "http://www.w3.org/2001/XMLSchema#integer";

    #[test]
    fn numeric_comparison_uses_value_space() {
        let ds = empty_ds();
        // "2"^^xsd:integer < "10"^^xsd:integer (value, not lexicographic).
        let lt = Expression::Less(
            Box::new(typed_lit("2", XINT)),
            Box::new(typed_lit("10", XINT)),
        );
        assert_eq!(ebv(&ds, &lt), Some(true));
    }

    #[test]
    fn kleene_or_with_error_and_true_is_true() {
        let ds = empty_ds();
        // (error || true) == true, even though the left operand errors.
        let err = Expression::Less(Box::new(iri("http://ex/a")), Box::new(iri("http://ex/b")));
        let expr = Expression::Or(
            Box::new(err),
            Box::new(typed_lit(
                "true",
                "http://www.w3.org/2001/XMLSchema#boolean",
            )),
        );
        assert_eq!(ebv(&ds, &expr), Some(true));
    }

    #[test]
    fn kleene_and_with_error_and_false_is_false() {
        let ds = empty_ds();
        let err = Expression::Less(Box::new(iri("http://ex/a")), Box::new(iri("http://ex/b")));
        let expr = Expression::And(
            Box::new(err),
            Box::new(typed_lit(
                "false",
                "http://www.w3.org/2001/XMLSchema#boolean",
            )),
        );
        assert_eq!(ebv(&ds, &expr), Some(false));
    }

    #[test]
    fn sameterm_distinguishes_lexical_forms() {
        let ds = empty_ds();
        // "1"^^xsd:integer = "01"^^xsd:integer (value equal) but NOT sameTerm.
        let eq = Expression::Equal(
            Box::new(typed_lit("1", XINT)),
            Box::new(typed_lit("01", XINT)),
        );
        let same = Expression::SameTerm(
            Box::new(typed_lit("1", XINT)),
            Box::new(typed_lit("01", XINT)),
        );
        assert_eq!(ebv(&ds, &eq), Some(true));
        assert_eq!(ebv(&ds, &same), Some(false));
    }

    #[test]
    fn equal_treats_cross_type_nan_as_same_value() {
        // SPARQL 1.2 §17.4.2.2 `sameValue` (which defines `=`), step 5, verbatim:
        // "NaN"^^xsd:double and "NaN"^^xsd:float are considered to represent the
        // SAME value, even though they are not the same RDF term (different
        // datatype IRIs) and `value_cmp`'s ordinary numeric-tower promotion
        // treats NaN as unordered (`f64`/`f32` `partial_cmp`, correctly, for
        // `<`/`>`/`ORDER BY`). Regression guard for the gap `sparql_value_eq`
        // closes: this used to evaluate to a type error (unbound), not `true`.
        const XDOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";
        const XFLOAT: &str = "http://www.w3.org/2001/XMLSchema#float";
        let ds = empty_ds();
        let eq = Expression::Equal(
            Box::new(typed_lit("NaN", XDOUBLE)),
            Box::new(typed_lit("NaN", XFLOAT)),
        );
        assert_eq!(ebv(&ds, &eq), Some(true));
        // Same-type NaN pairs already resolve via the identical-RDF-term
        // short-circuit (NaN's canonical lexical form is always "NaN"); prove
        // that path stays `true` too, not just the cross-type one this test
        // targets.
        let eq_same_type = Expression::Equal(
            Box::new(typed_lit("NaN", XDOUBLE)),
            Box::new(typed_lit("NaN", XDOUBLE)),
        );
        assert_eq!(ebv(&ds, &eq_same_type), Some(true));
        // A NaN is still UNORDERED under `<`: the carve-out is `sameValue`'s
        // alone and must not leak into the ordering operators.
        let lt = Expression::Less(
            Box::new(typed_lit("NaN", XDOUBLE)),
            Box::new(typed_lit("NaN", XFLOAT)),
        );
        assert_eq!(ebv(&ds, &lt), None);
    }

    #[test]
    fn str_and_concat_and_strlen() {
        let ds = empty_ds();
        let concat = Expression::FunctionCall(Function::Concat, vec![lit("foo"), lit("bar")]);
        assert_eq!(lex(&ds, &concat), Some("foobar".to_owned()));
        let strlen = Expression::FunctionCall(Function::StrLen, vec![lit("héllo")]);
        assert_eq!(lex(&ds, &strlen), Some("5".to_owned()));
        let str_of_iri = Expression::FunctionCall(Function::Str, vec![iri("http://ex/x")]);
        assert_eq!(lex(&ds, &str_of_iri), Some("http://ex/x".to_owned()));
    }

    #[test]
    fn contains_and_regex() {
        let ds = empty_ds();
        let contains =
            Expression::FunctionCall(Function::Contains, vec![lit("hello world"), lit("o w")]);
        assert_eq!(ebv(&ds, &contains), Some(true));
        let re = Expression::FunctionCall(
            Function::Regex,
            vec![lit("Hello"), lit("^h"), lit("i")], // case-insensitive
        );
        assert_eq!(ebv(&ds, &re), Some(true));
    }

    #[test]
    fn string_predicates_do_not_mint_nested_str_terms() {
        let ds = empty_ds();
        let schema = VarSchema::new();
        let mut ctx = EvalCtx::new(&ds);
        let expr = Expression::FunctionCall(
            Function::StrStarts,
            vec![
                Expression::FunctionCall(Function::Str, vec![iri("http://ex/alice")]),
                lit("http://ex/"),
            ],
        );

        assert_eq!(
            eval_ebv(&expr, &[], &schema, &mut ctx).expect("strstarts"),
            Some(true)
        );
        assert_eq!(
            ctx.scratch.computed_count(),
            1,
            "only the boolean result is minted"
        );
    }

    #[test]
    fn regex_cache_reuses_compiled_pattern_and_failures() {
        let ds = empty_ds();
        let schema = VarSchema::new();
        let mut ctx = EvalCtx::new(&ds);
        let re = Expression::FunctionCall(Function::Regex, vec![lit("Hello"), lit("^h"), lit("i")]);
        let bad =
            Expression::FunctionCall(Function::Regex, vec![lit("Hello"), lit("^h"), lit("z")]);
        // Total `(pattern, flags)` entries across the pattern-keyed two-level map.
        let entries = |ctx: &EvalCtx<'_, Arc<RdfDataset>>| {
            ctx.regex_cache
                .values()
                .map(crate::DetHashMap::len)
                .sum::<usize>()
        };

        assert_eq!(
            eval_ebv(&re, &[], &schema, &mut ctx).expect("first regex"),
            Some(true)
        );
        assert_eq!(
            eval_ebv(&re, &[], &schema, &mut ctx).expect("second regex"),
            Some(true)
        );
        assert_eq!(entries(&ctx), 1);

        assert_eq!(
            eval_ebv(&bad, &[], &schema, &mut ctx).expect("invalid regex"),
            None
        );
        assert_eq!(
            eval_ebv(&bad, &[], &schema, &mut ctx).expect("invalid regex cached"),
            None
        );
        assert_eq!(entries(&ctx), 2);
    }

    #[test]
    fn substr_one_based() {
        let ds = empty_ds();
        // SUBSTR("abcdef", 2, 3) == "bcd".
        let s = Expression::FunctionCall(
            Function::SubStr,
            vec![lit("abcdef"), typed_lit("2", XINT), typed_lit("3", XINT)],
        );
        assert_eq!(lex(&ds, &s), Some("bcd".to_owned()));
    }

    #[test]
    fn type_tests() {
        let ds = empty_ds();
        assert_eq!(
            ebv(
                &ds,
                &Expression::FunctionCall(Function::IsIri, vec![iri("http://ex/x")])
            ),
            Some(true)
        );
        assert_eq!(
            ebv(
                &ds,
                &Expression::FunctionCall(Function::IsLiteral, vec![lit("x")])
            ),
            Some(true)
        );
        assert_eq!(
            ebv(
                &ds,
                &Expression::FunctionCall(Function::IsNumeric, vec![typed_lit("3", XINT)])
            ),
            Some(true)
        );
        assert_eq!(
            ebv(
                &ds,
                &Expression::FunctionCall(Function::IsNumeric, vec![lit("x")])
            ),
            Some(false)
        );
    }

    #[test]
    fn coalesce_skips_errors() {
        let ds = empty_ds();
        // COALESCE(error, "fallback") → "fallback".
        let err = Expression::FunctionCall(Function::Str, vec![]); // STR() with no arg → error
        let expr = Expression::Coalesce(vec![err, lit("fallback")]);
        assert_eq!(lex(&ds, &expr), Some("fallback".to_owned()));
    }

    const XDEC: &str = "http://www.w3.org/2001/XMLSchema#decimal";

    // ---- arithmetic: positive tests ----------------------------------------

    #[test]
    fn arithmetic_add_integers() {
        let ds = empty_ds();
        // 1 + 2 = 3
        let expr = Expression::Add(
            Box::new(typed_lit("1", XINT)),
            Box::new(typed_lit("2", XINT)),
        );
        assert_eq!(lex(&ds, &expr), Some("3".to_owned()));
    }

    #[test]
    fn arithmetic_subtract_integers() {
        let ds = empty_ds();
        // 7 - 3 = 4
        let expr = Expression::Subtract(
            Box::new(typed_lit("7", XINT)),
            Box::new(typed_lit("3", XINT)),
        );
        assert_eq!(lex(&ds, &expr), Some("4".to_owned()));
    }

    #[test]
    fn arithmetic_multiply_integers() {
        let ds = empty_ds();
        // 3 * 4 = 12
        let expr = Expression::Multiply(
            Box::new(typed_lit("3", XINT)),
            Box::new(typed_lit("4", XINT)),
        );
        assert_eq!(lex(&ds, &expr), Some("12".to_owned()));
    }

    #[test]
    fn arithmetic_divide_integer_returns_decimal() {
        let ds = empty_ds();
        // 1 / 2 = 0.5 (decimal, per XPath op:numeric-divide)
        let expr = Expression::Divide(
            Box::new(typed_lit("1", XINT)),
            Box::new(typed_lit("2", XINT)),
        );
        // The result is a decimal; lexical "0.5" at scale 18 → canonical starts "0.5"
        let result = lex(&ds, &expr).expect("should produce a value");
        // Parse it back to verify the value; the canonical form has 18 fractional
        // digits so we just check that it starts with "0.5".
        assert!(
            result.starts_with("0.5"),
            "1/2 should be 0.5…, got {result}"
        );
    }

    #[test]
    fn arithmetic_divide_10_4() {
        let ds = empty_ds();
        // 10 / 4 = 2.5
        let expr = Expression::Divide(
            Box::new(typed_lit("10", XINT)),
            Box::new(typed_lit("4", XINT)),
        );
        let result = lex(&ds, &expr).expect("should produce a value");
        assert!(
            result.starts_with("2.5"),
            "10/4 should be 2.5…, got {result}"
        );
    }

    // ---- arithmetic: type error and divide-by-zero → Ok(None) --------------

    #[test]
    fn arithmetic_type_error_is_ok_none() {
        let ds = empty_ds();
        // "a" + 1 → type error → Ok(None) (a FILTER drops the row; no hard Err).
        let expr = Expression::Add(Box::new(lit("a")), Box::new(typed_lit("1", XINT)));
        let mut ctx = EvalCtx::new(&ds);
        let schema = VarSchema::new();
        let result = eval_expr(&expr, &[], &schema, &mut ctx).expect("no hard error");
        assert!(
            result.is_none(),
            "type error must be Ok(None), not Ok(Some)"
        );
    }

    #[test]
    fn arithmetic_divide_by_zero_is_ok_none() {
        let ds = empty_ds();
        // integer/0 → DivisionByZero → Ok(None)
        let expr = Expression::Divide(
            Box::new(typed_lit("5", XINT)),
            Box::new(typed_lit("0", XINT)),
        );
        let mut ctx = EvalCtx::new(&ds);
        let schema = VarSchema::new();
        let result = eval_expr(&expr, &[], &schema, &mut ctx).expect("no hard error");
        assert!(result.is_none(), "divide-by-zero must be Ok(None)");
    }

    // ---- unary operators ---------------------------------------------------

    #[test]
    fn arithmetic_unary_minus() {
        let ds = empty_ds();
        // -5 = -5
        let expr = Expression::UnaryMinus(Box::new(typed_lit("5", XINT)));
        assert_eq!(lex(&ds, &expr), Some("-5".to_owned()));
    }

    // ---- ABS / CEIL / FLOOR / ROUND ----------------------------------------

    #[test]
    fn function_abs() {
        let ds = empty_ds();
        // ABS(-3) = 3
        let expr = Expression::FunctionCall(Function::Abs, vec![typed_lit("-3", XINT)]);
        assert_eq!(lex(&ds, &expr), Some("3".to_owned()));
    }

    #[test]
    fn function_ceil() {
        let ds = empty_ds();
        // CEIL(2.1) = 3 (xsd:decimal; XSD 1.1 whole-decimal lexical has no point)
        let expr = Expression::FunctionCall(Function::Ceil, vec![typed_lit("2.1", XDEC)]);
        assert_eq!(lex(&ds, &expr), Some("3".to_owned()));
    }

    #[test]
    fn function_floor() {
        let ds = empty_ds();
        // FLOOR(2.9) = 2 (xsd:decimal; XSD 1.1 whole-decimal lexical has no point)
        let expr = Expression::FunctionCall(Function::Floor, vec![typed_lit("2.9", XDEC)]);
        assert_eq!(lex(&ds, &expr), Some("2".to_owned()));
    }

    #[test]
    fn function_round() {
        let ds = empty_ds();
        // ROUND(2.5) = 3 (round-half-toward-+infinity per XPath fn:round; XSD 1.1
        // whole-decimal lexical has no point)
        let expr = Expression::FunctionCall(Function::Round, vec![typed_lit("2.5", XDEC)]);
        assert_eq!(lex(&ds, &expr), Some("3".to_owned()));
    }

    // ---- BIND integration: arithmetic column over a real BGP ---------------

    #[test]
    fn bind_arithmetic_computed_column() {
        let ds = typed_graph();
        let mut ctx = EvalCtx::new(&ds);
        // { ?s :age ?n . BIND(?n + 1 AS ?plus1) }
        // :a has age 30, so plus1 should be 31.
        // :b has age 17, so plus1 should be 18.
        let inner = bgp1("s", "http://ex/age", "n");
        let expr = Expression::Add(
            Box::new(Expression::Variable(Variable::new("n"))),
            Box::new(typed_lit("1", XINT)),
        );
        let seq = eval(
            &GraphPattern::Extend {
                inner: Box::new(inner),
                variable: Variable::new("plus1"),
                expression: expr,
            },
            &mut ctx,
        )
        .expect("bind arithmetic");
        let plus1_col = seq.schema.index_of(&Variable::new("plus1")).unwrap();
        let mut results: Vec<String> = seq
            .rows
            .iter()
            .filter_map(|r| r[plus1_col])
            .map(|t| match ctx.scratch.value_of(&ds, t) {
                TermValue::Literal { lexical_form, .. } => lexical_form,
                other => format!("{other:?}"),
            })
            .collect();
        results.sort();
        assert_eq!(results, vec!["18".to_owned(), "31".to_owned()]);
    }

    // --- integration: FILTER / BIND / EXISTS over a real BGP ---------------

    fn typed_graph() -> Arc<RdfDataset> {
        // :a :age 30 ; :name "Ann" .
        // :b :age 17 .
        // :a :member :club .   (a is a member; b is not)
        use purrdf_core::RdfLiteral;
        let mut b = RdfDatasetBuilder::new();
        let age = b.intern_iri("http://ex/age");
        let name = b.intern_iri("http://ex/name");
        let member = b.intern_iri("http://ex/member");
        let a = b.intern_iri("http://ex/a");
        let bb = b.intern_iri("http://ex/b");
        let club = b.intern_iri("http://ex/club");
        let i30 = b.intern_literal(RdfLiteral {
            lexical_form: "30".to_owned(),
            datatype: Some(XINT.to_owned()),
            language: None,
            direction: None,
        });
        let i17 = b.intern_literal(RdfLiteral {
            lexical_form: "17".to_owned(),
            datatype: Some(XINT.to_owned()),
            language: None,
            direction: None,
        });
        let ann = b.intern_literal(RdfLiteral::simple("Ann"));
        b.push_quad(a, age, i30, None);
        b.push_quad(a, name, ann, None);
        b.push_quad(bb, age, i17, None);
        b.push_quad(a, member, club, None);
        b.freeze().expect("freeze")
    }

    /// A second FILTER fixture, alongside [`typed_graph`], for the SEP-0002
    /// temporal operators.
    ///
    /// Unbound and `false` are indistinguishable in a `BIND` (both leave the
    /// target unbound) and produce the *same* observable effect in a
    /// `FILTER` (the row disappears). A `FILTER` test that only checks which
    /// rows survive therefore asserts a discrimination it cannot make; every
    /// `FILTER` test built over this fixture is a `FILTER(p)` / `FILTER(!p)`
    /// **pair**, because `!false = true` but `!error = error` — only the
    /// pair separates them.
    ///
    /// `:e1`/`:e2` are ordinary same-timezone instant pairs (9 days and 2
    /// days apart). `:e3` mixes a timezone-bearing `:start` with a
    /// timezone-less `:end` — its instant difference is spec-mandated
    /// indeterminate, not merely "false", so it must be absent from a
    /// `FILTER(p)`/`FILTER(!p)` pair over it in BOTH directions. `:p1`/`:p2`
    /// each carry one general `xsd:duration`, value-equal to `"P30D"` only
    /// for `:p2` (`P1M` and `P30D` are unequal, not incomparable — duration
    /// `=` is total).
    fn temporal_graph() -> Arc<RdfDataset> {
        use purrdf_core::RdfLiteral;
        const XDATETIME: &str = "http://www.w3.org/2001/XMLSchema#dateTime";
        const XDURATION: &str = "http://www.w3.org/2001/XMLSchema#duration";
        let mut b = RdfDatasetBuilder::new();
        let start = b.intern_iri("http://ex/start");
        let end = b.intern_iri("http://ex/end");
        let period = b.intern_iri("http://ex/period");
        let e1 = b.intern_iri("http://ex/e1");
        let e2 = b.intern_iri("http://ex/e2");
        let e3 = b.intern_iri("http://ex/e3");
        let p1 = b.intern_iri("http://ex/p1");
        let p2 = b.intern_iri("http://ex/p2");
        let e1_start = b.intern_literal(RdfLiteral {
            lexical_form: "2024-03-01T00:00:00Z".to_owned(),
            datatype: Some(XDATETIME.to_owned()),
            language: None,
            direction: None,
        });
        let e1_end = b.intern_literal(RdfLiteral {
            lexical_form: "2024-03-10T00:00:00Z".to_owned(),
            datatype: Some(XDATETIME.to_owned()),
            language: None,
            direction: None,
        });
        let e2_start = b.intern_literal(RdfLiteral {
            lexical_form: "2024-03-01T00:00:00Z".to_owned(),
            datatype: Some(XDATETIME.to_owned()),
            language: None,
            direction: None,
        });
        let e2_end = b.intern_literal(RdfLiteral {
            lexical_form: "2024-03-03T00:00:00Z".to_owned(),
            datatype: Some(XDATETIME.to_owned()),
            language: None,
            direction: None,
        });
        let e3_start = b.intern_literal(RdfLiteral {
            lexical_form: "2024-03-01T00:00:00Z".to_owned(),
            datatype: Some(XDATETIME.to_owned()),
            language: None,
            direction: None,
        });
        // No timezone — the row whose instant difference must be indeterminate.
        let e3_end = b.intern_literal(RdfLiteral {
            lexical_form: "2024-03-10T00:00:00".to_owned(),
            datatype: Some(XDATETIME.to_owned()),
            language: None,
            direction: None,
        });
        let p1_period = b.intern_literal(RdfLiteral {
            lexical_form: "P1M".to_owned(),
            datatype: Some(XDURATION.to_owned()),
            language: None,
            direction: None,
        });
        let p2_period = b.intern_literal(RdfLiteral {
            lexical_form: "P30D".to_owned(),
            datatype: Some(XDURATION.to_owned()),
            language: None,
            direction: None,
        });
        b.push_quad(e1, start, e1_start, None);
        b.push_quad(e1, end, e1_end, None);
        b.push_quad(e2, start, e2_start, None);
        b.push_quad(e2, end, e2_end, None);
        b.push_quad(e3, start, e3_start, None);
        b.push_quad(e3, end, e3_end, None);
        b.push_quad(p1, period, p1_period, None);
        b.push_quad(p2, period, p2_period, None);
        b.freeze().expect("freeze")
    }

    fn bgp1(s: &str, p: &str, o: &str) -> GraphPattern {
        use purrdf_sparql_algebra::{NamedNodePattern, TermPattern, TriplePattern};
        GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: TermPattern::Variable(Variable::new(s)),
                predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked(p)),
                object: TermPattern::Variable(Variable::new(o)),
            }],
        }
    }

    /// A two-pattern BGP `{ ?s p1 ?o1 . ?s p2 ?o2 }`, joined on the shared
    /// subject variable — [`temporal_graph`]'s start/end pairs need both
    /// bound in one row so a FILTER can subtract them.
    fn bgp2(s: &str, p1: &str, o1: &str, p2: &str, o2: &str) -> GraphPattern {
        use purrdf_sparql_algebra::{NamedNodePattern, TermPattern, TriplePattern};
        GraphPattern::Bgp {
            patterns: vec![
                TriplePattern {
                    subject: TermPattern::Variable(Variable::new(s)),
                    predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked(p1)),
                    object: TermPattern::Variable(Variable::new(o1)),
                },
                TriplePattern {
                    subject: TermPattern::Variable(Variable::new(s)),
                    predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked(p2)),
                    object: TermPattern::Variable(Variable::new(o2)),
                },
            ],
        }
    }

    fn subjects(ds: &RdfDataset, seq: &SolutionSeq, var: &str) -> Vec<String> {
        let scratch = crate::scratch::ScratchInterner::new();
        let col = seq.schema.index_of(&Variable::new(var)).unwrap();
        let mut out: Vec<String> = seq
            .rows
            .iter()
            .filter_map(|r| r[col])
            .map(|t| match scratch.value_of(ds, t) {
                TermValue::Iri(s) => s,
                other => format!("{other:?}"),
            })
            .collect();
        out.sort();
        out
    }

    #[test]
    fn filter_numeric_over_bgp() {
        let ds = typed_graph();
        let mut ctx = EvalCtx::new(&ds);
        // { ?s :age ?n FILTER(?n >= 18) } → only :a.
        let inner = bgp1("s", "http://ex/age", "n");
        let cond = Expression::GreaterOrEqual(
            Box::new(Expression::Variable(Variable::new("n"))),
            Box::new(typed_lit("18", XINT)),
        );
        let seq = eval(
            &GraphPattern::Filter {
                expr: cond,
                inner: Box::new(inner),
            },
            &mut ctx,
        )
        .expect("filter");
        assert_eq!(subjects(&ds, &seq, "s"), vec!["http://ex/a".to_owned()]);
    }

    #[test]
    fn bind_adds_a_computed_column() {
        let ds = typed_graph();
        let mut ctx = EvalCtx::new(&ds);
        // { ?s :name ?nm . BIND(UCASE(?nm) AS ?u) }
        let inner = bgp1("s", "http://ex/name", "nm");
        let expr = Expression::FunctionCall(
            Function::UCase,
            vec![Expression::Variable(Variable::new("nm"))],
        );
        let seq = eval(
            &GraphPattern::Extend {
                inner: Box::new(inner),
                variable: Variable::new("u"),
                expression: expr,
            },
            &mut ctx,
        )
        .expect("bind");
        let u = seq.schema.index_of(&Variable::new("u")).unwrap();
        // UCASE("Ann") = "ANN" is a *computed* term, so it must be resolved through
        // the SAME scratch interner that the evaluation used (a fresh one cannot
        // resolve scratch ids — only dataset-resident `Existing` terms).
        let val = ctx.scratch.value_of(&ds, seq.rows[0][u].unwrap());
        assert!(matches!(val, TermValue::Literal { lexical_form, .. } if lexical_form == "ANN"));
    }

    #[test]
    fn filter_not_exists_over_bgp() {
        let ds = typed_graph();
        let mut ctx = EvalCtx::new(&ds);
        // { ?s :age ?n FILTER NOT EXISTS { ?s :member ?c } } → people with an age
        // who are NOT members → only :b (a is a member).
        let inner = bgp1("s", "http://ex/age", "n");
        let exists_pat = bgp1("s", "http://ex/member", "c");
        let not_exists = Expression::Not(Box::new(Expression::Exists(Box::new(exists_pat))));
        let seq = eval(
            &GraphPattern::Filter {
                expr: not_exists,
                inner: Box::new(inner),
            },
            &mut ctx,
        )
        .expect("not exists");
        assert_eq!(subjects(&ds, &seq, "s"), vec!["http://ex/b".to_owned()]);
    }

    // ---- FILTER coverage: SEP-0002 temporal operators over a real BGP -----
    //
    // Unbound and `false` are indistinguishable in a `BIND` and produce the
    // same observable effect in a `FILTER` (the row disappears), so a
    // `FILTER` test that only checks which rows survive asserts a
    // discrimination it cannot make. Every test below is therefore built as
    // a `FILTER(p)` / `FILTER(!p)` pair (or pins the pair explicitly),
    // because `!false = true` but `!error = error` — only the pair
    // separates a comparison that came out `false` from one that errored.

    #[test]
    fn filter_instant_difference_over_bgp() {
        let ds = temporal_graph();
        let mut ctx = EvalCtx::new(&ds);
        // { ?s :start ?start ; :end ?end FILTER(?end - ?start > "P7D"^^xsd:dayTimeDuration) }
        // :e1 is 9 days (> 7D, kept); :e2 is 2 days (not kept); :e3's
        // difference is indeterminate (mixed timezone, not kept).
        let inner = bgp2("s", "http://ex/start", "start", "http://ex/end", "end");
        let cond = Expression::Greater(
            Box::new(Expression::Subtract(
                Box::new(Expression::Variable(Variable::new("end"))),
                Box::new(Expression::Variable(Variable::new("start"))),
            )),
            Box::new(typed_lit("P7D", XSD_DAYTIME_DURATION)),
        );
        let seq = eval(
            &GraphPattern::Filter {
                expr: cond,
                inner: Box::new(inner),
            },
            &mut ctx,
        )
        .expect("filter");
        assert_eq!(subjects(&ds, &seq, "s"), vec!["http://ex/e1".to_owned()]);
    }

    #[test]
    fn filter_negation_separates_false_from_error() {
        // The discrimination `filter_instant_difference_over_bgp` cannot make
        // on its own: `:e3`'s instant difference is a type error (mixed
        // timezone), not a `false` comparison, so it must be absent from
        // BOTH `FILTER(p)` and `FILTER(!p)` over the same predicate — a
        // `false` comparison would instead surface `:e3` under the negated
        // form. Both evaluations live in this one test so the discrimination
        // is asserted in a single place.
        let ds = temporal_graph();
        let cond = Expression::Greater(
            Box::new(Expression::Subtract(
                Box::new(Expression::Variable(Variable::new("end"))),
                Box::new(Expression::Variable(Variable::new("start"))),
            )),
            Box::new(typed_lit("P7D", XSD_DAYTIME_DURATION)),
        );

        let mut ctx_pos = EvalCtx::new(&ds);
        let inner_pos = bgp2("s", "http://ex/start", "start", "http://ex/end", "end");
        let seq_pos = eval(
            &GraphPattern::Filter {
                expr: cond.clone(),
                inner: Box::new(inner_pos),
            },
            &mut ctx_pos,
        )
        .expect("filter positive");
        let pos = subjects(&ds, &seq_pos, "s");

        let mut ctx_neg = EvalCtx::new(&ds);
        let inner_neg = bgp2("s", "http://ex/start", "start", "http://ex/end", "end");
        let seq_neg = eval(
            &GraphPattern::Filter {
                expr: Expression::Not(Box::new(cond)),
                inner: Box::new(inner_neg),
            },
            &mut ctx_neg,
        )
        .expect("filter negated");
        let neg = subjects(&ds, &seq_neg, "s");

        assert_eq!(pos, vec!["http://ex/e1".to_owned()]);
        assert_eq!(neg, vec!["http://ex/e2".to_owned()]);
        assert!(
            !pos.contains(&"http://ex/e2".to_owned()) && !neg.contains(&"http://ex/e1".to_owned()),
            "the positive and negated filters must be complementary over the errorless rows"
        );
        assert!(
            !pos.contains(&"http://ex/e3".to_owned()) && !neg.contains(&"http://ex/e3".to_owned()),
            ":e3 must be absent from BOTH results — its presence in neither is the proof \
             its timezone mix errored rather than compared false"
        );
    }

    #[test]
    fn filter_instant_plus_duration_over_bgp() {
        let ds = temporal_graph();
        let mut ctx = EvalCtx::new(&ds);
        // { ?s :start ?start ; :end ?end FILTER(?start + "P5D"^^xsd:dayTimeDuration > ?end) }
        // pins that a COMPUTED dateTime (the sum) flows back into `compare`
        // rather than only a literal or a bare variable.
        // :e1: 2024-03-01 + 5D = 2024-03-06, not > 2024-03-10 end -> excluded.
        // :e2: 2024-03-01 + 5D = 2024-03-06, > 2024-03-03 end -> included.
        // :e3: same shifted start, compared against a timezone-less end that
        // is unambiguously later -> determinate `false`, excluded.
        let inner = bgp2("s", "http://ex/start", "start", "http://ex/end", "end");
        let cond = Expression::Greater(
            Box::new(Expression::Add(
                Box::new(Expression::Variable(Variable::new("start"))),
                Box::new(typed_lit("P5D", XSD_DAYTIME_DURATION)),
            )),
            Box::new(Expression::Variable(Variable::new("end"))),
        );
        let seq = eval(
            &GraphPattern::Filter {
                expr: cond,
                inner: Box::new(inner),
            },
            &mut ctx,
        )
        .expect("filter");
        assert_eq!(subjects(&ds, &seq, "s"), vec!["http://ex/e2".to_owned()]);
    }

    #[test]
    fn filter_over_a_bound_duration_round_trips_through_interning() {
        let ds = temporal_graph();
        let mut ctx = EvalCtx::new(&ds);
        // { ?s :start ?start ; :end ?end . BIND(?end - ?start AS ?len)
        //   FILTER(?len / "P1D"^^xsd:dayTimeDuration >= 7) }
        // The minted `?len` duration is interned as a term, read back,
        // re-parsed, and fed to a second operator (`/`) — a lexical that
        // emits correctly but does not re-parse fails ONLY here.
        // :e1: 9D / 1D = 9 >= 7 -> included. :e2: 2D / 1D = 2 -> excluded.
        // :e3: ?len is unbound (the subtraction errored), so the FILTER
        // predicate is unbound too -> excluded.
        let inner = bgp2("s", "http://ex/start", "start", "http://ex/end", "end");
        let bound = GraphPattern::Extend {
            inner: Box::new(inner),
            variable: Variable::new("len"),
            expression: Expression::Subtract(
                Box::new(Expression::Variable(Variable::new("end"))),
                Box::new(Expression::Variable(Variable::new("start"))),
            ),
        };
        let cond = Expression::GreaterOrEqual(
            Box::new(Expression::Divide(
                Box::new(Expression::Variable(Variable::new("len"))),
                Box::new(typed_lit("P1D", XSD_DAYTIME_DURATION)),
            )),
            Box::new(typed_lit("7", XINT)),
        );
        let seq = eval(
            &GraphPattern::Filter {
                expr: cond,
                inner: Box::new(bound),
            },
            &mut ctx,
        )
        .expect("filter");
        assert_eq!(subjects(&ds, &seq, "s"), vec!["http://ex/e1".to_owned()]);
    }

    #[test]
    fn filter_duration_equality_over_bgp() {
        let ds = temporal_graph();
        let mut ctx = EvalCtx::new(&ds);
        // { ?s :period ?period FILTER(?period = "P30D"^^xsd:duration) }
        // :p1's period is "P1M" — value-unequal to "P30D" (`P1M = P30D` is
        // `false`, not an error: duration `=` is total). :p2's is "P30D"
        // itself.
        let inner = bgp1("s", "http://ex/period", "period");
        let cond = Expression::Equal(
            Box::new(Expression::Variable(Variable::new("period"))),
            Box::new(typed_lit("P30D", XSD_DURATION)),
        );
        let seq = eval(
            &GraphPattern::Filter {
                expr: cond,
                inner: Box::new(inner),
            },
            &mut ctx,
        )
        .expect("filter");
        assert_eq!(subjects(&ds, &seq, "s"), vec!["http://ex/p2".to_owned()]);
    }

    #[test]
    fn filter_duration_equality_negated_over_bgp() {
        // Before duration `=` was made total, this comparison errored for
        // BOTH operands rather than answering `false` for the unequal pair —
        // a type error here would exclude :p1 from both filters, so this
        // negated form would have returned [] instead of [:p1].
        let ds = temporal_graph();
        let mut ctx = EvalCtx::new(&ds);
        // { ?s :period ?period FILTER(!(?period = "P30D"^^xsd:duration)) }
        let inner = bgp1("s", "http://ex/period", "period");
        let cond = Expression::Not(Box::new(Expression::Equal(
            Box::new(Expression::Variable(Variable::new("period"))),
            Box::new(typed_lit("P30D", XSD_DURATION)),
        )));
        let seq = eval(
            &GraphPattern::Filter {
                expr: cond,
                inner: Box::new(inner),
            },
            &mut ctx,
        )
        .expect("filter");
        assert_eq!(subjects(&ds, &seq, "s"), vec!["http://ex/p1".to_owned()]);
    }

    // ---- Gap 4: ENCODE_FOR_URI ---------------------------------------------

    #[test]
    fn encode_for_uri_basic() {
        let ds = empty_ds();
        let expr = Expression::FunctionCall(Function::EncodeForUri, vec![lit("a b/c")]);
        assert_eq!(lex(&ds, &expr), Some("a%20b%2Fc".to_owned()));
    }

    // ---- Gap 4: hash functions --------------------------------------------

    const XSD_DATETIME: &str = "http://www.w3.org/2001/XMLSchema#dateTime";

    #[test]
    fn md5_abc() {
        let ds = empty_ds();
        let expr = Expression::FunctionCall(Function::Md5, vec![lit("abc")]);
        assert_eq!(
            lex(&ds, &expr),
            Some("900150983cd24fb0d6963f7d28e17f72".to_owned())
        );
    }

    #[test]
    fn sha1_abc() {
        let ds = empty_ds();
        let expr = Expression::FunctionCall(Function::Sha1, vec![lit("abc")]);
        assert_eq!(
            lex(&ds, &expr),
            Some("a9993e364706816aba3e25717850c26c9cd0d89d".to_owned())
        );
    }

    #[test]
    fn sha256_abc() {
        let ds = empty_ds();
        let expr = Expression::FunctionCall(Function::Sha256, vec![lit("abc")]);
        assert_eq!(
            lex(&ds, &expr),
            Some("ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad".to_owned())
        );
    }

    // ---- Gap 4: date/time component extraction ----------------------------

    #[test]
    fn year_month_day_over_datetime() {
        let ds = empty_ds();
        let dt = typed_lit("2024-03-15T10:30:00Z", XSD_DATETIME);
        let year = Expression::FunctionCall(Function::Year, vec![dt.clone()]);
        let month = Expression::FunctionCall(Function::Month, vec![dt.clone()]);
        let day = Expression::FunctionCall(Function::Day, vec![dt]);
        assert_eq!(lex(&ds, &year), Some("2024".to_owned()));
        assert_eq!(lex(&ds, &month), Some("3".to_owned()));
        assert_eq!(lex(&ds, &day), Some("15".to_owned()));
    }

    #[test]
    fn hours_minutes_seconds_over_datetime() {
        let ds = empty_ds();
        let dt = typed_lit("2024-03-15T10:30:45Z", XSD_DATETIME);
        let hours = Expression::FunctionCall(Function::Hours, vec![dt.clone()]);
        let minutes = Expression::FunctionCall(Function::Minutes, vec![dt.clone()]);
        let seconds = Expression::FunctionCall(Function::Seconds, vec![dt]);
        assert_eq!(lex(&ds, &hours), Some("10".to_owned()));
        assert_eq!(lex(&ds, &minutes), Some("30".to_owned()));
        // SECONDS returns xsd:decimal; XSD 1.1 whole-decimal lexical has no point.
        assert_eq!(lex(&ds, &seconds), Some("45".to_owned()));
    }

    #[test]
    fn timezone_returns_daytime_duration() {
        let ds = empty_ds();
        // +05:30 offset → "PT5H30M"
        let dt = typed_lit("2024-03-15T10:30:00+05:30", XSD_DATETIME);
        let tz = Expression::FunctionCall(Function::Timezone, vec![dt]);
        let result = lex(&ds, &tz).expect("timezone result");
        assert_eq!(result, "PT5H30M");
    }

    #[test]
    fn timezone_utc_returns_pt0s() {
        let ds = empty_ds();
        let dt = typed_lit("2024-03-15T10:30:00Z", XSD_DATETIME);
        let tz = Expression::FunctionCall(Function::Timezone, vec![dt]);
        assert_eq!(lex(&ds, &tz), Some("PT0S".to_owned()));
    }

    #[test]
    fn tz_function_returns_string() {
        let ds = empty_ds();
        let dt_utc = typed_lit("2024-03-15T10:30:00Z", XSD_DATETIME);
        let dt_off = typed_lit("2024-03-15T10:30:00+05:30", XSD_DATETIME);
        let dt_none = typed_lit("2024-03-15T10:30:00", XSD_DATETIME);
        let tz_utc = Expression::FunctionCall(Function::Tz, vec![dt_utc]);
        let tz_off = Expression::FunctionCall(Function::Tz, vec![dt_off]);
        let tz_none = Expression::FunctionCall(Function::Tz, vec![dt_none]);
        assert_eq!(lex(&ds, &tz_utc), Some("Z".to_owned()));
        assert_eq!(lex(&ds, &tz_off), Some("+05:30".to_owned()));
        assert_eq!(lex(&ds, &tz_none), Some(String::new()));
    }

    // ---- ADJUST(value, timezone) -------------------------------------------
    //
    // The SPARQL 1.2 Query specification's Functions on Dates and Times table
    // (SEP-0002) maps ADJUST onto `fn:adjust-*-to-timezone` (XPath and XQuery
    // Functions and Operators §9.6); see `Function::Adjust` and
    // `adjust_timezone_arg` for the full source trail.

    const XSD_DATE: &str = "http://www.w3.org/2001/XMLSchema#date";
    const XSD_TIME: &str = "http://www.w3.org/2001/XMLSchema#time";
    const XSD_DAYTIME_DURATION: &str = "http://www.w3.org/2001/XMLSchema#dayTimeDuration";
    const XSD_YEARMONTH_DURATION: &str = "http://www.w3.org/2001/XMLSchema#yearMonthDuration";

    fn adjust(value: Expression, timezone: Expression) -> Expression {
        Expression::FunctionCall(Function::Adjust, vec![value, timezone])
    }

    /// The (lexical, datatype) pair of an evaluated constant expression.
    fn lex_and_dt(ds: &RdfDataset, expr: &Expression) -> Option<(String, String)> {
        let mut ctx = EvalCtx::new(ds);
        let schema = VarSchema::new();
        let term = eval_expr(expr, &[], &schema, &mut ctx).expect("eval")?;
        match value_of(&ctx, term) {
            TermValue::Literal {
                lexical_form,
                datatype,
                ..
            } => Some((lexical_form, datatype)),
            _ => None,
        }
    }

    #[test]
    fn adjust_datetime_shifts_an_existing_timezone() {
        let ds = empty_ds();
        // 10:30+05:30 (UTC 05:00) shifted to +01:00 → local 06:00.
        let e = adjust(
            typed_lit("2024-03-15T10:30:00+05:30", XSD_DATETIME),
            typed_lit("PT1H", XSD_DAYTIME_DURATION),
        );
        let (lex, dt) = lex_and_dt(&ds, &e).expect("adjust result");
        assert_eq!(lex, "2024-03-15T06:00:00+01:00");
        assert_eq!(dt, XSD_DATETIME);
    }

    #[test]
    fn adjust_datetime_attaches_to_a_tzless_value_without_shifting() {
        let ds = empty_ds();
        let e = adjust(
            typed_lit("2024-03-15T10:30:00", XSD_DATETIME),
            typed_lit("PT1H", XSD_DAYTIME_DURATION),
        );
        let (lex, _) = lex_and_dt(&ds, &e).expect("adjust result");
        assert_eq!(lex, "2024-03-15T10:30:00+01:00");
    }

    #[test]
    fn adjust_datetime_empty_string_removes_the_timezone() {
        let ds = empty_ds();
        // SPARQL has no empty sequence; the empty simple literal is the
        // ADJUST() stand-in for `fn:adjust-dateTime-to-timezone`'s
        // empty-`$timezone` "remove" case.
        let e = adjust(
            typed_lit("2024-03-15T10:30:00+05:30", XSD_DATETIME),
            lit(""),
        );
        let (lex, dt) = lex_and_dt(&ds, &e).expect("adjust result");
        assert_eq!(lex, "2024-03-15T10:30:00");
        assert_eq!(dt, XSD_DATETIME);
    }

    #[test]
    fn adjust_date_shifts_and_can_roll_the_day() {
        let ds = empty_ds();
        // `$timezone` is the ABSOLUTE target offset (per `fn:adjust-date-to-
        // timezone`): the date's own midnight is shifted by (target - source),
        // and any negative delta rolls the date back a day.
        let e = adjust(
            typed_lit("2024-03-15+01:00", XSD_DATE),
            typed_lit("-PT1H", XSD_DAYTIME_DURATION),
        );
        let (lex, dt) = lex_and_dt(&ds, &e).expect("adjust result");
        assert_eq!(lex, "2024-03-14-01:00");
        assert_eq!(dt, XSD_DATE);
    }

    #[test]
    fn adjust_date_removal() {
        let ds = empty_ds();
        let e = adjust(typed_lit("2024-03-15-07:00", XSD_DATE), lit(""));
        let (lex, _) = lex_and_dt(&ds, &e).expect("adjust result");
        assert_eq!(lex, "2024-03-15");
    }

    #[test]
    fn adjust_time_shifts_and_removes() {
        let ds = empty_ds();
        let shift = adjust(
            typed_lit("10:30:00+05:30", XSD_TIME),
            typed_lit("PT1H", XSD_DAYTIME_DURATION),
        );
        let (lex, dt) = lex_and_dt(&ds, &shift).expect("adjust result");
        assert_eq!(lex, "06:00:00+01:00");
        assert_eq!(dt, XSD_TIME);

        let removed = adjust(typed_lit("10:30:00+05:30", XSD_TIME), lit(""));
        let (lex, _) = lex_and_dt(&ds, &removed).expect("adjust result");
        assert_eq!(lex, "10:30:00");
    }

    #[test]
    fn adjust_out_of_range_timezone_is_a_type_error() {
        let ds = empty_ds();
        let e = adjust(
            typed_lit("2024-03-15T10:30:00Z", XSD_DATETIME),
            typed_lit("PT15H", XSD_DAYTIME_DURATION), // beyond ±14:00
        );
        assert_eq!(lex(&ds, &e), None);
    }

    #[test]
    fn adjust_non_whole_minute_timezone_is_a_type_error() {
        let ds = empty_ds();
        let e = adjust(
            typed_lit("2024-03-15T10:30:00Z", XSD_DATETIME),
            typed_lit("PT1H0M30S", XSD_DAYTIME_DURATION),
        );
        assert_eq!(lex(&ds, &e), None);
    }

    #[test]
    fn adjust_yearmonth_duration_second_arg_is_a_type_error() {
        let ds = empty_ds();
        // A nonzero-month duration can never be a timezone offset, regardless
        // of its lexical subtype tag — `adjust_timezone_arg` checks `months()`
        // by value, not the `xsd:yearMonthDuration` tag specifically.
        let e = adjust(
            typed_lit("2024-03-15T10:30:00Z", XSD_DATETIME),
            typed_lit("P1Y", XSD_YEARMONTH_DURATION),
        );
        assert_eq!(lex(&ds, &e), None);
    }

    #[test]
    fn adjust_non_temporal_first_arg_is_a_type_error() {
        let ds = empty_ds();
        let e = adjust(lit("not a date"), typed_lit("PT1H", XSD_DAYTIME_DURATION));
        assert_eq!(lex(&ds, &e), None);
    }

    // ---- SEP-0002 date/time/duration arithmetic, wired through `+ - * /` --

    const XSD_DURATION: &str = "http://www.w3.org/2001/XMLSchema#duration";

    #[test]
    fn datetime_plus_year_month_duration_clamps_to_month_end() {
        let ds = empty_ds();
        // 2024-01-31 + P1M must clamp to the TARGET month's last day AND land
        // on that month's leap day — "add 30 days" would give 2024-03-02,
        // which passes neither.
        let e = Expression::Add(
            Box::new(typed_lit("2024-01-31T00:00:00", XSD_DATETIME)),
            Box::new(typed_lit("P1M", XSD_YEARMONTH_DURATION)),
        );
        let (lex, dt) = lex_and_dt(&ds, &e).expect("dateTime + yearMonthDuration");
        assert_eq!(lex, "2024-02-29T00:00:00");
        assert_eq!(dt, XSD_DATETIME);
    }

    #[test]
    fn duration_plus_datetime_is_commuted() {
        let ds = empty_ds();
        let dt = typed_lit("2024-01-31T00:00:00", XSD_DATETIME);
        let dur = typed_lit("P1D", XSD_DAYTIME_DURATION);
        let forward = Expression::Add(Box::new(dt.clone()), Box::new(dur.clone()));
        let commuted = Expression::Add(Box::new(dur), Box::new(dt));
        let (flex, fdt) = lex_and_dt(&ds, &forward).expect("dateTime + duration");
        let (clex, cdt) = lex_and_dt(&ds, &commuted).expect("duration + dateTime");
        assert_eq!(flex, "2024-02-01T00:00:00");
        assert_eq!(clex, flex);
        assert_eq!(fdt, XSD_DATETIME);
        assert_eq!(cdt, fdt);
    }

    #[test]
    fn datetime_minus_duration_is_not_commutative() {
        let ds = empty_ds();
        // `-` has no commuted row: `duration - instant` is meaningless, even
        // though `instant - duration` (not tested here) is well-defined.
        let e = Expression::Subtract(
            Box::new(typed_lit("P1M", XSD_YEARMONTH_DURATION)),
            Box::new(typed_lit("2024-01-31T00:00:00", XSD_DATETIME)),
        );
        assert_eq!(lex(&ds, &e), None);
    }

    #[test]
    fn instant_difference_is_signed() {
        let ds = empty_ds();
        // earlier - later must be NEGATIVE; a `|a - b|` implementation would
        // pass an unsigned variant of this test but not this one.
        let e = Expression::Subtract(
            Box::new(typed_lit("2001-01-01T10:00:00Z", XSD_DATETIME)),
            Box::new(typed_lit("2001-01-10T10:00:00Z", XSD_DATETIME)),
        );
        let (lex, dt) = lex_and_dt(&ds, &e).expect("instant difference");
        assert_eq!(lex, "-P9D");
        assert_eq!(dt, XSD_DAYTIME_DURATION);
    }

    #[test]
    fn timezone_mix_is_indeterminate() {
        let ds = empty_ds();
        let e = Expression::Subtract(
            Box::new(typed_lit("2001-01-01T10:00:00Z", XSD_DATETIME)),
            Box::new(typed_lit("2001-01-01T10:00:00", XSD_DATETIME)),
        );
        assert_eq!(lex(&ds, &e), None);
    }

    /// The positive control for `timezone_mix_is_indeterminate`: without this,
    /// an unbound result there could mean "untimezoned instants never
    /// subtract" rather than "a timezone MIX is indeterminate".
    #[test]
    fn two_untimezoned_instants_still_subtract() {
        let ds = empty_ds();
        let e = Expression::Subtract(
            Box::new(typed_lit("2001-01-10T10:00:00", XSD_DATETIME)),
            Box::new(typed_lit("2001-01-01T10:00:00", XSD_DATETIME)),
        );
        let (lex, dt) = lex_and_dt(&ds, &e).expect("untimezoned difference");
        assert_eq!(lex, "P9D");
        assert_eq!(dt, XSD_DAYTIME_DURATION);
    }

    #[test]
    fn zero_result_keeps_the_operand_subtype() {
        let ds = empty_ds();
        // Dual discriminator A: a zero-valued yearMonthDuration result must
        // canonicalize as "P0M", not the general-duration "PT0S".
        let e = Expression::Subtract(
            Box::new(typed_lit("P1Y", XSD_YEARMONTH_DURATION)),
            Box::new(typed_lit("P1Y", XSD_YEARMONTH_DURATION)),
        );
        let (lex, dt) = lex_and_dt(&ds, &e).expect("P1Y - P1Y");
        assert_eq!(lex, "P0M");
        assert_eq!(dt, XSD_YEARMONTH_DURATION);
    }

    #[test]
    fn mixed_subtype_sum_is_the_general_duration() {
        let ds = empty_ds();
        // Dual discriminator B: the components alone look exactly like a
        // pure yearMonthDuration ("P1M"); only the declared tags differ, and
        // that must be enough to force the general xsd:duration result.
        let e = Expression::Add(
            Box::new(typed_lit("P1M", XSD_YEARMONTH_DURATION)),
            Box::new(typed_lit("PT0S", XSD_DAYTIME_DURATION)),
        );
        let (lex, dt) = lex_and_dt(&ds, &e).expect("P1M + PT0S");
        assert_eq!(lex, "P1M");
        assert_eq!(dt, XSD_DURATION);
    }

    #[test]
    fn integer_times_duration_commuted() {
        let ds = empty_ds();
        // The two-discriminant case: a numeric LEFT operand inside a
        // temporal domain routes through the same op as the numeric-first
        // form, not through a plain single-discriminant numeric dispatch.
        let n = typed_lit("3", XINT);
        let dur = typed_lit("P1D", XSD_DAYTIME_DURATION);
        let forward = Expression::Multiply(Box::new(n.clone()), Box::new(dur.clone()));
        let commuted = Expression::Multiply(Box::new(dur), Box::new(n));
        let (flex, fdt) = lex_and_dt(&ds, &forward).expect("3 * P1D");
        let (clex, cdt) = lex_and_dt(&ds, &commuted).expect("P1D * 3");
        assert_eq!(flex, "P3D");
        assert_eq!(clex, flex);
        assert_eq!(fdt, XSD_DAYTIME_DURATION);
        assert_eq!(cdt, fdt);
    }

    #[test]
    fn duration_times_a_double_factor_is_a_type_error() {
        let ds = empty_ds();
        const XSD_DOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";
        // The exact-tier rule: an inexact binary factor cannot scale an exact
        // duration without silent rounding, so this is a type error, not a
        // coerced multiplication.
        let dur = typed_lit("P1D", XSD_DAYTIME_DURATION);
        let by_double = Expression::Multiply(
            Box::new(dur.clone()),
            Box::new(typed_lit("1.5", XSD_DOUBLE)),
        );
        assert_eq!(lex(&ds, &by_double), None);
        // A NaN factor must stay a type error too, never coerce to zero.
        let by_nan = Expression::Multiply(Box::new(dur), Box::new(typed_lit("NaN", XSD_DOUBLE)));
        assert_eq!(lex(&ds, &by_nan), None);
    }

    #[test]
    fn numeric_divided_by_a_duration_is_a_type_error() {
        let ds = empty_ds();
        // Unlike `*`, `/` is not symmetric: `Nx / DUR` has no valid row even
        // though `DUR / Nx` does.
        let e = Expression::Divide(
            Box::new(typed_lit("3", XINT)),
            Box::new(typed_lit("P1D", XSD_DAYTIME_DURATION)),
        );
        assert_eq!(lex(&ds, &e), None);
    }

    #[test]
    fn unary_minus_on_a_duration_round_trips() {
        let ds = empty_ds();
        let d = typed_lit("P1Y2M", XSD_YEARMONTH_DURATION);
        let neg = Expression::UnaryMinus(Box::new(d));
        let (lex1, dt1) = lex_and_dt(&ds, &neg).expect("-(P1Y2M)");
        assert_eq!(lex1, "-P1Y2M");
        assert_eq!(dt1, XSD_YEARMONTH_DURATION);
        // -(-d) == d, evaluated as one nested expression.
        let double_neg = Expression::UnaryMinus(Box::new(neg));
        let (lex2, dt2) = lex_and_dt(&ds, &double_neg).expect("-(-(P1Y2M))");
        assert_eq!(lex2, "P1Y2M");
        assert_eq!(dt2, XSD_YEARMONTH_DURATION);
    }

    #[test]
    fn duration_equality_is_total_while_ordering_stays_indeterminate() {
        let ds = empty_ds();
        // "P1M" and "P30D" are value-incommensurable, so `<`/`>` stay
        // unbound — but `=` is total over the general xsd:duration and must
        // answer a bound `false`. An `=`-only test could pass even if the
        // fix landed in `value_cmp` instead of `sparql_value_eq`; this test
        // pins both outcomes side by side.
        let a = typed_lit("P1M", XSD_DURATION);
        let b = typed_lit("P30D", XSD_DURATION);
        let eq = Expression::Equal(Box::new(a.clone()), Box::new(b.clone()));
        assert_eq!(ebv(&ds, &eq), Some(false));
        let lt = Expression::Less(Box::new(a), Box::new(b));
        assert_eq!(ebv(&ds, &lt), None);
    }

    #[test]
    fn time_plus_duration_wraps_midnight() {
        let ds = empty_ds();
        // The only pin of `time`'s CyclicDay second-action through this
        // dispatch: crossing midnight in either direction must wrap, not
        // error or clamp.
        let forward = Expression::Add(
            Box::new(typed_lit("23:00:00", XSD_TIME)),
            Box::new(typed_lit("PT2H", XSD_DAYTIME_DURATION)),
        );
        let (flex, fdt) = lex_and_dt(&ds, &forward).expect("23:00:00 + PT2H");
        assert_eq!(flex, "01:00:00");
        assert_eq!(fdt, XSD_TIME);

        let backward = Expression::Subtract(
            Box::new(typed_lit("01:00:00", XSD_TIME)),
            Box::new(typed_lit("PT2H", XSD_DAYTIME_DURATION)),
        );
        let (blex, bdt) = lex_and_dt(&ds, &backward).expect("01:00:00 - PT2H");
        assert_eq!(blex, "23:00:00");
        assert_eq!(bdt, XSD_TIME);
    }

    // ---- Gap 4: NOW() with fixed ctx.now ----------------------------------

    #[test]
    fn now_returns_ctx_now() {
        let ds = empty_ds();
        // Override now with a known value for deterministic testing.
        let known_dt = purrdf_xsd::datetime_from_unix_seconds(0);
        let mut ctx = EvalCtx::new(&ds).with_now(XsdValue::DateTime(known_dt));
        let schema = VarSchema::new();
        let expr = Expression::FunctionCall(Function::Now, vec![]);
        let term = eval_expr(&expr, &[], &schema, &mut ctx)
            .expect("NOW()")
            .expect("some");
        match value_of(&ctx, term) {
            TermValue::Literal { lexical_form, .. } => {
                assert_eq!(lexical_form, "1970-01-01T00:00:00Z");
            }
            other => panic!("expected literal, got {other:?}"),
        }
    }

    #[test]
    fn xsd_float_double_cast_pins_xsd_1_0_positive_infinity() {
        // The operand-mapping rules pin XSD 1.0: `INF` casts, but the XSD 1.1
        // `+INF` spelling is a lexical error (the cast yields unbound).
        let ds = empty_ds();
        for dt in [
            "http://www.w3.org/2001/XMLSchema#double",
            "http://www.w3.org/2001/XMLSchema#float",
        ] {
            let cast = |lex: &str| {
                Expression::FunctionCall(
                    Function::Custom(NamedNode::new_unchecked(dt)),
                    vec![lit(lex)],
                )
            };
            assert_eq!(
                lex(&ds, &cast("INF")).as_deref(),
                Some("INF"),
                "INF casts for <{dt}>"
            );
            assert_eq!(
                lex(&ds, &cast("+INF")),
                None,
                "+INF is a cast error for <{dt}>"
            );
        }
    }

    #[test]
    fn double_cast_of_a_double_typed_plus_inf_source_is_by_value() {
        // The lexical constructor from a string applies the XSD-1.0 rules (above),
        // but a numeric→numeric cast goes by VALUE (SPARQL 17.1): a source already
        // typed `xsd:double` carries the value +INF, and casting that value to
        // double is identity. So the two entry points differ by design.
        let ds = empty_ds();
        let dbl = "http://www.w3.org/2001/XMLSchema#double";
        let expr = Expression::FunctionCall(
            Function::Custom(NamedNode::new_unchecked(dbl)),
            vec![typed_lit("+INF", dbl)],
        );
        assert_eq!(lex(&ds, &expr).as_deref(), Some("INF"));
    }

    // ---- Gap 4: RAND() deterministic with fixed seed ----------------------

    #[test]
    fn rand_deterministic_with_fixed_seed() {
        let ds = empty_ds();
        let mut ctx = EvalCtx::new(&ds).with_rng_seed(12345);
        let schema = VarSchema::new();
        let expr = Expression::FunctionCall(Function::Rand, vec![]);
        // First call
        let t1 = eval_expr(&expr, &[], &schema, &mut ctx)
            .expect("rand1")
            .expect("some");
        let v1 = value_of(&ctx, t1);
        // Second call with same seed-after-first
        let t2 = eval_expr(&expr, &[], &schema, &mut ctx)
            .expect("rand2")
            .expect("some");
        let v2 = value_of(&ctx, t2);
        // Both must be xsd:double literals in [0, 1)
        if let TermValue::Literal {
            lexical_form: lex1, ..
        } = &v1
        {
            let f1: f64 = lex1.parse().unwrap_or(f64::NAN);
            assert!((0.0..1.0).contains(&f1), "first rand {f1} not in [0,1)");
        } else {
            panic!("rand1 not a literal");
        }
        if let TermValue::Literal {
            lexical_form: lex2, ..
        } = &v2
        {
            let f2: f64 = lex2.parse().unwrap_or(f64::NAN);
            assert!((0.0..1.0).contains(&f2), "second rand {f2} not in [0,1)");
        } else {
            panic!("rand2 not a literal");
        }
        // The two values must differ (splitmix64 is not degenerate for non-zero seeds)
        assert_ne!(v1, v2, "rand should differ across calls");
    }

    // ---- Gap 4: UUID() well-formed urn:uuid: shape ------------------------

    #[test]
    fn uuid_is_well_formed_urn() {
        let ds = empty_ds();
        let mut ctx = EvalCtx::new(&ds);
        ctx.rng_state = 0xDEAD_BEEF_CAFE_BABEu64;
        let schema = VarSchema::new();
        let expr = Expression::FunctionCall(Function::Uuid, vec![]);
        let term = eval_expr(&expr, &[], &schema, &mut ctx)
            .expect("UUID")
            .expect("some");
        let val = value_of(&ctx, term);
        if let TermValue::Iri(iri) = &val {
            assert!(
                iri.starts_with("urn:uuid:"),
                "UUID IRI must start with urn:uuid:"
            );
            let uuid_part = &iri["urn:uuid:".len()..];
            let parts: Vec<&str> = uuid_part.split('-').collect();
            assert_eq!(parts.len(), 5, "UUID must have 5 dash-separated groups");
            assert_eq!(parts[0].len(), 8);
            assert_eq!(parts[1].len(), 4);
            assert_eq!(parts[2].len(), 4);
            assert_eq!(parts[3].len(), 4);
            assert_eq!(parts[4].len(), 12);
            // version 4 check: first char of group 3 must be '4'
            assert_eq!(&parts[2][..1], "4", "UUID version must be 4");
            // variant check: first char of group 4 must be '8', '9', 'a', or 'b'
            let variant_char = parts[3].chars().next().unwrap();
            assert!(
                matches!(variant_char, '8' | '9' | 'a' | 'b'),
                "UUID variant nibble {variant_char} must be 8/9/a/b"
            );
        } else {
            panic!("UUID() must produce an IRI, got {val:?}");
        }
    }

    #[test]
    fn struuid_is_well_formed_string() {
        let ds = empty_ds();
        let mut ctx = EvalCtx::new(&ds);
        ctx.rng_state = 0x1234_5678_9ABC_DEF0u64;
        let schema = VarSchema::new();
        let expr = Expression::FunctionCall(Function::StrUuid, vec![]);
        let term = eval_expr(&expr, &[], &schema, &mut ctx)
            .expect("STRUUID")
            .expect("some");
        let val = value_of(&ctx, term);
        if let TermValue::Literal { lexical_form, .. } = &val {
            let parts: Vec<&str> = lexical_form.split('-').collect();
            assert_eq!(parts.len(), 5);
            assert_eq!(&parts[2][..1], "4");
        } else {
            panic!("STRUUID() must produce a literal");
        }
    }

    // ── EXISTS decorrelation ──────────────────────────────────────────────────

    /// `:a :knows :b`, `:a :knows :c`, `:b :member :club` — duplicate outer
    /// subjects (`:a`) so a per-row EXISTS would re-evaluate the inner repeatedly.
    fn knows_ds() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let knows = b.intern_iri("http://ex/knows");
        let member = b.intern_iri("http://ex/member");
        let a = b.intern_iri("http://ex/a");
        let bb = b.intern_iri("http://ex/b");
        let c = b.intern_iri("http://ex/c");
        let club = b.intern_iri("http://ex/club");
        b.push_quad(a, knows, bb, None);
        b.push_quad(a, knows, c, None);
        b.push_quad(bb, member, club, None);
        b.freeze().expect("freeze")
    }

    /// Run `query` against `ds` with the EXISTS memo on/off, returning sorted
    /// stringified rows for a multiset comparison.
    fn run_rows(ds: &RdfDataset, query: &str, memo: bool) -> Vec<Vec<String>> {
        use crate::eval::Outcome;
        use crate::eval::evaluate_query;
        use purrdf_sparql_algebra::SparqlParser;

        let parsed = SparqlParser::new().parse_query(query).expect("parse");
        let mut ctx = EvalCtx::new(ds);
        ctx.options.exists_memo = memo;
        match evaluate_query(&parsed, &mut ctx).expect("eval") {
            Outcome::Solutions(seq) => {
                let mut out: Vec<Vec<String>> = seq
                    .rows
                    .iter()
                    .map(|row| {
                        row.iter()
                            .map(|c| match c {
                                None => "UNBOUND".to_owned(),
                                Some(t) => match value_of(&ctx, *t) {
                                    TermValue::Iri(i) => format!("<{i}>"),
                                    TermValue::Literal { lexical_form, .. } => lexical_form,
                                    TermValue::Blank { label, .. } => format!("_:{label}"),
                                    TermValue::Triple { .. } => "<<triple>>".to_owned(),
                                },
                            })
                            .collect()
                    })
                    .collect();
                out.sort();
                out
            }
            other => panic!("expected solutions, got {other:?}"),
        }
    }

    /// The memo MUST be transparent: identical results with it on and off.
    fn assert_memo_transparent(query: &str) {
        let ds = knows_ds();
        assert_eq!(
            run_rows(&ds, query, true),
            run_rows(&ds, query, false),
            "memo changed results for `{query}`"
        );
    }

    #[test]
    fn exists_memo_matches_naive_positive() {
        // ?o ∈ {:b, :c}; only :b has a member → keep the :a→:b row.
        let q = "SELECT ?s ?o WHERE { ?s <http://ex/knows> ?o \
                 FILTER EXISTS { ?o <http://ex/member> ?m } }";
        assert_memo_transparent(q);
        assert_eq!(
            run_rows(&knows_ds(), q, true),
            vec![vec!["<http://ex/a>".to_owned(), "<http://ex/b>".to_owned()]]
        );
    }

    #[test]
    fn exists_memo_matches_naive_not_exists() {
        // NOT EXISTS anti-join: keep the :a→:c row (:c has no member).
        let q = "SELECT ?s ?o WHERE { ?s <http://ex/knows> ?o \
                 FILTER NOT EXISTS { ?o <http://ex/member> ?m } }";
        assert_memo_transparent(q);
        assert_eq!(
            run_rows(&knows_ds(), q, true),
            vec![vec!["<http://ex/a>".to_owned(), "<http://ex/c>".to_owned()]]
        );
    }

    #[test]
    fn exists_memo_uncorrelated_inner() {
        // The inner shares no variable with the outer row (constant existence):
        // EXISTS holds for every outer row → both rows kept.
        let q = "SELECT ?s ?o WHERE { ?s <http://ex/knows> ?o \
                 FILTER EXISTS { ?x <http://ex/member> ?m } }";
        assert_memo_transparent(q);
        assert_eq!(run_rows(&knows_ds(), q, true).len(), 2);
    }

    #[test]
    fn exists_memo_populates_cache_once() {
        // Two outer rows share the same EXISTS site; with the memo on the inner
        // pattern is evaluated and cached exactly once.
        //
        // Driven directly via `eval`/`eval_ebv` on ONE shared `ctx`, rather than
        // through `evaluate_query`'s FILTER node: this EXISTS reaches no unsafe
        // builtin, so `eval_filter` routes it through
        // `crate::parallel::par_chunk_try_map_init`, which runs the per-row loop on a
        // FORKED child context — the memo would land on that (discarded-after-use)
        // child, not on a `ctx` inspected from outside `evaluate_query`. This
        // reproduces the identical per-row loop shape the forked child runs,
        // directly on `ctx`, to keep exercising the underlying "cache built once,
        // not once per outer row" invariant.
        use purrdf_sparql_algebra::{
            NamedNode, NamedNodePattern, TermPattern, TriplePattern, Variable,
        };

        let ds = knows_ds();
        let vp = |n: &str| TermPattern::Variable(Variable::new(n));
        let pred = |iri: &str| NamedNodePattern::NamedNode(NamedNode::new_unchecked(iri));
        let outer = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: vp("s"),
                predicate: pred("http://ex/knows"),
                object: vp("o"),
            }],
        };
        let inner = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: vp("z"),
                predicate: pred("http://ex/member"),
                object: vp("m"),
            }],
        };

        let mut ctx = EvalCtx::new(&ds);
        let seq = eval(&outer, &mut ctx).expect("outer bgp");
        let exists_expr = Expression::Exists(Box::new(inner));
        for row in &seq.rows {
            eval_ebv(&exists_expr, row, &seq.schema, &mut ctx).expect("ebv");
        }
        assert_eq!(
            ctx.exists_inner_cache.len(),
            1,
            "the single EXISTS site must cache exactly one inner result"
        );
    }

    // ── Correlated EXISTS: outer variable referenced in FILTER expression ──────
    //
    // Data: :a :knows :b ; :b :knows :c .
    //       :a :p :x .              (only :a has a :p property, :b does not)
    //
    // Query: SELECT ?s WHERE { ?s :knows ?o FILTER EXISTS { ?x :p ?y FILTER(?s = ?x) } }
    //
    // The EXISTS inner pattern references the outer-bound ?s inside a FILTER expression.
    // Correct result: only :a (because :a :p :x exists and :a = :a passes;
    //                          :b has no :p so the FILTER-constrained scan finds nothing).
    //
    // Buggy (old) behaviour: the inner is evaluated unconstrained, so ?s is unbound
    // inside FILTER(?s = ?x), which errors → all inner rows dropped → EXISTS always
    // false → zero rows returned — which is provably wrong.

    fn correlated_ds() -> Arc<RdfDataset> {
        // :a :knows :b
        // :b :knows :c
        // :a :p :x    ← only :a has :p
        let mut b = RdfDatasetBuilder::new();
        let knows = b.intern_iri("http://ex/knows");
        let p = b.intern_iri("http://ex/p");
        let a = b.intern_iri("http://ex/a");
        let bb = b.intern_iri("http://ex/b");
        let c = b.intern_iri("http://ex/c");
        let x = b.intern_iri("http://ex/x");
        b.push_quad(a, knows, bb, None);
        b.push_quad(bb, knows, c, None);
        b.push_quad(a, p, x, None);
        b.freeze().expect("freeze")
    }

    #[test]
    fn correlated_filter_exists_returns_correct_result() {
        // The EXISTS inner FILTER references outer ?s — the expression-correlated path
        // must be taken. Only :a should be returned (it has :p; :b does not).
        let ds = correlated_ds();
        let q = "SELECT ?s WHERE { \
                   ?s <http://ex/knows> ?o \
                   FILTER EXISTS { ?x <http://ex/p> ?y FILTER(?s = ?x) } \
                 }";
        let rows = run_rows(&ds, q, true);
        assert_eq!(
            rows,
            vec![vec!["<http://ex/a>".to_owned()]],
            "correlated EXISTS must return exactly :a (the subject with :p)"
        );
    }

    #[test]
    fn correlated_filter_exists_memo_off_matches_memo_on() {
        // memo=off is the reference (per-row naive); memo=on must agree.
        let ds = correlated_ds();
        let q = "SELECT ?s WHERE { \
                   ?s <http://ex/knows> ?o \
                   FILTER EXISTS { ?x <http://ex/p> ?y FILTER(?s = ?x) } \
                 }";
        assert_eq!(
            run_rows(&ds, q, true),
            run_rows(&ds, q, false),
            "memo must not change results for correlated EXISTS"
        );
    }

    #[test]
    fn correlated_not_exists_inverts_correctly() {
        // NOT EXISTS with correlated inner: :b (no :p) should survive; :a (has :p) drops.
        let ds = correlated_ds();
        let q = "SELECT ?s WHERE { \
                   ?s <http://ex/knows> ?o \
                   FILTER NOT EXISTS { ?x <http://ex/p> ?y FILTER(?s = ?x) } \
                 }";
        let rows = run_rows(&ds, q, true);
        assert_eq!(
            rows,
            vec![vec!["<http://ex/b>".to_owned()]],
            "correlated NOT EXISTS must return exactly :b (the subject without :p)"
        );
    }

    #[test]
    fn uncorrelated_exists_fast_path_still_uses_cache() {
        // Verify the fast/memoized path is still taken when there is no expression
        // correlation: the cache must be populated after the query.
        //
        // Driven directly via `eval`/`eval_ebv` on ONE shared `ctx` rather than
        // through `evaluate_query`'s FILTER node — see
        // `exists_memo_populates_cache_once`'s comment: a parallel-safe FILTER is
        // routed through a forked child context, so the memo would land there, not
        // on a `ctx` inspected from outside `evaluate_query`.
        use purrdf_sparql_algebra::{
            NamedNode, NamedNodePattern, TermPattern, TriplePattern, Variable,
        };

        let ds = knows_ds();
        let vp = |n: &str| TermPattern::Variable(Variable::new(n));
        let pred = |iri: &str| NamedNodePattern::NamedNode(NamedNode::new_unchecked(iri));
        let outer = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: vp("s"),
                predicate: pred("http://ex/knows"),
                object: vp("o"),
            }],
        };
        let inner = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: vp("z"),
                predicate: pred("http://ex/member"),
                object: vp("m"),
            }],
        };

        let mut ctx = EvalCtx::new(&ds);
        let seq = eval(&outer, &mut ctx).expect("outer bgp");
        let exists_expr = Expression::Exists(Box::new(inner));
        for row in &seq.rows {
            eval_ebv(&exists_expr, row, &seq.schema, &mut ctx).expect("ebv");
        }
        assert_eq!(
            ctx.exists_inner_cache.len(),
            1,
            "uncorrelated EXISTS must still populate the memo cache"
        );
    }

    // ── EXISTS memo under a governor trip ─────────────────────────────────────
    //
    // The memo holds ROW DATA keyed by the inner pattern's address, and a forked worker
    // inherits a clone of it. A bag the budget cut short, memoized under that key, is
    // read back by each subsequent probe as though it were the inner pattern's answer:
    // `EXISTS` then reports `false` where the answer is `true`, and `NOT EXISTS` — which
    // is where it really bites — reports `true`, FABRICATING an outer row that the true
    // answer does not contain. A missing row is a bound; an invented one is not.

    /// Four outer subjects, each with one `:knows` edge, of which two (`s1`, `s3`) also
    /// have a `:member` triple. `EXISTS { ?s :member ?c }` is therefore true for half the
    /// outer rows and false for the other half, so a memo poisoned with an empty or short
    /// inner bag changes the answer instead of merely shortening it.
    fn exists_governor_ds() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let knows = b.intern_iri("http://example.org/knows");
        let member = b.intern_iri("http://example.org/member");
        let club = b.intern_iri("http://example.org/club");
        for i in 1..=4 {
            let s = b.intern_iri(&format!("http://example.org/s{i}"));
            let o = b.intern_iri(&format!("http://example.org/o{i}"));
            b.push_quad(s, knows, o, None);
            if i % 2 == 1 {
                b.push_quad(s, member, club, None);
            }
        }
        b.freeze().expect("freeze")
    }

    /// `{ ?s :knows ?o }` and `{ ?s :member ?c }` — the outer pattern and the `EXISTS`
    /// inner pattern over [`exists_governor_ds`]. They share `?s` in a triple-pattern
    /// position (never in an expression), so `exists` takes the memoized fast path and
    /// probes the shared index — the path that owns the cache under test.
    fn exists_governor_patterns() -> (GraphPattern, GraphPattern) {
        use purrdf_sparql_algebra::{NamedNodePattern, TermPattern, TriplePattern};
        let vp = |n: &str| TermPattern::Variable(Variable::new(n));
        let pred = |iri: &str| NamedNodePattern::NamedNode(NamedNode::new_unchecked(iri));
        let bgp = |s, p, o| GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: s,
                predicate: p,
                object: o,
            }],
        };
        (
            bgp(vp("s"), pred("http://example.org/knows"), vp("o")),
            bgp(vp("s"), pred("http://example.org/member"), vp("c")),
        )
    }

    #[test]
    fn a_truncated_exists_inner_is_never_memoized() {
        use crate::eval::eval_evaluated;
        use crate::governor::lift::Evaluated;

        let ds = exists_governor_ds();
        let (outer, inner) = exists_governor_patterns();

        // The inner pattern's true, complete bag — the only bag the memo may ever hold.
        let mut reference_ctx = EvalCtx::new(&ds);
        let reference = eval(&inner, &mut reference_ctx).expect("inner");
        assert!(
            !reference.rows.is_empty(),
            "the fixture must match the inner"
        );

        let exists_expr = Expression::Exists(Box::new(inner));
        let mut saw_truncated_inner = false;
        let mut saw_memo = false;

        // Sweep the budget rather than tuning one number to the current charge schedule:
        // the point is that NO budget can leave a short bag in the memo, and a swept
        // budget keeps proving that when the schedule moves.
        for fuel in 1..=64_u64 {
            let governors = crate::QueryGovernors::UNBOUNDED.with_fuel(fuel);
            let state = Arc::new(crate::GovernorState::new(&governors));
            let mut ctx = EvalCtx::new(&ds).with_governors(Arc::clone(&state));

            let outer_rows = match eval_evaluated(&outer, &mut ctx).expect("outer") {
                Evaluated::Complete(seq) => seq,
                Evaluated::Truncated(truncation) => truncation.split().0,
            };
            for row in &outer_rows.rows {
                eval_ebv(&exists_expr, row, &outer_rows.schema, &mut ctx).expect("ebv");
            }

            if ctx.expression_barrier.observed().is_some() {
                // The inner was cut short at this budget: nothing may have been written
                // under the site's key, so the next probe re-evaluates instead of reading
                // a bag that is not the inner pattern's answer.
                saw_truncated_inner = true;
                assert!(
                    ctx.exists_inner_cache.is_empty(),
                    "fuel {fuel}: a truncated EXISTS inner was memoized"
                );
            }
            for entry in ctx.exists_inner_cache.values() {
                saw_memo = true;
                assert_eq!(
                    entry.inner.rows, reference.rows,
                    "fuel {fuel}: a memoized EXISTS inner must be the complete inner bag"
                );
            }
        }

        assert!(
            saw_truncated_inner,
            "no budget in the sweep stopped the execution inside the EXISTS inner, so \
             the test proves nothing about a truncated one"
        );
        assert!(
            saw_memo,
            "no budget in the sweep let the inner complete and populate the memo, so the \
             test would also pass against an evaluator that never memoizes at all"
        );
    }

    #[test]
    fn a_truncated_subtree_cannot_poison_a_later_evaluation() {
        use crate::eval::eval_evaluated;
        use crate::governor::lift::Evaluated;

        // Every governor latches: fuel and the cell/scratch counters only ever rise, and
        // a stop signal that has fired keeps firing, so once an execution has tripped it
        // stays tripped and there is no "and now a clean subtree" left INSIDE that
        // execution. The poisoning question therefore has to be asked across executions,
        // of everything a tripped one can write into and a later one can read: the plan
        // (the same value, so every address-keyed memo sees the same keys), the dataset,
        // and the engine-lived BGP join-order cache. Anything per-execution is discarded
        // with the context; anything that is not, this test reaches.
        let ds = exists_governor_ds();
        let (outer, inner) = exists_governor_patterns();
        let plan = GraphPattern::Filter {
            expr: Expression::Not(Box::new(Expression::Exists(Box::new(inner)))),
            inner: Box::new(outer),
        };
        let order_cache = crate::eval::BgpOrderCache::default();

        // `NOT EXISTS` keeps exactly the outer rows whose subject has no `:member`. A
        // short inner bag makes the negation true for rows it is false for, so poisoning
        // shows up here as EXTRA rows — the failure mode a subset-only certificate cannot
        // rule out and this assertion can.
        let mut baseline_ctx = EvalCtx::new(&ds);
        let baseline = eval(&plan, &mut baseline_ctx).expect("ungoverned");
        assert_eq!(
            baseline.rows.len(),
            2,
            "the fixture must keep exactly the two subjects without :member"
        );

        let mut saw_trip = false;
        for fuel in 1..=80_u64 {
            let governors = crate::QueryGovernors::UNBOUNDED.with_fuel(fuel);
            let state = Arc::new(crate::GovernorState::new(&governors));
            {
                let mut governed = EvalCtx::new(&ds)
                    .with_order_cache(&order_cache)
                    .with_governors(Arc::clone(&state));
                saw_trip |= matches!(
                    eval_evaluated(&plan, &mut governed).expect("governed"),
                    Evaluated::Truncated(_)
                );
            }

            let mut after = EvalCtx::new(&ds).with_order_cache(&order_cache);
            let again = eval(&plan, &mut after).expect("after the tripped run");
            assert_eq!(
                again.rows, baseline.rows,
                "fuel {fuel}: an execution stopped by a governor changed what a later, \
                 ungoverned execution of the same plan answered"
            );
        }
        assert!(
            saw_trip,
            "no budget in the sweep stopped the execution, so no truncated subtree was \
             ever produced to poison anything"
        );
    }

    // ── Correlated EXISTS over many outer rows: address-reuse cache hazard ─────
    //
    // Regression guard for the `ctx.in_substituted_exists` cache bypass: the
    // per-row `substitute_pattern` temporary built inside the expression-
    // correlated branch of `exists()` is a fresh heap allocation that is
    // dropped at the end of each outer row's evaluation. Across many rows the
    // allocator can (and in practice does) hand back the *same address* for
    // the next row's temporary. Before the fix, `const_atom_cache`,
    // `exists_expr_vars_cache`, and `exists_inner_cache` were keyed on that
    // address, so a later row could get a stale cache hit computed against an
    // earlier row's substituted constant — corrupting the solution set. This
    // test drives five outer rows (more than enough for address reuse to
    // occur) with an alternating true/false correlated-FILTER-EXISTS result,
    // so any stale hit flips at least one row to the wrong answer.

    /// Five outer subjects `?s`, each with a single `:knows` edge (so each
    /// contributes exactly one outer row). Only the odd-numbered subjects
    /// (`s1`, `s3`, `s5`) additionally have a `:p` triple.
    fn correlated_multi_row_ds() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let knows = b.intern_iri("http://example.org/knows");
        let p = b.intern_iri("http://example.org/p");
        let x = b.intern_iri("http://example.org/x");
        for i in 1..=5 {
            let s = b.intern_iri(&format!("http://example.org/s{i}"));
            let o = b.intern_iri(&format!("http://example.org/o{i}"));
            b.push_quad(s, knows, o, None);
            if i % 2 == 1 {
                // Odd subjects (s1, s3, s5) have :p; even ones (s2, s4) do not.
                b.push_quad(s, p, x, None);
            }
        }
        b.freeze().expect("freeze")
    }

    #[test]
    fn correlated_exists_substitution_ignores_address_keyed_caches() {
        // Each outer row substitutes a DIFFERENT constant for ?s into the inner
        // FILTER(?s = ?x); the expected result alternates true/false/true/false/true
        // across s1..s5. A stale address-keyed cache hit (the bug this guards)
        // would carry an earlier row's substituted result into a later row and
        // flip at least one entry — so the exact set below only holds because
        // `ctx.in_substituted_exists` forces every row's substitution to be
        // evaluated fresh.
        let ds = correlated_multi_row_ds();
        let q = "SELECT ?s WHERE { \
                   ?s <http://example.org/knows> ?o \
                   FILTER EXISTS { ?x <http://example.org/p> ?y FILTER(?s = ?x) } \
                 }";
        let rows = run_rows(&ds, q, true);
        assert_eq!(
            rows,
            vec![
                vec!["<http://example.org/s1>".to_owned()],
                vec!["<http://example.org/s3>".to_owned()],
                vec!["<http://example.org/s5>".to_owned()],
            ],
            "correlated EXISTS across many outer rows must return exactly the \
             odd-numbered subjects (s1, s3, s5), each judged against its OWN \
             substituted constant"
        );
        // Cross-check against the memo-off (naive per-row) reference path too.
        assert_eq!(
            rows,
            run_rows(&ds, q, false),
            "memo on/off must agree for the multi-row correlated EXISTS"
        );
    }

    // ── Arithmetic inside FILTER NOT EXISTS ────────────────────────────────────
    //
    // SPARQL 1.1 §18.6/§17.4.1.5: an outer row whose inner EXISTS group finds NO
    // matching solution SURVIVES `FILTER NOT EXISTS`. The inner group carries an
    // arithmetic FILTER — in (1a) over the inner's own re-bound value, in (1b) over
    // an OUTER-bound variable substituted into the inner. A row is removed only when
    // the arithmetic makes the inner group non-empty; a row whose inner FILTER kills
    // the only candidate has an empty inner group and must survive. These lock the
    // regression where these shapes returned an empty solution set.

    /// `ex:x :v 5 ; :w 3` (v-value 5 ≤ 10) and `ex:y :v 15` (v-value 15 > 10, no
    /// `:w`) — the (1a) fixture.
    fn arith_not_exists_1a_ds() -> Arc<RdfDataset> {
        use purrdf_core::RdfLiteral;
        let int = |lex: &str| RdfLiteral {
            lexical_form: lex.to_owned(),
            datatype: Some(XINT.to_owned()),
            language: None,
            direction: None,
        };
        let mut b = RdfDatasetBuilder::new();
        let v = b.intern_iri("http://example.org/v");
        let w = b.intern_iri("http://example.org/w");
        let x = b.intern_iri("http://example.org/x");
        let y = b.intern_iri("http://example.org/y");
        let i5 = b.intern_literal(int("5"));
        let i15 = b.intern_literal(int("15"));
        let i3 = b.intern_literal(int("3"));
        b.push_quad(x, v, i5, None);
        b.push_quad(x, w, i3, None);
        b.push_quad(y, v, i15, None);
        b.freeze().expect("freeze")
    }

    #[test]
    fn arithmetic_filter_not_exists_survives_empty_inner_group() {
        // ?s=x: ?a=5, inner FILTER(5 > 10) is false ⇒ inner group empty ⇒ NOT EXISTS
        //        holds ⇒ x SURVIVES.
        // ?s=y: ?a=15, inner FILTER(15 > 10) is true ⇒ inner group non-empty ⇒ y drops.
        let ds = arith_not_exists_1a_ds();
        let q = "SELECT ?s WHERE { \
                   ?s <http://example.org/v> ?a . \
                   OPTIONAL { ?s <http://example.org/w> ?b } \
                   FILTER NOT EXISTS { ?s <http://example.org/v> ?a . FILTER(?a > 10) } \
                 }";
        let rows = run_rows(&ds, q, true);
        assert_eq!(
            rows,
            vec![vec!["<http://example.org/x>".to_owned()]],
            "the row whose inner arithmetic FILTER kills its only candidate must survive"
        );
        assert_eq!(rows, run_rows(&ds, q, false), "memo must be transparent");
    }

    /// `ex:x :v 10 ; :w 3` (inner 3-10 = -7, not > 0) and `ex:y :v 1 ; :w 5`
    /// (inner 5-1 = 4 > 0) — the (1b) correlated fixture.
    fn arith_not_exists_1b_ds() -> Arc<RdfDataset> {
        use purrdf_core::RdfLiteral;
        let int = |lex: &str| RdfLiteral {
            lexical_form: lex.to_owned(),
            datatype: Some(XINT.to_owned()),
            language: None,
            direction: None,
        };
        let mut b = RdfDatasetBuilder::new();
        let v = b.intern_iri("http://example.org/v");
        let w = b.intern_iri("http://example.org/w");
        let x = b.intern_iri("http://example.org/x");
        let y = b.intern_iri("http://example.org/y");
        let i10 = b.intern_literal(int("10"));
        let i3 = b.intern_literal(int("3"));
        let i1 = b.intern_literal(int("1"));
        let i5 = b.intern_literal(int("5"));
        b.push_quad(x, v, i10, None);
        b.push_quad(x, w, i3, None);
        b.push_quad(y, v, i1, None);
        b.push_quad(y, w, i5, None);
        b.freeze().expect("freeze")
    }

    #[test]
    fn correlated_arithmetic_filter_not_exists_uses_outer_binding() {
        // The inner FILTER references the OUTER-bound ?a in an arithmetic term, so the
        // expression-correlated substitution path must make ?a visible inside:
        //   ?s=x: ?a=10, inner FILTER(3 - 10 > 0) is false ⇒ inner empty ⇒ x SURVIVES.
        //   ?s=y: ?a=1,  inner FILTER(5 - 1  > 0) is true  ⇒ inner non-empty ⇒ y drops.
        // If the outer ?a were NOT visible inside (unbound), the arithmetic would
        // error for BOTH rows, every inner group would be empty, and both rows would
        // wrongly survive — so this exact singleton set only holds with §18.6
        // substitution correct.
        let ds = arith_not_exists_1b_ds();
        let q = "SELECT ?s WHERE { \
                   ?s <http://example.org/v> ?a . \
                   FILTER NOT EXISTS { ?s <http://example.org/w> ?b . FILTER(?b - ?a > 0) } \
                 }";
        let rows = run_rows(&ds, q, true);
        assert_eq!(
            rows,
            vec![vec!["<http://example.org/x>".to_owned()]],
            "the correlated outer ?a must be visible in the inner arithmetic FILTER"
        );
        assert_eq!(rows, run_rows(&ds, q, false), "memo must be transparent");
    }

    // ── heldIn extension function ──────────────────────────────────────────────

    /// The `heldIn` extension call node as parsed under a caller-configured
    /// example.org namespace (the original IRI rides along for serialization).
    fn held_in_fn() -> Function {
        Function::Purrdf(purrdf_sparql_algebra::PurrdfCall {
            fn_kind: PurrdfFn::HeldIn,
            iri: "https://example.org/ext/heldIn".to_owned(),
        })
    }

    /// A pure-fixture (example.org) standpoint vocabulary — the predicate table is
    /// caller-supplied configuration: any ontology's IRIs work when configured.
    const EX_ACCORDING_TO: &str = "http://example.org/accordingTo";
    const EX_SHARPENS: &str = "http://example.org/sharpens";

    /// The fixture's caller-supplied standpoint predicate table.
    fn ex_standpoints() -> crate::eval::StandpointPredicates {
        crate::eval::StandpointPredicates::new(EX_ACCORDING_TO, EX_SHARPENS)
    }

    /// Build a dataset with a reifier `R` of a reified statement, annotated
    /// `R ex:accordingTo T1`, plus a direct `T1 ex:sharpens T2` edge.
    /// `T3` is an unrelated standpoint.
    fn held_in_ds() -> Arc<RdfDataset> {
        use purrdf_core::RdfLiteral;
        let mut b = RdfDatasetBuilder::new();
        let reifier = b.intern_iri("http://ex/r");
        let s = b.intern_iri("http://ex/s");
        let p = b.intern_iri("http://ex/p");
        let o = b.intern_literal(RdfLiteral::simple("v"));
        let t1 = b.intern_iri("http://ex/T1");
        let t2 = b.intern_iri("http://ex/T2");
        let _t3 = b.intern_iri("http://ex/T3");
        let according_to = b.intern_iri(EX_ACCORDING_TO);
        let sharpens = b.intern_iri(EX_SHARPENS);
        // The reified triple-term `<<( s p o )>>` and its reifier binding.
        let triple = b.intern_triple(s, p, o);
        b.push_reifier(reifier, triple);
        // The vantage standpoint annotation (annotation side-table).
        b.push_annotation(reifier, according_to, t1);
        // The direct, already-materialized sharpens edge (quads table): T1 ⊑ T2.
        b.push_quad(t1, sharpens, t2, None);
        b.freeze().expect("freeze")
    }

    /// Evaluate `heldIn(arg0, arg1)` over `ds` — with the fixture's
    /// standpoint predicate table configured — and return the EBV
    /// (`None` ⇒ SPARQL error / unbound).
    fn held_in(ds: &RdfDataset, arg0: Expression, arg1: Expression) -> Option<bool> {
        let expr = Expression::FunctionCall(held_in_fn(), vec![arg0, arg1]);
        let mut ctx = EvalCtx::new(ds).with_standpoint_predicates(ex_standpoints());
        let schema = VarSchema::new();
        eval_ebv(&expr, &[], &schema, &mut ctx).expect("eval")
    }

    #[test]
    fn held_in_without_a_configured_table_is_a_hard_eval_error() {
        // No StandpointPredicates configured ⇒ hard error (no fabricated default),
        // even before the arguments are inspected.
        let ds = held_in_ds();
        let expr =
            Expression::FunctionCall(held_in_fn(), vec![iri("http://ex/r"), iri("http://ex/T1")]);
        let mut ctx = EvalCtx::new(&ds);
        let schema = VarSchema::new();
        let err = eval_ebv(&expr, &[], &schema, &mut ctx)
            .expect_err("heldIn without a predicate table must hard-error");
        assert!(
            err.to_string()
                .contains("requires a standpoint predicate configuration"),
            "got: {err}"
        );
    }

    #[test]
    fn held_in_true_for_equal_standpoint() {
        let ds = held_in_ds();
        assert_eq!(
            held_in(&ds, iri("http://ex/r"), iri("http://ex/T1")),
            Some(true),
            "held directly in its own vantage standpoint"
        );
    }

    #[test]
    fn held_in_true_via_direct_sharpens_edge() {
        let ds = held_in_ds();
        // T1 sharpens T2, so a claim held in T1 counts as held in the broader T2.
        assert_eq!(
            held_in(&ds, iri("http://ex/r"), iri("http://ex/T2")),
            Some(true),
            "held in a standpoint that sharpens the queried one"
        );
    }

    #[test]
    fn held_in_false_for_unrelated_standpoint() {
        let ds = held_in_ds();
        assert_eq!(
            held_in(&ds, iri("http://ex/r"), iri("http://ex/T3")),
            Some(false),
            "not held in an unrelated standpoint"
        );
    }

    #[test]
    fn held_in_false_for_absent_standpoint() {
        let ds = held_in_ds();
        // A standpoint term not in the dataset is a clean negative, not an error.
        assert_eq!(
            held_in(&ds, iri("http://ex/r"), iri("http://ex/absent")),
            Some(false),
        );
    }

    #[test]
    fn held_in_none_for_unbound_arg() {
        let ds = held_in_ds();
        // An unbound variable argument is a SPARQL error (None), three-valued.
        let unbound = Expression::Variable(Variable::new("nope"));
        assert_eq!(
            held_in(&ds, unbound, iri("http://ex/T1")),
            None,
            "unbound argument ⇒ SPARQL error (None)"
        );
    }

    /// Determinism smoke test: a chained `BIND` — `BIND(?o + 5 AS ?sum)`
    /// then `BIND(CONCAT("v-", STR(?sum)) AS ?label)` over three rows — mints both
    /// a NUMERIC (`?sum`) and a STRING (`?label`) `Computed` term per row, each of
    /// which must escape a forked child via [`crate::parallel::portable_row`]/
    /// [`crate::parallel::reintern_portable_row`]. Evaluated once with the
    /// parallel `Extend` path FORCED and once with the sequential path FORCED,
    /// the two must produce byte-identical rows (row order + resolved values).
    #[test]
    fn bind_chain_numeric_and_string_forced_parallel_and_sequential_agree() {
        use purrdf_core::RdfLiteral;
        use purrdf_sparql_algebra::{NamedNodePattern, TermPattern, TriplePattern};

        const XINT: &str = "http://www.w3.org/2001/XMLSchema#integer";

        let mut b = RdfDatasetBuilder::new();
        let val = b.intern_iri("http://ex/val");
        for (s, n) in [("a", "10"), ("b", "20"), ("c", "30")] {
            let subj = b.intern_iri(&format!("http://ex/{s}"));
            let v = b.intern_literal(RdfLiteral {
                lexical_form: n.to_owned(),
                datatype: Some(XINT.to_owned()),
                language: None,
                direction: None,
            });
            b.push_quad(subj, val, v, None);
        }
        let ds = b.freeze().expect("freeze");

        let vp = |n: &str| TermPattern::Variable(Variable::new(n));
        let pred = |iri: &str| NamedNodePattern::NamedNode(NamedNode::new_unchecked(iri));
        let scan = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: vp("s"),
                predicate: pred("http://ex/val"),
                object: vp("o"),
            }],
        };
        let bind_sum = GraphPattern::Extend {
            inner: Box::new(scan),
            variable: Variable::new("sum"),
            expression: Expression::Add(
                Box::new(Expression::Variable(Variable::new("o"))),
                Box::new(typed_lit("5", XINT)),
            ),
        };
        let bind_label = GraphPattern::Extend {
            inner: Box::new(bind_sum),
            variable: Variable::new("label"),
            expression: Expression::FunctionCall(
                Function::Concat,
                vec![
                    lit("v-"),
                    Expression::FunctionCall(
                        Function::Str,
                        vec![Expression::Variable(Variable::new("sum"))],
                    ),
                ],
            ),
        };

        let run = |forced: bool| {
            let _guard = crate::parallel::force_parallel_for_test(forced);
            let mut ctx = EvalCtx::new(&ds);
            let seq = eval(&bind_label, &mut ctx).expect("eval");
            let schema = seq.schema.vars().to_vec();
            let label_col = seq.schema.index_of(&Variable::new("label")).unwrap();
            let labels: Vec<String> = seq
                .rows
                .iter()
                .map(
                    |row| match ctx.scratch.value_of(&ds, row[label_col].unwrap()) {
                        TermValue::Literal { lexical_form, .. } => lexical_form,
                        other => format!("{other:?}"),
                    },
                )
                .collect();
            (schema, seq.rows, labels)
        };

        let (schema_par, rows_par, labels_par) = run(true);
        let (schema_seq, rows_seq, labels_seq) = run(false);

        assert_eq!(
            schema_par, schema_seq,
            "schema must match regardless of path"
        );
        assert_eq!(
            rows_par, rows_seq,
            "parallel and sequential BIND paths must produce byte-identical row order"
        );
        assert_eq!(labels_par, labels_seq);
        assert_eq!(labels_seq, vec!["v-15", "v-25", "v-35"]);
    }

    // ── Values Insertion and Project-boundary narrowing ──────

    fn ex_iri(local: &str) -> NamedNode {
        NamedNode::new_unchecked(format!("https://example.org/lateral-substitution#{local}"))
    }

    fn ex_pred(local: &str) -> purrdf_sparql_algebra::NamedNodePattern {
        purrdf_sparql_algebra::NamedNodePattern::NamedNode(ex_iri(local))
    }

    fn ex_vp(name: &str) -> purrdf_sparql_algebra::TermPattern {
        purrdf_sparql_algebra::TermPattern::Variable(Variable::new(name))
    }

    /// A `SubstitutionRow` binding a single variable `var` to the IRI
    /// `https://example.org/lateral-substitution#{local}`, present in BOTH
    /// forms (`expr` and `term`) exactly as `outer_bindings_for_substitution`
    /// would build it for an IRI-valued row cell.
    fn row_binding_iri(var: &str, local: &str) -> SubstitutionRow {
        SubstitutionRow {
            expr: vec![(Variable::new(var), Expression::NamedNode(ex_iri(local)))],
            term: vec![(
                Variable::new(var),
                purrdf_sparql_algebra::GroundTerm::NamedNode(ex_iri(local)),
            )],
        }
    }

    #[test]
    fn non_iri_quoted_triple_predicate_degrades_without_panic() {
        // The production parser/interner can never build this: `TermValue::Triple`
        // with a non-IRI predicate is exactly the RDF 1.2 positional-constraint
        // violation `RdfDatasetBuilder::freeze` rejects via `require_iri_predicate`
        // before any `TermId` naming such a term could exist (see
        // `ground_term_from_term_value`'s doc). But `DatasetView` is public and
        // unsealed, so a foreign implementation is not forced through that gate —
        // a malformed `TermValue` reaching this function from such a view must
        // degrade to `None`, not panic the engine. Constructed directly here
        // because there is no other way to reach this arm.
        let malformed = TermValue::Triple {
            s: Box::new(TermValue::Iri("https://example.org/s".to_owned())),
            // A literal predicate: never producible by the parser or by
            // PurRDF's own interner, only by a hand-built (or foreign-view)
            // `TermValue`.
            p: Box::new(TermValue::simple_literal("not-an-iri")),
            o: Box::new(TermValue::Iri("https://example.org/o".to_owned())),
        };

        assert_eq!(
            ground_term_from_term_value(&malformed),
            None,
            "a non-IRI quoted-triple predicate must degrade to an unrepresentable \
             binding, not panic"
        );
    }

    #[test]
    fn non_iri_quoted_triple_predicate_nested_in_object_degrades_without_panic() {
        // The same malformed predicate, one level down (inside the object of an
        // otherwise well-formed outer triple): the recursive `?` propagation must
        // carry the `None` all the way up rather than the outer call constructing
        // a triple around a term that could not be built.
        let inner_malformed = TermValue::Triple {
            s: Box::new(TermValue::Iri("https://example.org/s2".to_owned())),
            p: Box::new(TermValue::simple_literal("still-not-an-iri")),
            o: Box::new(TermValue::Iri("https://example.org/o2".to_owned())),
        };
        let outer = TermValue::Triple {
            s: Box::new(TermValue::Iri("https://example.org/s".to_owned())),
            p: Box::new(TermValue::Iri("https://example.org/p".to_owned())),
            o: Box::new(inner_malformed),
        };

        assert_eq!(ground_term_from_term_value(&outer), None);
    }

    #[test]
    fn substitute_joins_a_values_row_at_bgp_leaves() {
        // Values Insertion (SEP-0007's `Replace` mechanism): a `Bgp` leaf
        // whose variable intersects the row becomes `Join(leaf, Values{..})`
        // rather than a term-rewrite — the leaf's own triple pattern is
        // untouched, still carrying the ORIGINAL variable.
        use purrdf_sparql_algebra::{GroundTerm, TriplePattern};

        let bgp = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: ex_vp("s"),
                predicate: ex_pred("knows"),
                object: ex_vp("o"),
            }],
        };
        let row = row_binding_iri("s", "alice");

        let substituted = *substitute_pattern(&bgp, &row);
        let GraphPattern::Join { left, right } = substituted else {
            panic!("expected a Join, got {bgp:?} -> not a Join");
        };
        assert_eq!(
            *left, bgp,
            "the leaf itself must be untouched — Values Insertion joins a row \
             onto it rather than rewriting its terms"
        );
        let GraphPattern::Values {
            variables,
            bindings,
        } = *right
        else {
            panic!("expected the joined right operand to be a Values node");
        };
        assert_eq!(variables, vec![Variable::new("s")]);
        assert_eq!(
            bindings,
            vec![vec![Some(GroundTerm::NamedNode(ex_iri("alice")))]]
        );
    }

    #[test]
    fn minus_under_substitution_keeps_its_domain() {
        // The MINUS disjoint-domain fix: BOTH sides of a `Minus` keep
        // their shared variable as a real schema column (a `Join(leaf, Values)`
        // beneath each side) rather than losing it to a term-rewrite, so
        // §18.5's domain-disjointness test reads the truth. Tree-shape first,
        // then the end-to-end correctness this shape exists to fix.
        use purrdf_sparql_algebra::TriplePattern;

        let side = || GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: ex_vp("s"),
                predicate: ex_pred("q"),
                object: purrdf_sparql_algebra::TermPattern::NamedNode(ex_iri("c")),
            }],
        };
        let minus = GraphPattern::Minus {
            left: Box::new(side()),
            right: Box::new(side()),
        };
        let row = row_binding_iri("s", "a");

        let substituted = *substitute_pattern(&minus, &row);
        let GraphPattern::Minus { left, right } = substituted else {
            panic!("Minus wrapper preserved");
        };
        for (label, wrapped) in [("left", *left), ("right", *right)] {
            let GraphPattern::Join {
                left: leaf,
                right: values,
            } = wrapped
            else {
                panic!(
                    "{label}: expected a Join(leaf, Values), Minus-right is NOT excluded from substitution here"
                );
            };
            assert_eq!(*leaf, side(), "{label}: the leaf keeps its ?s column");
            assert!(
                matches!(*values, GraphPattern::Values { .. }),
                "{label}: the injected row is a Values join"
            );
        }

        // End-to-end: `LATERAL { ?s :q :c MINUS { ?s :q :c } }` for a left row
        // where `(?s, :q, :c)` holds must subtract itself to the EMPTY answer —
        // the pre-fix term-rewrite erased the shared `?s` column on both sides,
        // making §18.5 see disjoint domains and skip the subtraction (the
        // "MINUS disjoint-domain flip").
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("https://example.org/lateral-substitution#s");
        let q = b.intern_iri("https://example.org/lateral-substitution#q");
        let c = b.intern_iri("https://example.org/lateral-substitution#c");
        b.push_quad(s, q, c, None);
        let ds = b.freeze().expect("freeze");
        let mut ctx = EvalCtx::new(&ds);
        let outer = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: ex_vp("s"),
                predicate: ex_pred("q"),
                object: ex_vp("c"),
            }],
        };
        let lateral = GraphPattern::Lateral {
            left: Box::new(outer),
            right: Box::new(minus),
        };
        let seq = eval(&lateral, &mut ctx).expect("eval");
        assert!(
            seq.rows.is_empty(),
            "MINUS must remove the row it is subtracting from itself, not be \
             fooled into keeping it by a domain the substitution erased"
        );
    }

    #[test]
    fn substitute_pattern_stops_at_unprojected_subselect_vars() {
        // Project-boundary narrowing (the SEP-0006 flagship example): a
        // sub-`SELECT` that does NOT project the outer-bound variable must not
        // have it injected below the `Project` boundary, even though the
        // variable NAME is shared and the inner `Bgp` mentions it.
        use purrdf_sparql_algebra::TriplePattern;

        let inner_bgp = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: ex_vp("s"),
                predicate: ex_pred("label"),
                object: ex_vp("label"),
            }],
        };
        let subselect = GraphPattern::Project {
            inner: Box::new(inner_bgp.clone()),
            variables: vec![Variable::new("label")], // does NOT project ?s
        };
        let row = row_binding_iri("s", "x1");

        let substituted = *substitute_pattern(&subselect, &row);
        let GraphPattern::Project { inner, variables } = substituted else {
            panic!("Project wrapper preserved");
        };
        assert_eq!(variables, vec![Variable::new("label")]);
        assert_eq!(
            *inner, inner_bgp,
            "?s must not be injected past the Project boundary the sub-SELECT \
             does not carry it across — no Values join, no rewrite"
        );
    }

    #[test]
    fn property_function_arg_with_a_literal_binding_behaves_unchanged() {
        // The fusion contract: a property-function argument's
        // substitution stays IRI-only. A literal-valued row binding must leave
        // the argument as the bare variable — UNCHANGED from before Values
        // Insertion — so the relation still reads the value from the per-row
        // argument dispatch, never from a rewritten constant a joined `VALUES`
        // could not have supplied either.
        use purrdf_sparql_algebra::PropertyFunctionCall;

        let call = GraphPattern::PropertyFunction(PropertyFunctionCall {
            iri: ex_iri("split").as_str().to_owned(),
            subject_args: vec![ex_vp("w")],
            object_args: vec![ex_vp("parts")],
        });
        let literal = Literal::new_typed("hello world", NamedNode::new_unchecked(XSD_STRING));
        let row = SubstitutionRow {
            expr: vec![(Variable::new("w"), Expression::Literal(literal.clone()))],
            term: vec![(
                Variable::new("w"),
                purrdf_sparql_algebra::GroundTerm::Literal(literal),
            )],
        };

        let substituted = *substitute_pattern(&call, &row);
        let GraphPattern::PropertyFunction(c) = substituted else {
            panic!("PropertyFunction node preserved");
        };
        assert_eq!(
            c.subject_args[0],
            ex_vp("w"),
            "a literal binding must not rewrite the argument"
        );
    }

    // ── Widened EXISTS correlation detection ───────────────────────────

    /// Two subjects, each `:owns` exactly one node, and each owned node has
    /// exactly one `:child` — chosen so their IRIs sort `x1`'s child before
    /// `x2`'s. A GLOBAL (unconstrained) `ORDER BY ?c LIMIT 1` over every
    /// `(?x, ?c)` pair in the dataset picks exactly one of the two — `x1`'s —
    /// which is what the unwidened (expression-only) correlation test would
    /// evaluate the `EXISTS` inner against, once, for every outer row alike.
    fn lateral_limit_exists_ds() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let owns = b.intern_iri("https://example.org/lateral-existswiden#owns");
        let child = b.intern_iri("https://example.org/lateral-existswiden#child");
        let a = b.intern_iri("https://example.org/lateral-existswiden#a");
        let bb = b.intern_iri("https://example.org/lateral-existswiden#b");
        let x1 = b.intern_iri("https://example.org/lateral-existswiden#x1");
        let x2 = b.intern_iri("https://example.org/lateral-existswiden#x2");
        let c1 = b.intern_iri("https://example.org/lateral-existswiden#c1");
        let c2 = b.intern_iri("https://example.org/lateral-existswiden#c2");
        b.push_quad(a, owns, x1, None);
        b.push_quad(bb, owns, x2, None);
        b.push_quad(x1, child, c1, None);
        b.push_quad(x2, child, c2, None);
        b.freeze().expect("freeze")
    }

    #[test]
    fn exists_over_a_lateral_limit_correlates_through_a_triple_position() {
        // `?x` (outer-bound via `?s :owns ?x`) occurs ONLY in a triple position
        // inside the EXISTS inner — never in an expression/FILTER/BIND position —
        // so `pattern_expr_vars` alone (the pre-widening test) reports no
        // correlation at all and the memoized evaluate-once-and-probe path would
        // run. The `LATERAL`'s own left is the unit table (nothing precedes it,
        // per SEP-0006), so it contributes nothing about `?x`: the ONLY place
        // `?x` is bound per-row is the correlated `EXISTS` substitution this item
        // widens detection for. The `LATERAL` right's `ORDER BY ?c LIMIT 1`
        // sub-select is what makes the unwidened (fast) path actively WRONG
        // rather than merely slow: evaluated once with `?x` free, the LIMIT
        // picks a SINGLE global `(?x, ?c)` winner instead of one per `?x`.
        let ds = lateral_limit_exists_ds();
        let q = "PREFIX : <https://example.org/lateral-existswiden#> \
                 SELECT ?s WHERE { \
                   ?s :owns ?x \
                   FILTER EXISTS { \
                     LATERAL { SELECT ?x ?c WHERE { ?x :child ?c } ORDER BY ?c LIMIT 1 } \
                   } \
                 }";
        let rows = run_rows(&ds, q, true);
        assert_eq!(
            rows,
            vec![
                vec!["<https://example.org/lateral-existswiden#a>".to_owned()],
                vec!["<https://example.org/lateral-existswiden#b>".to_owned()],
            ],
            "both owners must satisfy EXISTS — each has a child, judged against \
             its OWN children, not a single dataset-wide LIMIT winner"
        );
        assert_eq!(
            rows,
            run_rows(&ds, q, false),
            "memo on/off must agree once correlation is correctly detected"
        );
    }

    // ── Row-sensitivity through an expression-embedded EXISTS ──────────────
    //
    // `is_row_sensitive` (the classifier the widened check above depends on)
    // recurses into a `Filter`'s `inner` pattern but, pre-fix, never looked at
    // the `Filter`'s own `expr` field — so a `LATERAL` reachable ONLY inside a
    // nested `EXISTS`/`NOT EXISTS` expression (rather than directly as a
    // pattern node, which the test above already covers) was invisible to it,
    // and the outer `EXISTS` wrongly judged row-insensitive.

    /// `s1`/`s2` both `:knows` a `:tag "ok"` object (so the OUTER `FILTER
    /// EXISTS`'s own `Filter.inner` — visible to `pattern_all_vars` without
    /// any fix — binds `?o` identically for both outer rows: any divergence
    /// this fixture exposes must come from the `?s`-correlated `LATERAL`
    /// buried inside the NESTED `EXISTS`, not from the `?o` column the probe
    /// can already see). Only `s1`'s `:hasItem` has `:flag "true"`; `s2`'s
    /// does not.
    fn nested_exists_lateral_ds() -> Arc<RdfDataset> {
        use purrdf_core::RdfLiteral;

        let mut b = RdfDatasetBuilder::new();
        let knows = b.intern_iri("https://example.org/188-nested-exists-lateral#knows");
        let tag = b.intern_iri("https://example.org/188-nested-exists-lateral#tag");
        let has_item = b.intern_iri("https://example.org/188-nested-exists-lateral#hasItem");
        let flag = b.intern_iri("https://example.org/188-nested-exists-lateral#flag");
        let s1 = b.intern_iri("https://example.org/188-nested-exists-lateral#s1");
        let s2 = b.intern_iri("https://example.org/188-nested-exists-lateral#s2");
        let o1 = b.intern_iri("https://example.org/188-nested-exists-lateral#o1");
        let o2 = b.intern_iri("https://example.org/188-nested-exists-lateral#o2");
        let v1 = b.intern_iri("https://example.org/188-nested-exists-lateral#v1");
        let v2 = b.intern_iri("https://example.org/188-nested-exists-lateral#v2");
        let ok = b.intern_literal(RdfLiteral::simple("ok"));
        let t = b.intern_literal(RdfLiteral::simple("true"));
        let f = b.intern_literal(RdfLiteral::simple("false"));
        b.push_quad(s1, knows, o1, None);
        b.push_quad(s2, knows, o2, None);
        b.push_quad(o1, tag, ok, None);
        b.push_quad(o2, tag, ok, None);
        b.push_quad(s1, has_item, v1, None);
        b.push_quad(s2, has_item, v2, None);
        b.push_quad(v1, flag, t, None);
        b.push_quad(v2, flag, f, None);
        b.freeze().expect("freeze")
    }

    #[test]
    fn exists_nested_in_a_filter_expression_with_a_lateral_inner_correlates() {
        // The OUTER `FILTER EXISTS { ?o :tag "ok" . FILTER EXISTS { ?s :hasItem ?v
        // LATERAL { ?v :flag "true" } } }` has its row-sensitive `LATERAL` reachable
        // ONLY through the nested `FILTER EXISTS`'s expression position — never as
        // a directly-nested pattern node under the outer `Filter`. Pre-fix,
        // `is_row_sensitive` on the outer `Filter` ignores its `expr` field
        // entirely, so it never notices the buried `LATERAL`; the outer `EXISTS`
        // then takes the memoized evaluate-once-and-probe path. Evaluated once
        // unconstrained, `?s` is free throughout, so the nested `EXISTS { LATERAL
        // { ?v :flag "true" } }` collapses to one GLOBAL boolean (true, because
        // SOME subject — `s1` — has a `true`-flagged item) instead of one answer
        // PER outer `?s`, and the top-level probe has no shared column for `?s` to
        // filter on (the outer `Filter.inner`'s own output is keyed only by `?o`,
        // which is identical for both outer rows) — so the wrong, constant answer
        // passes both `s1` and `s2`. The correct per-row answer keeps only `s1`.
        let ds = nested_exists_lateral_ds();
        let q = "PREFIX : <https://example.org/188-nested-exists-lateral#> \
                 SELECT ?s WHERE { \
                   ?s :knows ?o \
                   FILTER EXISTS { \
                     ?o :tag \"ok\" \
                     FILTER EXISTS { \
                       ?s :hasItem ?v LATERAL { ?v :flag \"true\" } \
                     } \
                   } \
                 }";
        let rows = run_rows(&ds, q, true);
        assert_eq!(
            rows,
            vec![vec![
                "<https://example.org/188-nested-exists-lateral#s1>".to_owned()
            ]],
            "only s1 (whose :hasItem is :flag \"true\") may satisfy the outer EXISTS; \
             the pre-fix classifier let s2 through on a constant, ?s-independent answer"
        );
        assert_eq!(
            rows,
            run_rows(&ds, q, false),
            "memo on/off must agree once correlation is correctly detected"
        );
    }
}

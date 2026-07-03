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
//! **`TIMEZONE`/`TZ`**, **`MD5`/`SHA1`/`SHA256`/`SHA384`/`SHA512`**,
//! **`RAND`**, and **`UUID`/`STRUUID`**. Still deferred (`Unsupported`):
//! `SERVICE` (S6b #928), property paths (S8 #914), and `Function::Custom`.

use std::cmp::Ordering;
use std::sync::Arc;

use purrdf_core::{BlankScope, DatasetView, GraphMatch, TermRef, TermValue};
use purrdf_sparql_algebra::{Expression, Function, GraphPattern, PurrdfFn, Variable};
use purrdf_xsd::{
    effective_boolean_value, numeric_abs, numeric_add, numeric_ceil, numeric_div, numeric_floor,
    numeric_mul, numeric_round, numeric_sub, numeric_unary_minus, numeric_unary_plus,
    parse_by_iri, parse_xsd10, value_cmp, XsdDatatype, XsdValue,
};
use sha2::Digest; // brings the Digest trait in scope for all RustCrypto hash calls

use crate::error::EvalError;
use crate::eval::{eval, EvalCtx};
use crate::scratch::SolutionTerm;
use crate::solution::{SolutionSeq, VarSchema};
use crate::DetHashSet;

const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";

/// Evaluate an expression over a solution. See the [module docs](self) for the
/// `Ok(Some)` / `Ok(None)` / `Err` contract.
pub(crate) fn eval_expr(
    expr: &Expression,
    row: &[Option<SolutionTerm>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_>,
) -> Result<Option<SolutionTerm>, EvalError> {
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
        // SPARQL three-valued contract: type errors (non-numeric operands,
        // overflow, divide-by-zero) → Ok(None), NOT Err. A hard EvalError would
        // propagate out of FILTER and break the query; Ok(None) just drops the row.
        Expression::Add(a, b) => binary_numeric(a, b, row, schema, ctx, numeric_add),
        Expression::Subtract(a, b) => binary_numeric(a, b, row, schema, ctx, numeric_sub),
        Expression::Multiply(a, b) => binary_numeric(a, b, row, schema, ctx, numeric_mul),
        Expression::Divide(a, b) => binary_numeric(a, b, row, schema, ctx, numeric_div),
        Expression::UnaryPlus(a) => unary_numeric(a, row, schema, ctx, numeric_unary_plus),
        Expression::UnaryMinus(a) => unary_numeric(a, row, schema, ctx, numeric_unary_minus),

        // ---- functions -----------------------------------------------------
        Expression::FunctionCall(function, args) => eval_function(function, args, row, schema, ctx),
    }
}

/// Evaluate `expr` and reduce it to an effective boolean value (`Ok(None)` =
/// error/unbound).
pub(crate) fn eval_ebv(
    expr: &Expression,
    row: &[Option<SolutionTerm>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_>,
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
pub(crate) fn eval_filter(
    expr: &Expression,
    inner: &GraphPattern,
    ctx: &mut EvalCtx<'_>,
) -> Result<SolutionSeq, EvalError> {
    let seq = eval(inner, ctx)?;
    let schema = seq.schema.clone();
    let rows = if crate::parallel::is_parallel_safe(expr) {
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
    Ok(SolutionSeq { schema, rows })
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
pub(crate) fn eval_extend(
    inner: &GraphPattern,
    var: &Variable,
    expr: &Expression,
    ctx: &mut EvalCtx<'_>,
) -> Result<SolutionSeq, EvalError> {
    let seq = eval(inner, ctx)?;
    let mut schema = (*seq.schema).clone();
    let col = schema.push(var.clone());
    let width = schema.len();
    let schema = Arc::new(schema);

    let rows = if crate::parallel::is_parallel_safe(expr) {
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
        for mut row in seq.rows {
            row.resize(width, None);
            let value = eval_expr(expr, &row, &schema, ctx)?;
            row[col] = value;
            rows.push(row);
        }
        rows
    };
    Ok(SolutionSeq { schema, rows })
}

// ---------------------------------------------------------------------------
// internals
// ---------------------------------------------------------------------------

/// Look up a variable's binding in a solution.
fn lookup(
    var: &Variable,
    row: &[Option<SolutionTerm>],
    schema: &VarSchema,
) -> Option<SolutionTerm> {
    schema.index_of(var).and_then(|c| row[c])
}

/// Intern a value to a solution term (promoting to an existing dataset id).
fn intern(ctx: &mut EvalCtx<'_>, value: TermValue) -> SolutionTerm {
    ctx.scratch.intern(ctx.dataset, value)
}

/// Intern a constant atom (`NamedNode`/`Literal`), memoized per query by the
/// node's AST address (see [`EvalCtx::const_atom_cache`]). `build` — which owns
/// the `TermValue` allocation — runs only on a cache miss, so a FILTER/BIND over
/// N rows pays the `to_owned()` + intern probe once, not N times.
fn const_atom(
    ctx: &mut EvalCtx<'_>,
    expr: &Expression,
    build: impl FnOnce() -> TermValue,
) -> SolutionTerm {
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
fn value_of(ctx: &EvalCtx<'_>, term: SolutionTerm) -> TermValue {
    ctx.scratch.value_of(ctx.dataset, term)
}

/// Intern an `xsd:boolean` literal.
///
/// The two boolean terms are resolved **once per [`EvalCtx`]** (lazily) and then
/// served from `cached_bool_terms`: a FILTER over N rows pays the value-hash
/// intern probe once, not N times. The cache is exact — interning is
/// deterministic for the context's pinned dataset and dedup-by-value scratch, so
/// the cached term is the same `SolutionTerm` a fresh intern would produce.
fn bool_term(ctx: &mut EvalCtx<'_>, b: bool) -> SolutionTerm {
    let slot = usize::from(b);
    if let Some(term) = ctx.cached_bool_terms[slot] {
        return term;
    }
    let term = intern(ctx, typed(if b { "true" } else { "false" }, XSD_BOOLEAN));
    ctx.cached_bool_terms[slot] = Some(term);
    term
}

/// Intern an `xsd:string` literal.
fn string_term(ctx: &mut EvalCtx<'_>, lexical: &str) -> SolutionTerm {
    intern(ctx, typed(lexical, XSD_STRING))
}

/// Intern an `xsd:integer` literal.
fn integer_term(ctx: &mut EvalCtx<'_>, value: i64) -> SolutionTerm {
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
fn ebv_of(
    expr: &Expression,
    row: &[Option<SolutionTerm>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_>,
) -> Result<Option<bool>, EvalError> {
    match eval_expr(expr, row, schema, ctx)? {
        Some(term) => Ok(ebv_term(ctx, term)),
        None => Ok(None),
    }
}

/// The effective boolean value of a concrete term (`None` = type error).
fn ebv_term(ctx: &mut EvalCtx<'_>, term: SolutionTerm) -> Option<bool> {
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
fn xsd_of_term(ctx: &mut EvalCtx<'_>, term: SolutionTerm) -> Option<XsdValue> {
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
                    other => unreachable!("literal datatype must be an IRI, got {other:?}"),
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
fn term_is_literal(ctx: &EvalCtx<'_>, term: SolutionTerm) -> bool {
    match term {
        SolutionTerm::Existing(id) => {
            matches!(ctx.dataset.resolve(id), TermRef::Literal { .. })
        }
        SolutionTerm::Computed(sid) => {
            matches!(ctx.scratch.computed_value(sid), TermValue::Literal { .. })
        }
    }
}

/// Evaluate a comparison: both operands to values, compare in the XSD value space,
/// and test the resulting [`Ordering`] with `keep`. `None` (error/unbound operand
/// or incomparable values) propagates.
fn compare(
    a: &Expression,
    b: &Expression,
    row: &[Option<SolutionTerm>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_>,
    keep: impl Fn(Ordering) -> bool,
) -> Result<Option<SolutionTerm>, EvalError> {
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

/// Evaluate `a = b` under SPARQL RDFterm-equality (SPARQL §17.4.1.7 / `RDFterm-equal`):
/// both operands resolve to a term, identical terms are equal, value-comparable
/// literals compare in the XSD value space, distinct terms where at least one is a
/// non-literal (IRI/blank) are **unequal** (`false`, NOT a type error), and two
/// incomparable literals are a type error (`None`). This is the equality companion to
/// the ordering [`compare`]; using `compare` for `=` would wrongly turn a distinct
/// IRI pair into an error.
fn equal(
    a: &Expression,
    b: &Expression,
    row: &[Option<SolutionTerm>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_>,
) -> Result<Option<SolutionTerm>, EvalError> {
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
    // value-space comparison and the literal/non-literal split remain — evaluated
    // here on borrowed views (no owned TermValue clones), semantically identical
    // to `rdf_equal(&value_of(ctx, ta), &value_of(ctx, tb))`.
    let ax = xsd_of_term(ctx, ta);
    let bx = xsd_of_term(ctx, tb);
    let eq = match (ax, bx) {
        (Some(ax), Some(bx)) => value_cmp(&ax, &bx).map(|o| o == Ordering::Equal),
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
fn eval_in(
    needle: &Expression,
    haystack: &[Expression],
    row: &[Option<SolutionTerm>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_>,
) -> Result<Option<SolutionTerm>, EvalError> {
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
    match (xsd_of(a), xsd_of(b)) {
        (Some(ax), Some(bx)) => value_cmp(&ax, &bx).map(|o| o == Ordering::Equal),
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
                match agg {
                    purrdf_sparql_algebra::AggregateExpression::CountStar { .. } => {}
                    purrdf_sparql_algebra::AggregateExpression::FunctionCall {
                        expression, ..
                    } => expr_vars(expression, out),
                }
            }
            pattern_expr_vars(inner, out);
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
fn exists(
    pattern: &GraphPattern,
    row: &[Option<SolutionTerm>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_>,
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

    let is_expression_correlated = inner_expr_vars.iter().any(|v| outer_bound.contains(v));

    if is_expression_correlated {
        // Correct per-row path: substitute the outer row's bound variable values
        // into the inner pattern's expression positions before evaluating.
        //
        // This implements the W3C SPARQL §18.6 EXISTS substitution semantics:
        // every reference to an outer-bound variable inside an expression (FILTER
        // condition, BIND, aggregate, etc.) is replaced with the corresponding
        // constant, so the inner evaluation sees the outer bindings as ground
        // constants rather than unbound variables that would error in expressions.
        //
        // After substitution, the resulting pattern is a valid (all-ground-expression)
        // pattern whose result is specific to this outer row; it is NOT memoized.
        let bindings = outer_bindings_for_substitution(row, schema, ctx);
        let substituted = substitute_pattern(pattern, &bindings);
        // `substituted` is a fresh heap allocation, specific to this outer row,
        // that is dropped at the end of this call — its node addresses do NOT
        // outlive the query. Flag the window so address-keyed memoization
        // (`const_atom`, `exists_expr_vars_cache`, `exists_inner_cache`) is
        // bypassed while it is evaluated, and restore the prior value afterward
        // (even on error) so a doubly-nested correlated EXISTS is handled
        // correctly.
        let prev_in_substituted_exists = ctx.in_substituted_exists;
        ctx.in_substituted_exists = true;
        let inner = eval(&substituted, ctx);
        ctx.in_substituted_exists = prev_in_substituted_exists;
        let inner = inner?;
        Ok(!inner.is_empty())
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
                let inner = Arc::new(eval(pattern, ctx)?);
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

/// Substitute outer-bound variables into expression positions within a graph pattern.
///
/// This implements the W3C SPARQL EXISTS substitution semantics (§18.6): for each
/// variable `v` in `bindings`, every `Expression::Variable(v)` occurrence inside
/// the pattern's expression positions (FILTER conditions, BIND expressions, etc.)
/// is replaced with the corresponding constant expression. Triple-pattern term
/// positions are also substituted when the bound value is representable as a
/// `NamedNode` (blank nodes and triple terms cannot appear as triple-pattern
/// constants in a `Bgp`, so those positions are left as variables — the later
/// seed-join in the uncorrelated path handles them instead).
fn substitute_pattern(pattern: &GraphPattern, bindings: &[(Variable, Expression)]) -> GraphPattern {
    match pattern {
        GraphPattern::Bgp { patterns } => GraphPattern::Bgp {
            patterns: patterns
                .iter()
                .map(|tp| substitute_triple_pattern(tp, bindings))
                .collect(),
        },
        GraphPattern::Filter { expr, inner } => GraphPattern::Filter {
            expr: substitute_expr(expr, bindings),
            inner: Box::new(substitute_pattern(inner, bindings)),
        },
        GraphPattern::Extend {
            inner,
            variable,
            expression,
        } => GraphPattern::Extend {
            inner: Box::new(substitute_pattern(inner, bindings)),
            variable: variable.clone(),
            expression: substitute_expr(expression, bindings),
        },
        GraphPattern::Join { left, right } => GraphPattern::Join {
            left: Box::new(substitute_pattern(left, bindings)),
            right: Box::new(substitute_pattern(right, bindings)),
        },
        GraphPattern::Union { left, right } => GraphPattern::Union {
            left: Box::new(substitute_pattern(left, bindings)),
            right: Box::new(substitute_pattern(right, bindings)),
        },
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => GraphPattern::LeftJoin {
            left: Box::new(substitute_pattern(left, bindings)),
            right: Box::new(substitute_pattern(right, bindings)),
            expression: expression.as_ref().map(|e| substitute_expr(e, bindings)),
        },
        GraphPattern::Minus { left, right } => GraphPattern::Minus {
            left: Box::new(substitute_pattern(left, bindings)),
            right: Box::new(substitute_pattern(right, bindings)),
        },
        GraphPattern::Lateral { left, right } => GraphPattern::Lateral {
            left: Box::new(substitute_pattern(left, bindings)),
            right: Box::new(substitute_pattern(right, bindings)),
        },
        GraphPattern::Graph { name, inner } => GraphPattern::Graph {
            name: name.clone(),
            inner: Box::new(substitute_pattern(inner, bindings)),
        },
        GraphPattern::Service {
            name,
            inner,
            silent,
        } => GraphPattern::Service {
            name: name.clone(),
            inner: Box::new(substitute_pattern(inner, bindings)),
            silent: *silent,
        },
        GraphPattern::OrderBy { inner, expression } => GraphPattern::OrderBy {
            inner: Box::new(substitute_pattern(inner, bindings)),
            expression: expression
                .iter()
                .map(|oe| match oe {
                    purrdf_sparql_algebra::OrderExpression::Asc(e) => {
                        purrdf_sparql_algebra::OrderExpression::Asc(substitute_expr(e, bindings))
                    }
                    purrdf_sparql_algebra::OrderExpression::Desc(e) => {
                        purrdf_sparql_algebra::OrderExpression::Desc(substitute_expr(e, bindings))
                    }
                })
                .collect(),
        },
        GraphPattern::Group {
            inner,
            variables,
            aggregates,
        } => GraphPattern::Group {
            inner: Box::new(substitute_pattern(inner, bindings)),
            variables: variables.clone(),
            aggregates: aggregates
                .iter()
                .map(|(v, agg)| {
                    let new_agg = match agg {
                        purrdf_sparql_algebra::AggregateExpression::CountStar { distinct } => {
                            purrdf_sparql_algebra::AggregateExpression::CountStar {
                                distinct: *distinct,
                            }
                        }
                        purrdf_sparql_algebra::AggregateExpression::FunctionCall {
                            function,
                            expression,
                            distinct,
                        } => purrdf_sparql_algebra::AggregateExpression::FunctionCall {
                            function: function.clone(),
                            expression: Box::new(substitute_expr(expression, bindings)),
                            distinct: *distinct,
                        },
                    };
                    (v.clone(), new_agg)
                })
                .collect(),
        },
        // Leaf patterns that need no substitution.
        GraphPattern::Distinct { inner } => GraphPattern::Distinct {
            inner: Box::new(substitute_pattern(inner, bindings)),
        },
        GraphPattern::Reduced { inner } => GraphPattern::Reduced {
            inner: Box::new(substitute_pattern(inner, bindings)),
        },
        GraphPattern::Slice {
            inner,
            start,
            length,
        } => GraphPattern::Slice {
            inner: Box::new(substitute_pattern(inner, bindings)),
            start: *start,
            length: *length,
        },
        GraphPattern::Project { inner, variables } => GraphPattern::Project {
            inner: Box::new(substitute_pattern(inner, bindings)),
            variables: variables.clone(),
        },
        GraphPattern::Path { .. } | GraphPattern::Values { .. } => pattern.clone(),
    }
}

/// Substitute outer-bound variables into a triple pattern's term positions.
/// Only IRI-valued bindings can replace a variable in triple-pattern subject/object
/// position (blank nodes and literals are not valid there in the algebra). Predicate
/// positions are `NamedNodePattern` which cannot be a free variable — left unchanged.
fn substitute_triple_pattern(
    tp: &purrdf_sparql_algebra::TriplePattern,
    bindings: &[(Variable, Expression)],
) -> purrdf_sparql_algebra::TriplePattern {
    use purrdf_sparql_algebra::{TermPattern, TriplePattern};

    let subst_term = |term: &TermPattern| -> TermPattern {
        if let TermPattern::Variable(v) = term {
            for (bv, expr) in bindings {
                if bv == v {
                    if let Expression::NamedNode(n) = expr {
                        return TermPattern::NamedNode(n.clone());
                    }
                    // Literal or other: leave as variable (the expr substitution
                    // in FILTER will handle value comparison).
                }
            }
        }
        term.clone()
    };

    // Predicate is NamedNodePattern — it can be a variable but rarely is in practice;
    // leave as-is (IRI substitution there is uncommon and the Filter handles equality).
    TriplePattern {
        subject: subst_term(&tp.subject),
        predicate: tp.predicate.clone(),
        object: subst_term(&tp.object),
    }
}

/// Substitute outer-bound variables in expression positions by replacing
/// `Expression::Variable(v)` with the corresponding constant expression.
fn substitute_expr(expr: &Expression, bindings: &[(Variable, Expression)]) -> Expression {
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
            // BOUND(?v) where ?v is outer-bound → always true (the variable IS bound).
            for (bv, _) in bindings {
                if bv == v {
                    return Expression::Literal(purrdf_sparql_algebra::Literal::new_typed(
                        "true",
                        purrdf_sparql_algebra::NamedNode::new_unchecked(XSD_BOOLEAN),
                    ));
                }
            }
            expr.clone()
        }
        Expression::NamedNode(_) | Expression::Literal(_) => expr.clone(),
        Expression::Or(a, b) => Expression::Or(
            Box::new(substitute_expr(a, bindings)),
            Box::new(substitute_expr(b, bindings)),
        ),
        Expression::And(a, b) => Expression::And(
            Box::new(substitute_expr(a, bindings)),
            Box::new(substitute_expr(b, bindings)),
        ),
        Expression::Equal(a, b) => Expression::Equal(
            Box::new(substitute_expr(a, bindings)),
            Box::new(substitute_expr(b, bindings)),
        ),
        Expression::SameTerm(a, b) => Expression::SameTerm(
            Box::new(substitute_expr(a, bindings)),
            Box::new(substitute_expr(b, bindings)),
        ),
        Expression::Greater(a, b) => Expression::Greater(
            Box::new(substitute_expr(a, bindings)),
            Box::new(substitute_expr(b, bindings)),
        ),
        Expression::GreaterOrEqual(a, b) => Expression::GreaterOrEqual(
            Box::new(substitute_expr(a, bindings)),
            Box::new(substitute_expr(b, bindings)),
        ),
        Expression::Less(a, b) => Expression::Less(
            Box::new(substitute_expr(a, bindings)),
            Box::new(substitute_expr(b, bindings)),
        ),
        Expression::LessOrEqual(a, b) => Expression::LessOrEqual(
            Box::new(substitute_expr(a, bindings)),
            Box::new(substitute_expr(b, bindings)),
        ),
        Expression::Add(a, b) => Expression::Add(
            Box::new(substitute_expr(a, bindings)),
            Box::new(substitute_expr(b, bindings)),
        ),
        Expression::Subtract(a, b) => Expression::Subtract(
            Box::new(substitute_expr(a, bindings)),
            Box::new(substitute_expr(b, bindings)),
        ),
        Expression::Multiply(a, b) => Expression::Multiply(
            Box::new(substitute_expr(a, bindings)),
            Box::new(substitute_expr(b, bindings)),
        ),
        Expression::Divide(a, b) => Expression::Divide(
            Box::new(substitute_expr(a, bindings)),
            Box::new(substitute_expr(b, bindings)),
        ),
        Expression::UnaryPlus(a) => Expression::UnaryPlus(Box::new(substitute_expr(a, bindings))),
        Expression::UnaryMinus(a) => Expression::UnaryMinus(Box::new(substitute_expr(a, bindings))),
        Expression::Not(a) => Expression::Not(Box::new(substitute_expr(a, bindings))),
        Expression::If(c, t, e) => Expression::If(
            Box::new(substitute_expr(c, bindings)),
            Box::new(substitute_expr(t, bindings)),
            Box::new(substitute_expr(e, bindings)),
        ),
        Expression::In(needle, haystack) => Expression::In(
            Box::new(substitute_expr(needle, bindings)),
            haystack
                .iter()
                .map(|h| substitute_expr(h, bindings))
                .collect(),
        ),
        Expression::Coalesce(items) => {
            Expression::Coalesce(items.iter().map(|i| substitute_expr(i, bindings)).collect())
        }
        Expression::FunctionCall(f, args) => Expression::FunctionCall(
            f.clone(),
            args.iter().map(|a| substitute_expr(a, bindings)).collect(),
        ),
        Expression::Exists(inner_pat) => {
            Expression::Exists(Box::new(substitute_pattern(inner_pat, bindings)))
        }
    }
}

/// Build the binding list for substitution from the outer row's bound variables,
/// materializing each `SolutionTerm` to a constant `Expression`.
fn outer_bindings_for_substitution(
    row: &[Option<SolutionTerm>],
    schema: &VarSchema,
    ctx: &EvalCtx<'_>,
) -> Vec<(Variable, Expression)> {
    use purrdf_core::TermValue;
    use purrdf_sparql_algebra::{Literal, NamedNode};

    let mut bindings = Vec::new();
    for (i, var) in schema.vars().iter().enumerate() {
        if let Some(term) = row[i] {
            let value = ctx.scratch.value_of(ctx.dataset, term);
            let expr: Option<Expression> = match value {
                TermValue::Iri(iri) => Some(Expression::NamedNode(NamedNode::new_unchecked(iri))),
                TermValue::Literal {
                    lexical_form,
                    datatype,
                    language,
                    ..
                } => {
                    let lit = if let Some(lang) = language {
                        Literal::new_lang(lexical_form, lang, None)
                    } else {
                        Literal::new_typed(lexical_form, NamedNode::new_unchecked(datatype))
                    };
                    Some(Expression::Literal(lit))
                }
                // Blank nodes and triple terms: leave the variable unbound in
                // expression positions (uncommon in practice; the seed-join
                // handles them in triple-pattern positions).
                TermValue::Blank { .. } | TermValue::Triple { .. } => None,
            };
            if let Some(e) = expr {
                bindings.push((var.clone(), e));
            }
        }
    }
    bindings
}

/// Dispatch a built-in (or custom) function call.
fn eval_function(
    function: &Function,
    args: &[Expression],
    row: &[Option<SolutionTerm>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_>,
) -> Result<Option<SolutionTerm>, EvalError> {
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
                Ok(Some(intern(ctx, TermValue::Iri(lexical_form.clone()))))
            }
            _ => Ok(None),
        },
        Function::StrLang => eval_str_lang(ctx, &vals),
        Function::StrDt => eval_str_dt(ctx, &vals),
        Function::BNode => {
            // BNODE() / BNODE(str): mint a fresh blank node per call.
            ctx.bnode_counter += 1;
            let label = format!("bnode{}", ctx.bnode_counter);
            Ok(Some(intern(
                ctx,
                TermValue::Blank {
                    label,
                    scope: BlankScope::DEFAULT,
                },
            )))
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
            if let Some(target) = XsdDatatype::from_iri(iri.as_str()) {
                return Ok(eval_xsd_cast(ctx, target, arg(&vals, 0)));
            }
            Err(EvalError::unsupported(format!(
                "custom SPARQL function <{}>",
                iri.as_str()
            )))
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
/// (and string/boolean/temporal targets); when it fails, a numeric source is cast by
/// VALUE through [`cast_numeric_value`].
fn eval_xsd_cast(
    ctx: &mut EvalCtx<'_>,
    target: XsdDatatype,
    source: Option<&TermValue>,
) -> Option<SolutionTerm> {
    let source = source?;
    let lexical = match source {
        TermValue::Literal { lexical_form, .. } => lexical_form.clone(),
        TermValue::Iri(iri) if target == XsdDatatype::String => iri.clone(),
        _ => return None,
    };
    // The operand-mapping rules pin XSD 1.0, so a `xsd:float`/`xsd:double` constructor
    // rejects the XSD 1.1-only `+INF` spelling (only `INF`); other targets are
    // unaffected (`parse_xsd10` delegates to `parse`).
    if let Ok(value) = parse_xsd10(&lexical, target) {
        return Some(xsd_to_term(ctx, &value));
    }
    // The lexical is not directly valid for `target`. If both source and target are
    // numeric, convert by value (e.g. a `double`/`float` scientific-notation lexical
    // into the equivalent `decimal`/`integer`), matching the spec's casting tower.
    let value = cast_numeric_value(&xsd_of(source)?, target)?;
    Some(xsd_to_term(ctx, &value))
}

/// Cast a numeric [`XsdValue`] to a numeric `target` datatype **by value** (the
/// SPARQL §17.1 numeric casting rules): the source's numeric value is re-expressed in
/// the target's value space. Returns `None` when the source is non-numeric, the target
/// is non-numeric, or the value is out of the target's range (e.g. a non-integral
/// double cast to integer truncates toward zero, as XPath `xs:integer` mandates).
fn cast_numeric_value(source: &XsdValue, target: XsdDatatype) -> Option<XsdValue> {
    use purrdf_xsd::parse as xsd_parse;
    // The source's exact numeric value, as the widest faithful form available.
    let as_f64 = match source {
        XsdValue::Integer { value, .. } => *value as f64,
        XsdValue::Decimal(d) => d.to_f64(),
        XsdValue::Float(f) => f64::from(*f),
        XsdValue::Double(d) => *d,
        _ => return None,
    };
    match target {
        XsdDatatype::Double => Some(XsdValue::Double(as_f64)),
        XsdDatatype::Float => Some(XsdValue::Float(as_f64 as f32)),
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
/// Per CONSTITUTION Principle 17 the native logic solver is the sole reasoning
/// authority: this does NOT walk/compute the `sharpens` transitive closure —
/// it relies on the closure being materialized upstream as direct edges. It returns
/// true iff some vantage standpoint `T` of the reifier (the objects of the reifier's
/// `accordingTo` annotations) either equals the queried standpoint or has a
/// direct `(T, sharpens, standpoint)` quad (T is more specific than the
/// queried standpoint, so a claim held in T counts as held in the broader one).
///
/// Three-valued: an unbound argument yields `Ok(None)` (a SPARQL error). An argument
/// absent from the dataset is a well-formed negative answer — `Ok(Some(false))`, not
/// `None`. Missing `accordingTo`/`sharpens` interning simply yields no
/// matches (→ false), which is correct.
fn eval_held_in(
    vals: &[Option<TermValue>],
    ctx: &mut EvalCtx<'_>,
) -> Result<Option<SolutionTerm>, EvalError> {
    // The predicate table is mandatory configuration — fail loudly BEFORE looking at
    // the arguments, so a misconfigured deployment cannot get a quietly-wrong answer.
    let Some(predicates) = ctx.standpoint_predicates.clone() else {
        return Err(EvalError::unsupported(
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
            .annotations_of(reifier_id)
            .filter(|(pred, _)| *pred == atid)
            .map(|(_, vantage)| vantage)
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

fn eval_string_pred_expr(
    args: &[Expression],
    row: &[Option<SolutionTerm>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_>,
    f: impl Fn(&str, &str) -> bool,
) -> Result<Option<SolutionTerm>, EvalError> {
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

fn eval_regex_expr(
    args: &[Expression],
    row: &[Option<SolutionTerm>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_>,
) -> Result<Option<SolutionTerm>, EvalError> {
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

fn eval_lang_matches_expr(
    args: &[Expression],
    row: &[Option<SolutionTerm>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_>,
) -> Result<Option<SolutionTerm>, EvalError> {
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

fn eval_string_arg_expr(
    expr: Option<&Expression>,
    row: &[Option<SolutionTerm>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_>,
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
            Ok(string_arg_value(&value))
        }
    }
}

fn eval_str_lexical_expr(
    expr: &Expression,
    row: &[Option<SolutionTerm>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_>,
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

fn eval_lang_lexical_expr(
    expr: &Expression,
    row: &[Option<SolutionTerm>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_>,
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

fn str_lexical_term(ctx: &EvalCtx<'_>, term: SolutionTerm) -> Option<String> {
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

fn lang_lexical_term(ctx: &EvalCtx<'_>, term: SolutionTerm) -> Option<String> {
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
    string_arg_value(arg(vals, i)?)
}

fn string_arg_value(value: &TermValue) -> Option<(String, Option<String>)> {
    match value {
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            ..
        } if datatype == XSD_STRING || datatype == RDF_LANG_STRING => {
            Some((lexical_form.clone(), language.clone()))
        }
        _ => None,
    }
}

/// Apply a pure string transform to a single string argument, preserving its
/// language tag.
fn map_string(
    ctx: &mut EvalCtx<'_>,
    vals: &[Option<TermValue>],
    f: impl Fn(&str) -> String,
) -> Result<Option<SolutionTerm>, EvalError> {
    match string_arg(vals, 0) {
        Some((s, lang)) => Ok(Some(make_string(ctx, f(&s), lang))),
        None => Ok(None),
    }
}

/// A two-string boolean predicate (CONTAINS/STRSTARTS/STRENDS).
fn string_pred(
    ctx: &mut EvalCtx<'_>,
    vals: &[Option<TermValue>],
    f: impl Fn(&str, &str) -> bool,
) -> Result<Option<SolutionTerm>, EvalError> {
    match (string_arg(vals, 0), string_arg(vals, 1)) {
        (Some((h, _)), Some((n, _))) => Ok(Some(bool_term(ctx, f(&h, &n)))),
        _ => Ok(None),
    }
}

/// Intern a string literal, as `rdf:langString@lang` if a language is present, else
/// `xsd:string`.
fn make_string(ctx: &mut EvalCtx<'_>, lexical: String, lang: Option<String>) -> SolutionTerm {
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

/// `CONCAT(...)`: concatenate string arguments. The result keeps a common language
/// tag iff every argument shares it; otherwise it is `xsd:string`.
fn eval_concat(
    ctx: &mut EvalCtx<'_>,
    vals: &[Option<TermValue>],
) -> Result<Option<SolutionTerm>, EvalError> {
    let mut out = String::new();
    let mut common: Option<Option<String>> = None;
    for i in 0..vals.len() {
        let Some((s, lang)) = string_arg(vals, i) else {
            return Ok(None);
        };
        out.push_str(&s);
        common = Some(match common {
            None => lang,
            Some(prev) if prev == lang => prev,
            Some(_) => None,
        });
    }
    let lang = common.flatten();
    Ok(Some(make_string(ctx, out, lang)))
}

/// `SUBSTR(str, start[, length])` with 1-based indexing over Unicode scalars.
fn eval_substr(
    ctx: &mut EvalCtx<'_>,
    vals: &[Option<TermValue>],
) -> Result<Option<SolutionTerm>, EvalError> {
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

/// `STRBEFORE`/`STRAFTER(haystack, needle)`.
fn eval_str_before_after(
    ctx: &mut EvalCtx<'_>,
    vals: &[Option<TermValue>],
    before: bool,
) -> Result<Option<SolutionTerm>, EvalError> {
    let (Some((h, lang)), Some((n, _))) = (string_arg(vals, 0), string_arg(vals, 1)) else {
        return Ok(None);
    };
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
fn eval_replace(
    ctx: &mut EvalCtx<'_>,
    vals: &[Option<TermValue>],
) -> Result<Option<SolutionTerm>, EvalError> {
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
fn eval_regex(
    ctx: &mut EvalCtx<'_>,
    vals: &[Option<TermValue>],
) -> Result<Option<SolutionTerm>, EvalError> {
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
fn cached_regex(ctx: &mut EvalCtx<'_>, pattern: &str, flags: &str) -> Option<Arc<regex::Regex>> {
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
fn eval_lang_matches(
    ctx: &mut EvalCtx<'_>,
    vals: &[Option<TermValue>],
) -> Result<Option<SolutionTerm>, EvalError> {
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
fn eval_str_lang(
    ctx: &mut EvalCtx<'_>,
    vals: &[Option<TermValue>],
) -> Result<Option<SolutionTerm>, EvalError> {
    let (Some((lex, _)), Some((lang, _))) = (string_arg(vals, 0), string_arg(vals, 1)) else {
        return Ok(None);
    };
    Ok(Some(make_string(ctx, lex, Some(lang.to_ascii_lowercase()))))
}

/// `STRDT(lexical, datatypeIri)`.
fn eval_str_dt(
    ctx: &mut EvalCtx<'_>,
    vals: &[Option<TermValue>],
) -> Result<Option<SolutionTerm>, EvalError> {
    let Some((lex, _)) = string_arg(vals, 0) else {
        return Ok(None);
    };
    let Some(TermValue::Iri(dt)) = arg(vals, 1) else {
        return Ok(None);
    };
    Ok(Some(intern(ctx, typed(&lex, dt))))
}

/// `TRIPLE(s, p, o)` — RDF 1.2 triple-term constructor.
fn eval_triple_ctor(
    ctx: &mut EvalCtx<'_>,
    vals: &[Option<TermValue>],
) -> Result<Option<SolutionTerm>, EvalError> {
    let (Some(s), Some(p), Some(o)) = (arg(vals, 0), arg(vals, 1), arg(vals, 2)) else {
        return Ok(None);
    };
    let triple = TermValue::Triple {
        s: Box::new(s.clone()),
        p: Box::new(p.clone()),
        o: Box::new(o.clone()),
    };
    Ok(Some(intern(ctx, triple)))
}

/// Extract a component of a triple term (`SUBJECT`/`PREDICATE`/`OBJECT`).
fn triple_part(
    ctx: &mut EvalCtx<'_>,
    vals: &[Option<TermValue>],
    pick: impl Fn(TermValue, TermValue, TermValue) -> TermValue,
) -> Result<Option<SolutionTerm>, EvalError> {
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
pub(crate) fn xsd_to_term(ctx: &mut EvalCtx<'_>, v: &XsdValue) -> SolutionTerm {
    intern(ctx, typed(&v.canonical_lexical(), v.datatype().iri()))
}

/// Evaluate a binary numeric expression: resolve both operands to [`XsdValue`], call
/// `op`, and return `Ok(Some(term))` on success or `Ok(None)` on any error (type
/// error, overflow, divide-by-zero — all SPARQL expression errors).
fn binary_numeric(
    a: &Expression,
    b: &Expression,
    row: &[Option<SolutionTerm>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_>,
    op: impl Fn(&XsdValue, &XsdValue) -> Result<XsdValue, purrdf_xsd::XsdError>,
) -> Result<Option<SolutionTerm>, EvalError> {
    let (Some(ta), Some(tb)) = (
        eval_expr(a, row, schema, ctx)?,
        eval_expr(b, row, schema, ctx)?,
    ) else {
        return Ok(None);
    };
    let (va, vb) = (value_of(ctx, ta), value_of(ctx, tb));
    let (Some(xa), Some(xb)) = (xsd_of(&va), xsd_of(&vb)) else {
        return Ok(None); // non-numeric operand → SPARQL type error
    };
    match op(&xa, &xb) {
        Ok(result) => Ok(Some(xsd_to_term(ctx, &result))),
        Err(_) => Ok(None), // overflow / div-by-zero / type-mismatch → expression error
    }
}

/// Evaluate a unary numeric expression (`+` / `-`): resolve the operand, call `op`,
/// return `Ok(None)` on any error.
fn unary_numeric(
    a: &Expression,
    row: &[Option<SolutionTerm>],
    schema: &VarSchema,
    ctx: &mut EvalCtx<'_>,
    op: impl Fn(&XsdValue) -> Result<XsdValue, purrdf_xsd::XsdError>,
) -> Result<Option<SolutionTerm>, EvalError> {
    let Some(ta) = eval_expr(a, row, schema, ctx)? else {
        return Ok(None);
    };
    let va = value_of(ctx, ta);
    let Some(xa) = xsd_of(&va) else {
        return Ok(None);
    };
    match op(&xa) {
        Ok(result) => Ok(Some(xsd_to_term(ctx, &result))),
        Err(_) => Ok(None),
    }
}

/// Apply a unary numeric function from the `vals` pre-evaluated argument list.
/// Argument 0 must be a numeric literal; type errors → `Ok(None)`.
fn unary_numeric_fn(
    ctx: &mut EvalCtx<'_>,
    vals: &[Option<TermValue>],
    op: impl Fn(&XsdValue) -> Result<XsdValue, purrdf_xsd::XsdError>,
) -> Result<Option<SolutionTerm>, EvalError> {
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
fn next_u64(ctx: &mut EvalCtx<'_>) -> u64 {
    ctx.rng_state = ctx.rng_state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = ctx.rng_state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
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

/// Mint a version-4 UUID from the PRNG state and return it as a
/// lowercase-hyphenated `8-4-4-4-12` string (without any `urn:uuid:` prefix).
fn make_uuid(ctx: &mut EvalCtx<'_>) -> (String, [u8; 16]) {
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
        let entries = |ctx: &EvalCtx<'_>| {
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
        // CEIL(2.1) = 3 (as xsd:decimal)
        let expr = Expression::FunctionCall(Function::Ceil, vec![typed_lit("2.1", XDEC)]);
        assert_eq!(lex(&ds, &expr), Some("3.0".to_owned()));
    }

    #[test]
    fn function_floor() {
        let ds = empty_ds();
        // FLOOR(2.9) = 2 (as xsd:decimal)
        let expr = Expression::FunctionCall(Function::Floor, vec![typed_lit("2.9", XDEC)]);
        assert_eq!(lex(&ds, &expr), Some("2.0".to_owned()));
    }

    #[test]
    fn function_round() {
        let ds = empty_ds();
        // ROUND(2.5) = 3 (round-half-toward-+infinity per XPath fn:round)
        let expr = Expression::FunctionCall(Function::Round, vec![typed_lit("2.5", XDEC)]);
        assert_eq!(lex(&ds, &expr), Some("3.0".to_owned()));
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
        // SECONDS returns xsd:decimal; canonical form of integer-valued decimal is "45.0"
        assert_eq!(lex(&ds, &seconds), Some("45.0".to_owned()));
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
            assert_eq!(lex(&ds, &cast("INF")).as_deref(), Some("INF"), "INF casts for <{dt}>");
            assert_eq!(lex(&ds, &cast("+INF")), None, "+INF is a cast error for <{dt}>");
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
        use crate::eval::evaluate_query;
        use crate::eval::Outcome;
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
        // builtin, so (Task 5) `eval_filter` routes it through
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
        // `exists_memo_populates_cache_once`'s comment: Task 5 routes this
        // parallel-safe FILTER through a forked child context, so the memo would
        // land there, not on a `ctx` inspected from outside `evaluate_query`.
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

    /// Determinism smoke test (Task 6): a chained `BIND` — `BIND(?o + 5 AS ?sum)`
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
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The PurRDF `rdf:List` SPARQL extension functions.
//!
//! These bind the FnO list primitives — `listLength`, `listGet`, `listIndexOf`,
//! `listSlice`, `listConcat`, `listContains` — to executable SPARQL extension
//! functions, so a query can spell `ext:listLength(?list)` under whatever extension-function
//! namespace the caller configures. They are
//! recognized at parse time as members of the closed
//! [`PurrdfFn`](purrdf_sparql_algebra::PurrdfFn) registry and dispatched from the
//! `Function::Purrdf` arm of [`crate::expr`].
//!
//! Two shapes:
//!
//! * **Scalar readers** (`listLength`/`listGet`/`listIndexOf`/`listContains`) walk
//!   the `rdf:first`/`rdf:rest` chain in the dataset and return a single term. These
//!   mirror the reasoning-layer recursion (conformance case
//!   `goal-rdf-list-functions`) — parity is the contract.
//! * **Constructors** (`listSlice`/`listConcat`) invent a fresh `rdf:List`. Because
//!   a SPARQL expression returns one term, the new cells are emitted into the
//!   per-query constructed-quads buffer on [`EvalCtx`] and surface at the result
//!   boundary (CONSTRUCT output and the SELECT auxiliary graph). See
//!   [`materialize_list`].
//!
//! The walk is cycle-guarded: a cyclic or torn `rdf:List` is malformed input and
//! hard-fails ([`EvalError::Data`]) rather than looping forever.

use purrdf_core::{BlankScope, TermId, TermValue};
use purrdf_sparql_algebra::PurrdfFn;
use purrdf_xsd::XsdValue;

use crate::DetHashSet;
use crate::error::EvalError;
use crate::eval::EvalCtx;
use crate::expr::xsd_of;
use crate::scratch::{SolutionTerm, term_id_to_value};

const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";

/// Evaluate a PurRDF `rdf:List` extension function.
///
/// The parser has already resolved the call to a [`PurrdfFn`] variant, so this is a
/// total dispatch over the six list functions. The result follows the usual
/// expression contract: `Ok(Some)` is a value, `Ok(None)` is a SPARQL error/unbound,
/// and `Err` is a hard failure. The non-list [`PurrdfFn::HeldIn`] is handled by its
/// own arm in [`crate::expr`] and never reaches here (defensively an internal error).
pub(crate) fn dispatch(
    func: PurrdfFn,
    vals: &[Option<TermValue>],
    ctx: &mut EvalCtx<'_>,
) -> Result<Option<SolutionTerm>, EvalError> {
    match func {
        PurrdfFn::ListLength => list_length(ctx, vals),
        PurrdfFn::ListGet => list_get(ctx, vals),
        PurrdfFn::ListIndexOf => list_index_of(ctx, vals),
        PurrdfFn::ListContains => list_contains(ctx, vals),
        PurrdfFn::ListSlice => list_slice(ctx, vals),
        PurrdfFn::ListConcat => list_concat(ctx, vals),
        PurrdfFn::HeldIn => Err(EvalError::internal("heldIn is not an rdf:List function")),
    }
}

/// `listLength(list)` → the number of members, as `xsd:integer`. A non-list
/// argument yields a SPARQL error (`Ok(None)`).
fn list_length(
    ctx: &mut EvalCtx<'_>,
    vals: &[Option<TermValue>],
) -> Result<Option<SolutionTerm>, EvalError> {
    let Some(head) = arg(vals, 0) else {
        return Ok(None);
    };
    match walk(ctx, head)? {
        Some(members) => Ok(Some(integer_term(ctx, members.len() as i64))),
        None => Ok(None),
    }
}

/// `listGet(list, index)` → the zero-based member, or a SPARQL error when the
/// index is out of range / not an integer.
fn list_get(
    ctx: &mut EvalCtx<'_>,
    vals: &[Option<TermValue>],
) -> Result<Option<SolutionTerm>, EvalError> {
    let (Some(head), Some(index)) = (arg(vals, 0), arg(vals, 1)) else {
        return Ok(None);
    };
    let Some(idx) = as_index(index) else {
        return Ok(None);
    };
    let Some(members) = walk(ctx, head)? else {
        return Ok(None);
    };
    if idx < 0 {
        return Ok(None);
    }
    match members.into_iter().nth(idx as usize) {
        Some(value) => Ok(Some(intern(ctx, value))),
        None => Ok(None),
    }
}

/// `listIndexOf(list, value)` → the zero-based index of the first occurrence,
/// or a SPARQL error when the value is absent.
fn list_index_of(
    ctx: &mut EvalCtx<'_>,
    vals: &[Option<TermValue>],
) -> Result<Option<SolutionTerm>, EvalError> {
    let (Some(head), Some(value)) = (arg(vals, 0), arg(vals, 1)) else {
        return Ok(None);
    };
    let Some(members) = walk(ctx, head)? else {
        return Ok(None);
    };
    match members.iter().position(|m| m == value) {
        Some(pos) => Ok(Some(integer_term(ctx, pos as i64))),
        None => Ok(None),
    }
}

/// `listContains(list, value)` → `xsd:boolean`. A non-list argument yields a
/// SPARQL error (`Ok(None)`); membership over a valid (possibly empty) list is total.
fn list_contains(
    ctx: &mut EvalCtx<'_>,
    vals: &[Option<TermValue>],
) -> Result<Option<SolutionTerm>, EvalError> {
    let (Some(head), Some(value)) = (arg(vals, 0), arg(vals, 1)) else {
        return Ok(None);
    };
    let Some(members) = walk(ctx, head)? else {
        return Ok(None);
    };
    Ok(Some(bool_term(ctx, members.iter().any(|m| m == value))))
}

/// `listSlice(list, start, end)` → a fresh `rdf:List` of the members in the
/// half-open index range `[start, end)`. Indices are clamped to the list bounds
/// (negatives to 0), so an out-of-range or inverted range yields `rdf:nil`. The new
/// cells are buffered on [`EvalCtx`] and surface at the result boundary (see
/// [`materialize_list`]). A non-list / non-integer argument yields a SPARQL error.
fn list_slice(
    ctx: &mut EvalCtx<'_>,
    vals: &[Option<TermValue>],
) -> Result<Option<SolutionTerm>, EvalError> {
    let (Some(head), Some(start), Some(end)) = (arg(vals, 0), arg(vals, 1), arg(vals, 2)) else {
        return Ok(None);
    };
    let (Some(start), Some(end)) = (as_index(start), as_index(end)) else {
        return Ok(None);
    };
    let Some(members) = walk(ctx, head)? else {
        return Ok(None);
    };
    let len = members.len() as i64;
    let lo = start.clamp(0, len);
    let hi = end.clamp(lo, len); // also enforces hi >= lo → inverted ranges are empty
    let slice: Vec<TermValue> = members[lo as usize..hi as usize].to_vec();
    let value = materialize_list(ctx, slice);
    Ok(Some(intern(ctx, value)))
}

/// `listConcat(listA, listB)` → a fresh `rdf:List` of A's members followed by
/// B's. The new cells are buffered on [`EvalCtx`] and surface at the result boundary
/// (see [`materialize_list`]). A non-list argument yields a SPARQL error.
fn list_concat(
    ctx: &mut EvalCtx<'_>,
    vals: &[Option<TermValue>],
) -> Result<Option<SolutionTerm>, EvalError> {
    let (Some(a), Some(b)) = (arg(vals, 0), arg(vals, 1)) else {
        return Ok(None);
    };
    let (Some(mut left), Some(right)) = (walk(ctx, a)?, walk(ctx, b)?) else {
        return Ok(None);
    };
    left.extend(right);
    let value = materialize_list(ctx, left);
    Ok(Some(intern(ctx, value)))
}

/// Invent a fresh `rdf:List` carrying `members` in order, returning its head term.
///
/// Each cell is a fresh blank node (minted from the shared `bnode_counter`, so
/// labels never collide with CONSTRUCT-template or `BNODE()` blanks). The cell quads
/// `cell rdf:first member` / `cell rdf:rest next` are pushed onto
/// [`EvalCtx::constructed`] to surface at the result boundary; the empty list is
/// simply `rdf:nil` (no cells).
fn materialize_list(ctx: &mut EvalCtx<'_>, members: Vec<TermValue>) -> TermValue {
    if members.is_empty() {
        return iri(RDF_NIL);
    }
    let n = members.len();
    let cells: Vec<TermValue> = (0..n)
        .map(|_| {
            ctx.bnode_counter += 1;
            TermValue::Blank {
                label: format!("lc{}", ctx.bnode_counter),
                scope: BlankScope::DEFAULT,
            }
        })
        .collect();
    for (i, member) in members.into_iter().enumerate() {
        let rest = if i + 1 < n {
            cells[i + 1].clone()
        } else {
            iri(RDF_NIL)
        };
        ctx.constructed
            .push((cells[i].clone(), iri(RDF_FIRST), member));
        ctx.constructed
            .push((cells[i].clone(), iri(RDF_REST), rest));
    }
    cells[0].clone()
}

// ---------------------------------------------------------------------------
// internals
// ---------------------------------------------------------------------------

/// The argument value at index `i`, if bound (not unbound/error).
fn arg(vals: &[Option<TermValue>], i: usize) -> Option<&TermValue> {
    vals.get(i).and_then(|v| v.as_ref())
}

/// Extract a zero-based index from an `xsd:integer`-derived literal.
fn as_index(value: &TermValue) -> Option<i64> {
    match xsd_of(value)? {
        XsdValue::Integer { value, .. } => i64::try_from(value).ok(),
        _ => None,
    }
}

/// Intern a value to a solution term (promoting to an existing dataset id).
fn intern(ctx: &mut EvalCtx<'_>, value: TermValue) -> SolutionTerm {
    ctx.scratch.intern(ctx.dataset, value)
}

/// Intern an `xsd:integer` literal.
fn integer_term(ctx: &mut EvalCtx<'_>, value: i64) -> SolutionTerm {
    intern(ctx, typed(&value.to_string(), XSD_INTEGER))
}

/// Intern an `xsd:boolean` literal.
fn bool_term(ctx: &mut EvalCtx<'_>, b: bool) -> SolutionTerm {
    intern(ctx, typed(if b { "true" } else { "false" }, XSD_BOOLEAN))
}

const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";

/// Build a typed (no-language) literal value.
fn typed(lexical: &str, datatype: &str) -> TermValue {
    TermValue::Literal {
        lexical_form: lexical.to_owned(),
        datatype: datatype.to_owned(),
        language: None,
        direction: None,
    }
}

/// Walk an `rdf:List` from `head`, returning its member values in order.
///
/// Reads from the active dataset first, then from the per-query constructed buffer,
/// so a list freshly minted by `listSlice`/`listConcat` is readable by another list
/// function within the same query (e.g. `g:listLength(g:listSlice(?l, 1, 3))`).
///
/// * `Ok(Some(members))` — a well-formed list (an empty list, i.e. `rdf:nil`, gives
///   `[]`).
/// * `Ok(None)` — `head` is not a list node we can read: it is `rdf:nil`-free, not
///   interned in the active dataset nor minted in this query, or has no `rdf:first`
///   (a SPARQL error — the function yields unbound).
/// * `Err(EvalError::Data)` — a cyclic, torn, or multi-edge list (a cell revisited,
///   an interior cell missing `rdf:first`/`rdf:rest`, or a cell carrying two of
///   either edge): malformed input, a hard fail.
fn walk(ctx: &EvalCtx<'_>, head: &TermValue) -> Result<Option<Vec<TermValue>>, EvalError> {
    // The empty list is `rdf:nil`, whether or not it happens to be interned.
    if is_nil(head) {
        return Ok(Some(Vec::new()));
    }
    if let Some(members) = walk_dataset(ctx, head)? {
        return Ok(Some(members));
    }
    // Not a dataset list — it may be a list minted earlier in THIS query, whose cells
    // live only in the per-query constructed buffer (value-constructing functions used
    // nested, e.g. `g:listLength(g:listConcat(?a, ?b))`).
    match walk_constructed(ctx, head) {
        Some(result) => result.map(Some),
        None => Ok(None),
    }
}

/// Walk a list interned in the active dataset (the common case).
fn walk_dataset(ctx: &EvalCtx<'_>, head: &TermValue) -> Result<Option<Vec<TermValue>>, EvalError> {
    // The list nodes and edges must exist in the active dataset to be walkable.
    let (Some(first_id), Some(rest_id), Some(nil_id)) = (
        ctx.dataset.term_id_by_value(&iri(RDF_FIRST)),
        ctx.dataset.term_id_by_value(&iri(RDF_REST)),
        ctx.dataset.term_id_by_value(&iri(RDF_NIL)),
    ) else {
        // No list vocabulary in the dataset at all — `head` is not a readable list.
        return Ok(None);
    };
    let Some(head_id) = ctx.dataset.term_id_by_value(head) else {
        return Ok(None);
    };

    let scope = ctx.active_dataset.scope_for(ctx.active_graph);
    let mut members: Vec<TermValue> = Vec::new();
    let mut seen: DetHashSet<TermId> = DetHashSet::default();
    let mut cur = head_id;
    loop {
        if cur == nil_id {
            return Ok(Some(members));
        }
        if !seen.insert(cur) {
            return Err(EvalError::data(
                "cyclic rdf:List (a cell is reachable from itself)",
            ));
        }

        // A well-formed cell has exactly one `rdf:first` and one `rdf:rest`. Two of
        // either makes the cell ambiguous: take no arbitrary, iteration-order-dependent
        // branch — hard-fail (the malformed-input contract).
        let mut first_obj: Option<TermId> = None;
        let mut first_count = 0usize;
        scope.for_each_quad(ctx.dataset, Some(cur), Some(first_id), None, |q| {
            first_obj = Some(q.o);
            first_count += 1;
        });
        if first_count > 1 {
            return Err(EvalError::data(
                "rdf:List cell with multiple rdf:first edges",
            ));
        }
        let Some(fo) = first_obj else {
            // No `rdf:first`: the head is simply not a list (SPARQL error); an
            // interior cell without `rdf:first` is a torn list (hard fail).
            if members.is_empty() {
                return Ok(None);
            }
            return Err(EvalError::data("rdf:List cell missing rdf:first"));
        };
        members.push(term_id_to_value(ctx.dataset, fo));

        let mut rest_obj: Option<TermId> = None;
        let mut rest_count = 0usize;
        scope.for_each_quad(ctx.dataset, Some(cur), Some(rest_id), None, |q| {
            rest_obj = Some(q.o);
            rest_count += 1;
        });
        if rest_count > 1 {
            return Err(EvalError::data(
                "rdf:List cell with multiple rdf:rest edges",
            ));
        }
        let Some(ro) = rest_obj else {
            return Err(EvalError::data("rdf:List cell missing rdf:rest"));
        };
        cur = ro;
    }
}

/// Walk a list whose cells live only in the per-query constructed buffer
/// (`ctx.constructed`) — a list minted by `listSlice`/`listConcat` and read again
/// within the same query. Returns `None` when `head` is not a constructed-list head
/// (the caller then treats it as a non-list / unbound); otherwise the same
/// well-formed / malformed contract as [`walk_dataset`].
fn walk_constructed(
    ctx: &EvalCtx<'_>,
    head: &TermValue,
) -> Option<Result<Vec<TermValue>, EvalError>> {
    let first = iri(RDF_FIRST);
    let rest = iri(RDF_REST);
    let mut members: Vec<TermValue> = Vec::new();
    let mut cur = head.clone();
    let mut at_head = true;
    loop {
        if is_nil(&cur) {
            return Some(Ok(members));
        }
        // Our own mints are finite and acyclic; more members than buffered cells means
        // a cycle (defensive — bounds the walk without needing `Hash` on `TermValue`).
        if members.len() > ctx.constructed.len() {
            return Some(Err(EvalError::data(
                "cyclic rdf:List (a cell is reachable from itself)",
            )));
        }
        let fo = match single_constructed_edge(ctx, &cur, &first) {
            Err(e) => return Some(Err(e)),
            Ok(Some(o)) => o,
            Ok(None) => {
                // No `rdf:first`: at the head, `head` is simply not a constructed list
                // (fall through to unbound); mid-list it is a torn cell (hard fail).
                if at_head {
                    return None;
                }
                return Some(Err(EvalError::data("rdf:List cell missing rdf:first")));
            }
        };
        members.push(fo);
        let ro = match single_constructed_edge(ctx, &cur, &rest) {
            Err(e) => return Some(Err(e)),
            Ok(Some(o)) => o,
            Ok(None) => return Some(Err(EvalError::data("rdf:List cell missing rdf:rest"))),
        };
        cur = ro;
        at_head = false;
    }
}

/// The unique object of `(subject, predicate, ?)` in the per-query constructed
/// buffer: `Ok(None)` if absent, `Ok(Some)` if exactly one, `Err` if more than one
/// (a multi-edge malformed cell — same hard-fail as the dataset walk).
fn single_constructed_edge(
    ctx: &EvalCtx<'_>,
    subject: &TermValue,
    predicate: &TermValue,
) -> Result<Option<TermValue>, EvalError> {
    let mut found: Option<&TermValue> = None;
    let mut count = 0usize;
    for (s, p, o) in &ctx.constructed {
        if s == subject && p == predicate {
            found = Some(o);
            count += 1;
        }
    }
    if count > 1 {
        return Err(EvalError::data("rdf:List cell with multiple edges"));
    }
    Ok(found.cloned())
}

/// An IRI value.
fn iri(s: &str) -> TermValue {
    TermValue::Iri(s.to_owned())
}

/// Whether a term is `rdf:nil`.
fn is_nil(value: &TermValue) -> bool {
    matches!(value, TermValue::Iri(i) if i == RDF_NIL)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use purrdf_core::{RdfDataset, RdfDatasetBuilder, TermValue};

    use crate::error::EvalError;
    use crate::eval::{EvalCtx, Outcome, evaluate_query};

    /// The three-element list `(x y z)` rooted at `ex:l0`, plus an anchor triple
    /// `ex:q ex:list ex:l0` so a BGP can bind the head.
    fn list_ds() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let first = b.intern_iri(super::RDF_FIRST);
        let rest = b.intern_iri(super::RDF_REST);
        let nil = b.intern_iri(super::RDF_NIL);
        let l0 = b.intern_iri("http://ex/l0");
        let l1 = b.intern_iri("http://ex/l1");
        let l2 = b.intern_iri("http://ex/l2");
        let x = b.intern_iri("http://ex/x");
        let y = b.intern_iri("http://ex/y");
        let z = b.intern_iri("http://ex/z");
        let q = b.intern_iri("http://ex/q");
        let list = b.intern_iri("http://ex/list");
        b.push_quad(l0, first, x, None);
        b.push_quad(l0, rest, l1, None);
        b.push_quad(l1, first, y, None);
        b.push_quad(l1, rest, l2, None);
        b.push_quad(l2, first, z, None);
        b.push_quad(l2, rest, nil, None);
        b.push_quad(q, list, l0, None);
        b.freeze().expect("freeze")
    }

    /// The caller-configured extension-function namespace these tests parse with
    /// (the `g:` prefix in `PREFIX` below binds to the same namespace).
    fn ext_options() -> purrdf_sparql_algebra::ParserOptions {
        purrdf_sparql_algebra::ParserOptions {
            extension_fn_namespaces: vec!["https://example.org/ext/".to_owned()],
        }
    }

    /// Run `query` and return sorted stringified rows (a multiset comparison).
    fn rows(ds: &RdfDataset, query: &str) -> Vec<Vec<String>> {
        use purrdf_sparql_algebra::SparqlParser;
        let parsed = SparqlParser::new()
            .parse_query_with(query, &ext_options())
            .expect("parse");
        let mut ctx = EvalCtx::new(ds);
        match evaluate_query(&parsed, &mut ctx).expect("eval") {
            Outcome::Solutions(seq) => {
                let mut out: Vec<Vec<String>> = seq
                    .rows
                    .iter()
                    .map(|row| {
                        row.iter()
                            .map(|c| match c {
                                None => "UNBOUND".to_owned(),
                                Some(t) => match ctx.scratch.value_of(ctx.dataset, *t) {
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

    /// Evaluate a query expected to hard-fail, returning the error.
    fn eval_err(ds: &RdfDataset, query: &str) -> EvalError {
        use purrdf_sparql_algebra::SparqlParser;
        let parsed = SparqlParser::new()
            .parse_query_with(query, &ext_options())
            .expect("parse");
        let mut ctx = EvalCtx::new(ds);
        evaluate_query(&parsed, &mut ctx).expect_err("expected a hard failure")
    }

    const PREFIX: &str = "PREFIX g: <https://example.org/ext/> ";

    #[test]
    fn list_length_counts_members() {
        let ds = list_ds();
        let q = format!(
            "{PREFIX} SELECT ?n WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listLength(?l) AS ?n) }}"
        );
        assert_eq!(rows(&ds, &q), vec![vec!["3".to_owned()]]);
    }

    #[test]
    fn list_length_of_nil_is_zero() {
        let ds = list_ds();
        let q = format!(
            "{PREFIX} SELECT ?n WHERE {{ \
             BIND(g:listLength(<http://www.w3.org/1999/02/22-rdf-syntax-ns#nil>) AS ?n) }}"
        );
        assert_eq!(rows(&ds, &q), vec![vec!["0".to_owned()]]);
    }

    #[test]
    fn list_get_returns_indexed_member() {
        let ds = list_ds();
        let q = format!(
            "{PREFIX} SELECT ?x WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listGet(?l, 1) AS ?x) }}"
        );
        assert_eq!(rows(&ds, &q), vec![vec!["<http://ex/y>".to_owned()]]);
    }

    #[test]
    fn list_get_out_of_range_is_unbound() {
        let ds = list_ds();
        let q = format!(
            "{PREFIX} SELECT ?x WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listGet(?l, 5) AS ?x) }}"
        );
        assert_eq!(rows(&ds, &q), vec![vec!["UNBOUND".to_owned()]]);
    }

    #[test]
    fn list_index_of_finds_value() {
        let ds = list_ds();
        let q = format!(
            "{PREFIX} SELECT ?n WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listIndexOf(?l, <http://ex/z>) AS ?n) }}"
        );
        assert_eq!(rows(&ds, &q), vec![vec!["2".to_owned()]]);
    }

    #[test]
    fn list_index_of_absent_is_unbound() {
        let ds = list_ds();
        let q = format!(
            "{PREFIX} SELECT ?n WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listIndexOf(?l, <http://ex/absent>) AS ?n) }}"
        );
        assert_eq!(rows(&ds, &q), vec![vec!["UNBOUND".to_owned()]]);
    }

    #[test]
    fn list_contains_true_and_false() {
        let ds = list_ds();
        let q_true = format!(
            "{PREFIX} SELECT ?b WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listContains(?l, <http://ex/y>) AS ?b) }}"
        );
        assert_eq!(rows(&ds, &q_true), vec![vec!["true".to_owned()]]);
        let q_false = format!(
            "{PREFIX} SELECT ?b WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listContains(?l, <http://ex/absent>) AS ?b) }}"
        );
        assert_eq!(rows(&ds, &q_false), vec![vec!["false".to_owned()]]);
    }

    #[test]
    fn unknown_extension_function_is_a_parse_error() {
        // The extension-function surface is a CLOSED registry: an unrecognized
        // IRI under a configured namespace in call position fails fast at parse
        // time and never reaches evaluation.
        use purrdf_sparql_algebra::SparqlParser;
        let q = format!("{PREFIX} SELECT ?x WHERE {{ BIND(g:notAListFunction(1) AS ?x) }}");
        let err = SparqlParser::new()
            .parse_query_with(&q, &ext_options())
            .expect_err("closed registry must reject an unknown extension function");
        assert!(
            err.to_string().contains("unknown extension function"),
            "got {err}"
        );
    }

    #[test]
    fn unknown_custom_function_still_hard_fails() {
        // A custom IRI outside the configured namespace parses to
        // `Function::Custom` and hard-fails at eval.
        let ds = list_ds();
        let q = "SELECT ?x WHERE { BIND(<http://other.example/notAFunction>(1) AS ?x) }";
        let err = eval_err(&ds, q);
        assert!(matches!(err, EvalError::Unsupported(_)), "got {err:?}");
    }

    #[test]
    fn cyclic_list_is_a_hard_data_error() {
        // l0 -> first x, rest l1 ; l1 -> first y, rest l0  (a cycle, no rdf:nil).
        let mut b = RdfDatasetBuilder::new();
        let first = b.intern_iri(super::RDF_FIRST);
        let rest = b.intern_iri(super::RDF_REST);
        let nil = b.intern_iri(super::RDF_NIL); // present so the walk starts
        let l0 = b.intern_iri("http://ex/l0");
        let l1 = b.intern_iri("http://ex/l1");
        let x = b.intern_iri("http://ex/x");
        let y = b.intern_iri("http://ex/y");
        let z = b.intern_iri("http://ex/z");
        b.push_quad(l0, first, x, None);
        b.push_quad(l0, rest, l1, None);
        b.push_quad(l1, first, y, None);
        b.push_quad(l1, rest, l0, None);
        // A well-formed terminator elsewhere so rdf:nil is interned.
        b.push_quad(z, rest, nil, None);
        let ds = b.freeze().expect("freeze");

        let q = format!("{PREFIX} SELECT ?n WHERE {{ BIND(g:listLength(<http://ex/l0>) AS ?n) }}");
        let err = eval_err(&ds, &q);
        assert!(matches!(err, EvalError::Data(_)), "got {err:?}");
        assert!(err.to_string().contains("cyclic"));
    }

    #[test]
    fn list_membership_is_term_exact_not_value_space() {
        // The single member is "1"^^xsd:integer. listIndexOf/listContains match by
        // structural (lexical + datatype) term identity — the SAME equality the
        // logic oracle uses (Prolog unification), which is the parity contract — and
        // NOT SPARQL value-space: "1"^^xsd:decimal is numerically equal but a
        // distinct term, so it does not match.
        use purrdf_core::RdfLiteral;
        const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
        const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";
        let mut b = RdfDatasetBuilder::new();
        let first = b.intern_iri(super::RDF_FIRST);
        let rest = b.intern_iri(super::RDF_REST);
        let nil = b.intern_iri(super::RDF_NIL);
        let l0 = b.intern_iri("http://ex/l0");
        let one_int = b.intern_literal(RdfLiteral::typed("1", XSD_INTEGER));
        b.push_quad(l0, first, one_int, None);
        b.push_quad(l0, rest, nil, None);
        let ds = b.freeze().expect("freeze");

        // The exact term is a member at index 0.
        let q_exact = format!(
            "{PREFIX} SELECT ?b ?n WHERE {{ \
             BIND(g:listContains(<http://ex/l0>, \"1\"^^<{XSD_INTEGER}>) AS ?b) \
             BIND(g:listIndexOf(<http://ex/l0>, \"1\"^^<{XSD_INTEGER}>) AS ?n) }}"
        );
        assert_eq!(
            rows(&ds, &q_exact),
            vec![vec!["true".to_owned(), "0".to_owned()]]
        );

        // A value-equal but structurally distinct term (different datatype) does not
        // match: listContains is false, listIndexOf is unbound.
        let q_distinct = format!(
            "{PREFIX} SELECT ?b ?n WHERE {{ \
             BIND(g:listContains(<http://ex/l0>, \"1\"^^<{XSD_DECIMAL}>) AS ?b) \
             BIND(g:listIndexOf(<http://ex/l0>, \"1\"^^<{XSD_DECIMAL}>) AS ?n) }}"
        );
        assert_eq!(
            rows(&ds, &q_distinct),
            vec![vec!["false".to_owned(), "UNBOUND".to_owned()]]
        );
    }

    #[test]
    fn torn_list_missing_rest_is_a_hard_data_error() {
        // l0 -> first x, rest l1 ; l1 -> first y  (no rdf:rest on the 2nd cell).
        let mut b = RdfDatasetBuilder::new();
        let first = b.intern_iri(super::RDF_FIRST);
        let rest = b.intern_iri(super::RDF_REST);
        let nil = b.intern_iri(super::RDF_NIL);
        let l0 = b.intern_iri("http://ex/l0");
        let l1 = b.intern_iri("http://ex/l1");
        let x = b.intern_iri("http://ex/x");
        let y = b.intern_iri("http://ex/y");
        let z = b.intern_iri("http://ex/z");
        b.push_quad(l0, first, x, None);
        b.push_quad(l0, rest, l1, None);
        b.push_quad(l1, first, y, None);
        // l1 has no rdf:rest — a torn list. Intern rdf:nil elsewhere so the walk starts.
        b.push_quad(z, rest, nil, None);
        let ds = b.freeze().expect("freeze");

        let q = format!("{PREFIX} SELECT ?n WHERE {{ BIND(g:listLength(<http://ex/l0>) AS ?n) }}");
        let err = eval_err(&ds, &q);
        assert!(matches!(err, EvalError::Data(_)), "got {err:?}");
        assert!(err.to_string().contains("missing rdf:rest"), "got {err}");
    }

    #[test]
    fn torn_list_interior_missing_first_is_a_hard_data_error() {
        // l0 -> first x, rest l1 ; l1 -> rest nil  (no rdf:first on the interior cell).
        let mut b = RdfDatasetBuilder::new();
        let first = b.intern_iri(super::RDF_FIRST);
        let rest = b.intern_iri(super::RDF_REST);
        let nil = b.intern_iri(super::RDF_NIL);
        let l0 = b.intern_iri("http://ex/l0");
        let l1 = b.intern_iri("http://ex/l1");
        let x = b.intern_iri("http://ex/x");
        b.push_quad(l0, first, x, None);
        b.push_quad(l0, rest, l1, None);
        // l1 has rdf:rest but no rdf:first — torn, and `members` is already non-empty
        // (so this is a torn interior cell, not a non-list head).
        b.push_quad(l1, rest, nil, None);
        let ds = b.freeze().expect("freeze");

        let q = format!("{PREFIX} SELECT ?n WHERE {{ BIND(g:listLength(<http://ex/l0>) AS ?n) }}");
        let err = eval_err(&ds, &q);
        assert!(matches!(err, EvalError::Data(_)), "got {err:?}");
        assert!(err.to_string().contains("missing rdf:first"), "got {err}");
    }

    #[test]
    fn multi_edge_dataset_cell_is_a_hard_data_error() {
        // A cell carrying two rdf:first quads is ambiguous — hard-fail rather than
        // pick an iteration-order-dependent branch.
        let mut b = RdfDatasetBuilder::new();
        let first = b.intern_iri(super::RDF_FIRST);
        let rest = b.intern_iri(super::RDF_REST);
        let nil = b.intern_iri(super::RDF_NIL);
        let l0 = b.intern_iri("http://ex/l0");
        let x = b.intern_iri("http://ex/x");
        let y = b.intern_iri("http://ex/y");
        b.push_quad(l0, first, x, None);
        b.push_quad(l0, first, y, None); // a second rdf:first — malformed
        b.push_quad(l0, rest, nil, None);
        let ds = b.freeze().expect("freeze");

        let q = format!("{PREFIX} SELECT ?n WHERE {{ BIND(g:listLength(<http://ex/l0>) AS ?n) }}");
        let err = eval_err(&ds, &q);
        assert!(matches!(err, EvalError::Data(_)), "got {err:?}");
        assert!(err.to_string().contains("multiple rdf:first"), "got {err}");
    }

    #[test]
    fn constructed_list_is_readable_within_the_same_query() {
        // A list minted by listSlice/listConcat must be walkable by another list
        // function in the SAME query: its cells live only in the per-query buffer, so
        // walk() consults the constructed buffer as well as the dataset.
        let ds = list_ds();

        // listLength(listSlice((x y z), 1, 3)) = |(y z)| = 2
        let q_len = format!(
            "{PREFIX} SELECT ?n WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listLength(g:listSlice(?l, 1, 3)) AS ?n) }}"
        );
        assert_eq!(rows(&ds, &q_len), vec![vec!["2".to_owned()]]);

        // listGet(listConcat(L, L), 3) = first member of the second copy = x
        let q_get = format!(
            "{PREFIX} SELECT ?x WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listGet(g:listConcat(?l, ?l), 3) AS ?x) }}"
        );
        assert_eq!(rows(&ds, &q_get), vec![vec!["<http://ex/x>".to_owned()]]);
    }

    // ── constructing functions: listSlice / listConcat ───────────────────────

    use purrdf_core::{SparqlEngine, SparqlRequest, SparqlResult, TermRef};

    use crate::engine::NativeSparqlEngine;

    const RDF_NIL_STR: &str = "<http://www.w3.org/1999/02/22-rdf-syntax-ns#nil>";

    /// Resolve a dataset to sorted `(s, p, o)` string triples.
    fn triples(ds: &RdfDataset) -> Vec<(String, String, String)> {
        let term = |id| match ds.resolve(id) {
            TermRef::Iri(i) => format!("<{i}>"),
            TermRef::Blank { label, .. } => format!("_:{label}"),
            TermRef::Literal { lexical, .. } => lexical.to_owned(),
            TermRef::Triple { .. } => "<<triple>>".to_owned(),
        };
        let mut out: Vec<_> = ds
            .quads()
            .map(|q| (term(q.s), term(q.p), term(q.o)))
            .collect();
        out.sort();
        out
    }

    /// Walk a constructed `rdf:List` from `head`, returning member object strings.
    fn members_of(ds: &RdfDataset, head: &str) -> Vec<String> {
        let first = format!("<{}>", super::RDF_FIRST);
        let rest = format!("<{}>", super::RDF_REST);
        let ts = triples(ds);
        let mut members = Vec::new();
        let mut cur = head.to_owned();
        while cur != RDF_NIL_STR {
            let f = ts
                .iter()
                .find(|(s, p, _)| s == &cur && p == &first)
                .map(|(_, _, o)| o.clone());
            let r = ts
                .iter()
                .find(|(s, p, _)| s == &cur && p == &rest)
                .map(|(_, _, o)| o.clone());
            match (f, r) {
                (Some(f), Some(r)) => {
                    members.push(f);
                    cur = r;
                }
                _ => break,
            }
        }
        members
    }

    /// Run a SELECT/ASK and return its rows plus the auxiliary constructed graph.
    fn run_constructed(
        ds: &Arc<RdfDataset>,
        query: &str,
    ) -> (Vec<Vec<Option<TermValue>>>, Arc<RdfDataset>) {
        let engine = NativeSparqlEngine::new().with_parser_options(ext_options());
        let res = engine
            .query(
                ds,
                SparqlRequest {
                    query,
                    base_iri: None,
                    substitutions: &[],
                },
            )
            .expect("query");
        match res {
            SparqlResult::Solutions { rows, aux, .. } => (rows, aux),
            other => panic!("expected solutions, got {other:?}"),
        }
    }

    /// Run a CONSTRUCT and return its output graph.
    fn run_graph(ds: &Arc<RdfDataset>, query: &str) -> Arc<RdfDataset> {
        let engine = NativeSparqlEngine::new().with_parser_options(ext_options());
        match engine
            .query(
                ds,
                SparqlRequest {
                    query,
                    base_iri: None,
                    substitutions: &[],
                },
            )
            .expect("query")
        {
            SparqlResult::Graph(g) => g,
            other => panic!("expected a graph, got {other:?}"),
        }
    }

    /// The single SELECT head cell as a comparable string (`<iri>` or `_:label`).
    fn head_str(rows: &[Vec<Option<TermValue>>]) -> String {
        match &rows[0][0] {
            Some(TermValue::Iri(i)) => format!("<{i}>"),
            Some(TermValue::Blank { label, .. }) => format!("_:{label}"),
            other => panic!("expected a list head term, got {other:?}"),
        }
    }

    #[test]
    fn list_slice_surfaces_subrange_in_aux_graph() {
        let ds = list_ds();
        let q = format!(
            "{PREFIX} SELECT ?s WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listSlice(?l, 1, 3) AS ?s) }}"
        );
        let (rows, aux) = run_constructed(&ds, &q);
        let head = head_str(&rows);
        assert!(head.starts_with("_:"), "head must be a fresh blank: {head}");
        assert_eq!(
            members_of(&aux, &head),
            vec!["<http://ex/y>".to_owned(), "<http://ex/z>".to_owned()]
        );
        // A 2-member list is exactly 4 cell quads.
        assert_eq!(aux.quad_count(), 4);
    }

    #[test]
    fn list_slice_empty_range_is_nil() {
        let ds = list_ds();
        let q = format!(
            "{PREFIX} SELECT ?s WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listSlice(?l, 2, 2) AS ?s) }}"
        );
        let (rows, aux) = run_constructed(&ds, &q);
        assert_eq!(head_str(&rows), RDF_NIL_STR);
        assert_eq!(aux.quad_count(), 0);
    }

    #[test]
    fn list_slice_clamps_out_of_bounds_and_inverted_ranges() {
        let ds = list_ds();
        // end past the list end → clamps to the full tail [1, len).
        let q = format!(
            "{PREFIX} SELECT ?s WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listSlice(?l, 1, 99) AS ?s) }}"
        );
        let (rows, aux) = run_constructed(&ds, &q);
        assert_eq!(
            members_of(&aux, &head_str(&rows)),
            vec!["<http://ex/y>".to_owned(), "<http://ex/z>".to_owned()]
        );
        // inverted range (start > end) → empty.
        let q = format!(
            "{PREFIX} SELECT ?s WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listSlice(?l, 2, 1) AS ?s) }}"
        );
        let (rows, _) = run_constructed(&ds, &q);
        assert_eq!(head_str(&rows), RDF_NIL_STR);
    }

    #[test]
    fn list_concat_appends_members() {
        let ds = list_ds();
        // concat the list with itself → [x, y, z, x, y, z].
        let q = format!(
            "{PREFIX} SELECT ?s WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listConcat(?l, ?l) AS ?s) }}"
        );
        let (rows, aux) = run_constructed(&ds, &q);
        assert_eq!(
            members_of(&aux, &head_str(&rows)),
            vec![
                "<http://ex/x>".to_owned(),
                "<http://ex/y>".to_owned(),
                "<http://ex/z>".to_owned(),
                "<http://ex/x>".to_owned(),
                "<http://ex/y>".to_owned(),
                "<http://ex/z>".to_owned(),
            ]
        );
    }

    #[test]
    fn list_concat_with_nil_is_identity_and_nil_nil_is_nil() {
        let ds = list_ds();
        let q = format!(
            "{PREFIX} SELECT ?s WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listConcat(?l, <{}>) AS ?s) }}",
            super::RDF_NIL
        );
        let (rows, aux) = run_constructed(&ds, &q);
        assert_eq!(
            members_of(&aux, &head_str(&rows)),
            vec![
                "<http://ex/x>".to_owned(),
                "<http://ex/y>".to_owned(),
                "<http://ex/z>".to_owned(),
            ]
        );
        // nil ++ nil → nil (no cells).
        let q = format!(
            "{PREFIX} SELECT ?s WHERE {{ BIND(g:listConcat(<{nil}>, <{nil}>) AS ?s) }}",
            nil = super::RDF_NIL
        );
        let (rows, aux) = run_constructed(&ds, &q);
        assert_eq!(head_str(&rows), RDF_NIL_STR);
        assert_eq!(aux.quad_count(), 0);
    }

    #[test]
    fn list_slice_materializes_into_construct_output() {
        let ds = list_ds();
        let q = format!(
            "{PREFIX} CONSTRUCT {{ <http://ex/out> <http://ex/has> ?s }} \
             WHERE {{ ?q <http://ex/list> ?l . BIND(g:listSlice(?l, 0, 2) AS ?s) }}"
        );
        let graph = run_graph(&ds, &q);
        // The head is the object of ex:out ex:has — find it, then walk the cells.
        let ts = triples(&graph);
        let head = ts
            .iter()
            .find(|(s, p, _)| s == "<http://ex/out>" && p == "<http://ex/has>")
            .map(|(_, _, o)| o.clone())
            .expect("the binding triple is present");
        assert_eq!(
            members_of(&graph, &head),
            vec!["<http://ex/x>".to_owned(), "<http://ex/y>".to_owned()]
        );
        // binding triple (1) + two cells (4) = 5 quads.
        assert_eq!(graph.quad_count(), 5);
    }

    #[test]
    fn pruned_row_does_not_leak_constructed_cells_into_aux() {
        // A list is minted on a row that FILTER then removes. Its cells were buffered,
        // but no row survives, so they must NOT surface in the SELECT aux graph (the
        // row↔aux contract — no orphaned cells).
        let ds = list_ds();
        let q = format!(
            "{PREFIX} SELECT ?s WHERE {{ ?q <http://ex/list> ?l . \
             BIND(g:listSlice(?l, 0, 2) AS ?s) FILTER(1 > 2) }}"
        );
        let (rows, aux) = run_constructed(&ds, &q);
        assert!(rows.is_empty(), "all rows are filtered out, got {rows:?}");
        assert_eq!(
            aux.quad_count(),
            0,
            "orphaned cells leaked: {:?}",
            triples(&aux)
        );
    }

    #[test]
    fn pruned_row_does_not_leak_constructed_cells_into_construct() {
        // Same contract for CONSTRUCT: a list minted on a filtered row contributes no
        // orphaned cells to the output graph.
        let ds = list_ds();
        let q = format!(
            "{PREFIX} CONSTRUCT {{ <http://ex/out> <http://ex/has> ?s }} \
             WHERE {{ ?q <http://ex/list> ?l . BIND(g:listSlice(?l, 0, 2) AS ?s) FILTER(1 > 2) }}"
        );
        let graph = run_graph(&ds, &q);
        assert_eq!(
            graph.quad_count(),
            0,
            "orphaned cells leaked: {:?}",
            triples(&graph)
        );
    }
}

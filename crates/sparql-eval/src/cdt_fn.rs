// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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
//! * **In** ([`to_cdt_term`]) — a literal the query *authored* keeps its lexical
//!   form byte for byte, so `"[  1 ,  2 ]"^^cdt:List` and `cdt:List(1,2)` stay
//!   different RDF terms (`list-functions/sameterm-04.rq`). Only when a
//!   `cdt:`-typed literal actually *parses* is it lifted to a
//!   [`CdtTerm::Composite`], so nesting a composite inside a composite costs one
//!   bracket level rather than a fresh round of string escaping.
//! * **Out** ([`from_cdt_term`]) — a value PurRDF *computed* is spelled in
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
//! [`BlankScope::qualify_label`] / [`BlankScope::unqualify_label`] — the kernel's
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
    CdtValue, MapRemoval, TextDirection,
};
use purrdf_core::{BlankScope, DatasetView, RdfTextDirection, TermValue};
use purrdf_sparql_algebra::CdtFn;

use crate::error::EvalError;
use crate::eval::EvalCtx;
use crate::scratch::SolutionTerm;

const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";

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
    match func {
        CdtFn::ListConstructor => {
            // A failed argument is the `null` element, not a failed call.
            let items = vals
                .iter()
                .map(|value| argument_element(value.as_ref()))
                .collect::<Result<Vec<_>, _>>()?;
            value_result(ctx, purrdf_cdt::list_constructor(items))
        }
        CdtFn::MapConstructor => {
            // The parser has already refused an odd argument count, so every chunk
            // is a full key/value pair.
            let mut pairs: Vec<(CdtTerm, CdtTerm)> = Vec::with_capacity(vals.len() / 2);
            for [key, value] in vals.as_chunks::<2>().0 {
                pairs.push((
                    argument_element(key.as_ref())?,
                    argument_element(value.as_ref())?,
                ));
            }
            value_result(ctx, purrdf_cdt::map_constructor(&pairs))
        }

        CdtFn::Concat => match composite_arguments(vals)? {
            Some(values) => value_result(ctx, purrdf_cdt::concat(&values)),
            None => Ok(None),
        },
        CdtFn::Merge => match composite_arguments(vals)? {
            Some(values) => value_result(ctx, purrdf_cdt::merge(&values)),
            None => Ok(None),
        },

        CdtFn::Size => match composite_argument(vals, 0)? {
            Some(value) => Ok(Some(integer_term(ctx, purrdf_cdt::size(&value)))),
            None => Ok(None),
        },
        CdtFn::Head => match composite_argument(vals, 0)? {
            Some(value) => term_result(ctx, purrdf_cdt::head(&value)),
            None => Ok(None),
        },
        CdtFn::Tail => match composite_argument(vals, 0)? {
            Some(value) => value_result(ctx, purrdf_cdt::tail(&value)),
            None => Ok(None),
        },
        CdtFn::Reverse => match composite_argument(vals, 0)? {
            Some(value) => value_result(ctx, purrdf_cdt::reverse(&value)),
            None => Ok(None),
        },
        CdtFn::Keys => match composite_argument(vals, 0)? {
            Some(value) => value_result(ctx, purrdf_cdt::keys(&value)),
            None => Ok(None),
        },

        CdtFn::Get => match (composite_argument(vals, 0)?, element_argument(vals, 1)?) {
            (Some(value), Some(key)) => term_result(ctx, purrdf_cdt::get(&value, &key)),
            _ => Ok(None),
        },
        CdtFn::Contains => match (composite_argument(vals, 0)?, element_argument(vals, 1)?) {
            (Some(value), Some(term)) => bool_result(ctx, purrdf_cdt::contains(&value, &term)),
            _ => Ok(None),
        },
        CdtFn::ContainsKey => match (composite_argument(vals, 0)?, element_argument(vals, 1)?) {
            (Some(value), Some(key)) => bool_result(ctx, purrdf_cdt::contains_key(&value, &key)),
            _ => Ok(None),
        },
        CdtFn::Subseq => {
            let (Some(value), Some(start)) =
                (composite_argument(vals, 0)?, element_argument(vals, 1)?)
            else {
                return Ok(None);
            };
            // The third argument is a LENGTH and is optional; supplied-but-failed is
            // not the same as omitted, so it raises rather than running to the end.
            let length = match vals.get(2) {
                None => None,
                Some(None) => return Ok(None),
                Some(Some(_)) => Some(element_argument(vals, 2)?.ok_or_else(|| {
                    EvalError::internal("a bound cdt:subseq length argument yielded no element")
                })?),
            };
            value_result(ctx, purrdf_cdt::subseq(&value, &start, length.as_ref()))
        }
        CdtFn::Put => {
            let (Some(value), Some(key)) =
                (composite_argument(vals, 0)?, element_argument(vals, 1)?)
            else {
                return Ok(None);
            };
            // An omitted OR failed value argument is the `null` entry
            // (`map-functions/put-02.rq` and `put-03.rq`), which is why this is
            // `argument_element` and not `element_argument`.
            let item = argument_element(vals.get(2).and_then(Option::as_ref))?;
            value_result(ctx, purrdf_cdt::put(&value, &key, &item))
        }
        CdtFn::Remove => {
            let (Some(value), Some(key)) =
                (composite_argument(vals, 0)?, element_argument(vals, 1)?)
            else {
                return Ok(None);
            };
            match purrdf_cdt::remove(&value, &key) {
                CdtOutcome::Value(MapRemoval::Removed(value)) => {
                    Ok(Some(intern(ctx, composite_literal(&value))))
                }
                // Nothing was removed, so the answer is the caller's OWN term, with
                // its own lexical form — `map-functions/remove-01.rq` asserts it
                // with `SAMETERM`, which a re-rendered equal map would fail.
                CdtOutcome::Value(MapRemoval::Unchanged) => {
                    let original = vals[0]
                        .clone()
                        .ok_or_else(|| EvalError::internal("cdt:remove lost its map argument"))?;
                    Ok(Some(intern(ctx, original)))
                }
                CdtOutcome::Error(_) => Ok(None),
                CdtOutcome::Bound(error) => Err(bound(&error)),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// outcome plumbing
// ---------------------------------------------------------------------------

/// A refused mint: the value the function was asked to build crosses one of
/// `purrdf-cdt`'s three bounds. A hard failure, never an expression error.
fn bound(error: &CdtError) -> EvalError {
    EvalError::composite_bound(error.to_string())
}

/// A [`CdtOutcome<CdtValue>`] as a solution term: the value is minted in canonical
/// form.
fn value_result<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    outcome: CdtOutcome<CdtValue>,
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    match outcome {
        CdtOutcome::Value(value) => Ok(Some(intern(ctx, composite_literal(&value)))),
        CdtOutcome::Error(_) => Ok(None),
        CdtOutcome::Bound(error) => Err(bound(&error)),
    }
}

/// A [`CdtOutcome<CdtTerm>`] as a solution term. A `null` element has no term to
/// return, so it is an expression error — the same answer as an absent one, which
/// is exactly what `list-functions/get-null-01.rq` and `get-null-02.rq` require.
fn term_result<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    outcome: CdtOutcome<CdtTerm>,
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    match outcome {
        CdtOutcome::Value(term) => match from_cdt_term(&term) {
            Some(value) => Ok(Some(intern(ctx, value))),
            None => Ok(None),
        },
        CdtOutcome::Error(_) => Ok(None),
        CdtOutcome::Bound(error) => Err(bound(&error)),
    }
}

/// A [`CdtOutcome<bool>`] as an `xsd:boolean` solution term.
fn bool_result<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    outcome: CdtOutcome<bool>,
) -> Result<Option<SolutionTerm<D::Id>>, EvalError> {
    match outcome {
        CdtOutcome::Value(answer) => Ok(Some(bool_term(ctx, answer))),
        CdtOutcome::Error(_) => Ok(None),
        CdtOutcome::Bound(error) => Err(bound(&error)),
    }
}

// ---------------------------------------------------------------------------
// argument shapes
// ---------------------------------------------------------------------------

/// A **constructor** argument: a failed one is the SEP-0009 `null` element rather
/// than a failure of the call (`list-functions/list-constructor-null-01.rq`,
/// `list-constructor-null-02.rq`, `map-functions/put-03.rq`).
fn argument_element(value: Option<&TermValue>) -> Result<CdtTerm, EvalError> {
    match value {
        None => Ok(CdtTerm::Null),
        Some(value) => to_cdt_term(value),
    }
}

/// An ordinary (strict) element argument: `None` means the call raises.
fn element_argument(
    vals: &[Option<TermValue>],
    index: usize,
) -> Result<Option<CdtTerm>, EvalError> {
    match vals.get(index).and_then(Option::as_ref) {
        None => Ok(None),
        Some(value) => to_cdt_term(value).map(Some),
    }
}

/// A composite argument: the term must be a `cdt:List` / `cdt:Map` literal whose
/// lexical form parses. Anything else — a plain string, an IRI, an ill-formed
/// composite literal — yields `None`, i.e. a SPARQL expression error
/// (`list-functions/size-error-01.rq`, `get-error-01.rq`).
fn composite_argument(
    vals: &[Option<TermValue>],
    index: usize,
) -> Result<Option<CdtValue>, EvalError> {
    Ok(vals
        .get(index)
        .and_then(Option::as_ref)
        .and_then(as_composite))
}

/// Every argument as a composite, or `None` if any one of them is not
/// (`list-functions/concat-error-01.rq`: one non-list poisons the whole call).
fn composite_arguments(vals: &[Option<TermValue>]) -> Result<Option<Vec<CdtValue>>, EvalError> {
    let mut values = Vec::with_capacity(vals.len());
    for value in vals {
        let Some(value) = value.as_ref().and_then(as_composite) else {
            return Ok(None);
        };
        values.push(value);
    }
    Ok(Some(values))
}

/// The composite value a term denotes, or `None` when it denotes none.
pub(crate) fn as_composite(value: &TermValue) -> Option<CdtValue> {
    let TermValue::Literal {
        lexical_form,
        datatype,
        language,
        ..
    } = value
    else {
        return None;
    };
    // A language tag makes the literal an `rdf:langString` whatever else it
    // carries, so it is not a composite.
    if language.is_some() {
        return None;
    }
    purrdf_cdt::parse_cdt_by_iri(lexical_form, datatype)
        .ok()
        .flatten()
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

/// A composite value as the literal that carries it, in `purrdf-cdt`'s canonical
/// lexical form.
///
/// Canonical because PurRDF *computed* this value: two independent evaluations of
/// `cdt:List()` must be the same RDF term (`list-functions/sameterm-01.rq`), which
/// only a deterministic spelling can give. A literal the query authored is never
/// re-spelled — see [`to_cdt_term`].
pub(crate) fn composite_literal(value: &CdtValue) -> TermValue {
    TermValue::Literal {
        lexical_form: value.canonical_lexical(),
        datatype: match value.datatype() {
            CdtDatatype::List => CDT_LIST.to_owned(),
            CdtDatatype::Map => CDT_MAP.to_owned(),
        },
        language: None,
        direction: None,
    }
}

/// One step of the iterative [`to_cdt_term`] walk.
enum InJob<'a> {
    /// Convert this term and push the result.
    Visit(&'a TermValue),
    /// Pop three converted components and combine them into a triple term.
    Triple,
}

/// Convert an evaluator term into the composite element it stands for.
///
/// The mapping, and the two places it is not the obvious one:
///
/// * a **blank node** carries its `(label, scope)` pair through
///   [`BlankScope::qualify_label`], so a blank from a non-default scope cannot
///   collide with a same-labelled blank from another;
/// * a `cdt:`-typed literal whose lexical form **parses** is lifted to a
///   [`CdtTerm::Composite`], so nesting costs one bracket level rather than a
///   fresh round of string escaping — and one whose lexical form does **not**
///   parse stays a literal, verbatim, so the relations can report it as ill-typed
///   rather than silently treating it as absent.
///
/// # Errors
///
/// [`EvalError::CompositeBound`] when the element could never appear in any
/// composite — a nested value already at the nesting bound, or a triple term whose
/// three components combine past one of the three bounds. That is a hard failure,
/// not an expression error: the term exists, and there is no composite that can
/// hold it.
fn to_cdt_term(value: &TermValue) -> Result<CdtTerm, EvalError> {
    let mut jobs: Vec<InJob<'_>> = vec![InJob::Visit(value)];
    let mut done: Vec<CdtTerm> = Vec::new();
    while let Some(job) = jobs.pop() {
        match job {
            InJob::Triple => {
                // Pushed subject-first, so they pop object, predicate, subject.
                let object = pop(&mut done)?;
                let predicate = pop(&mut done)?;
                let subject = pop(&mut done)?;
                done.push(CdtTerm::triple(subject, predicate, object).map_err(|e| bound(&e))?);
            }
            InJob::Visit(TermValue::Iri(iri)) => done.push(CdtTerm::Iri(iri.clone())),
            InJob::Visit(TermValue::Blank { label, scope }) => {
                done.push(CdtTerm::Blank(scope.qualify_label(label).into_owned()));
            }
            InJob::Visit(
                literal @ TermValue::Literal {
                    lexical_form,
                    datatype,
                    language,
                    direction,
                },
            ) => {
                if let Some(composite) = as_composite(literal) {
                    done.push(CdtTerm::composite(composite).map_err(|e| bound(&e))?);
                } else {
                    done.push(CdtTerm::Literal(cdt_literal(
                        lexical_form,
                        datatype,
                        language.as_deref(),
                        *direction,
                    )));
                }
            }
            InJob::Visit(TermValue::Triple { s, p, o }) => {
                jobs.push(InJob::Triple);
                jobs.push(InJob::Visit(o));
                jobs.push(InJob::Visit(p));
                jobs.push(InJob::Visit(s));
            }
        }
    }
    pop(&mut done)
}

/// One step of the iterative [`from_cdt_term`] walk.
enum OutJob<'a> {
    /// Convert this element and push the result.
    Visit(&'a CdtTerm),
    /// Pop three converted components and combine them into a triple term.
    Triple,
}

/// Convert a composite element back into an evaluator term, or `None` when it is
/// the SEP-0009 `null` element.
///
/// `null` is a position in a value that carries no term at all, so there is
/// nothing to return and the caller turns it into a SPARQL expression error —
/// `list-functions/get-null-01.rq`. That propagates out of a triple term too: a
/// triple with a null component is not a term.
///
/// A [`CdtTerm::Composite`] becomes a `cdt:List` / `cdt:Map` literal carrying the
/// value's canonical lexical form, and a [`CdtTerm::Blank`] becomes a real blank
/// node, decoded back to its `(label, scope)` pair — the exact inverse of what
/// [`to_cdt_term`] encoded.
pub(crate) fn from_cdt_term(term: &CdtTerm) -> Option<TermValue> {
    let mut jobs: Vec<OutJob<'_>> = vec![OutJob::Visit(term)];
    let mut done: Vec<TermValue> = Vec::new();
    while let Some(job) = jobs.pop() {
        match job {
            OutJob::Triple => {
                let object = done.pop()?;
                let predicate = done.pop()?;
                let subject = done.pop()?;
                done.push(TermValue::Triple {
                    s: Box::new(subject),
                    p: Box::new(predicate),
                    o: Box::new(object),
                });
            }
            OutJob::Visit(CdtTerm::Null) => return None,
            OutJob::Visit(CdtTerm::Iri(iri)) => done.push(TermValue::Iri(iri.clone())),
            OutJob::Visit(CdtTerm::Blank(qualified)) => {
                let (label, scope) = BlankScope::unqualify_label(qualified);
                done.push(TermValue::Blank {
                    label: label.into_owned(),
                    scope,
                });
            }
            OutJob::Visit(CdtTerm::Literal(literal)) => done.push(TermValue::Literal {
                lexical_form: literal.lexical.clone(),
                datatype: literal.datatype.clone(),
                language: literal.language.clone(),
                direction: literal.direction.map(|d| match d {
                    TextDirection::Ltr => RdfTextDirection::Ltr,
                    TextDirection::Rtl => RdfTextDirection::Rtl,
                }),
            }),
            OutJob::Visit(CdtTerm::Composite(value)) => {
                done.push(composite_literal(value.as_ref()));
            }
            OutJob::Visit(CdtTerm::TripleTerm(triple)) => {
                let CdtTripleTerm {
                    subject,
                    predicate,
                    object,
                } = triple.as_ref();
                jobs.push(OutJob::Triple);
                jobs.push(OutJob::Visit(object));
                jobs.push(OutJob::Visit(predicate));
                jobs.push(OutJob::Visit(subject));
            }
        }
    }
    done.pop()
}

/// Pop the single converted element a completed walk step left behind.
fn pop(done: &mut Vec<CdtTerm>) -> Result<CdtTerm, EvalError> {
    done.pop()
        .ok_or_else(|| EvalError::internal("the composite-element walk lost a converted component"))
}

// ---------------------------------------------------------------------------
// comparison — SEP-0009 `=` and `<` over composite-typed operands
// ---------------------------------------------------------------------------

/// Which comparison a caller is asking [`compare`] for.
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
pub(crate) fn compare(relation: CdtRelation, left: &TermValue, right: &TermValue) -> Option<bool> {
    let (left, right) = (to_cdt_operand(left), to_cdt_operand(right));
    let (first, second) = match relation {
        // `a > b` is `b < a`, and `a >= b` is `b <= a`; SEP-0009 defines only `<`.
        CdtRelation::Greater | CdtRelation::GreaterOrEqual => (&right, &left),
        CdtRelation::Equal | CdtRelation::Less | CdtRelation::LessOrEqual => (&left, &right),
    };
    let answer = match relation {
        CdtRelation::Equal => purrdf_cdt::term_equal(first, second),
        CdtRelation::Less | CdtRelation::Greater => purrdf_cdt::term_less_than(first, second),
        CdtRelation::LessOrEqual | CdtRelation::GreaterOrEqual => {
            match purrdf_cdt::term_less_than(first, second) {
                Ok(true) => Ok(true),
                Ok(false) => purrdf_cdt::term_equal(first, second),
                Err(error) => Err(error),
            }
        }
    };
    answer.ok()
}

/// One step of the iterative [`to_cdt_operand`] walk.
enum OperandJob<'a> {
    /// Convert this term and push the result.
    Visit(&'a TermValue),
    /// Pop three converted components and combine them into a triple term.
    Triple,
}

/// A comparison **operand**, as an element the SEP-0009 relations can read.
///
/// Deliberately NOT [`to_cdt_term`], in two ways, and both matter:
///
/// * a `cdt:`-typed literal is left as a literal rather than lifted to a
///   [`CdtTerm::Composite`]. The relations already see through both spellings
///   ([`purrdf_cdt::term_equal`] and [`purrdf_cdt::term_less_than`] resolve a
///   composite-typed literal themselves, which is what makes
///   `list-functions/contains-07.rq` and `contains-08.rq` agree), and lifting
///   would make a comparison *fail* — a value already at the nesting bound has no
///   element form — where SEP-0009 says it simply has an answer;
/// * a triple term is assembled from the variant directly rather than through the
///   bounds-checking [`CdtTerm::triple`]. Nothing built here is ever placed inside
///   a [`CdtValue`], so it carries no bound to establish: it is read once by a
///   relation and dropped, and its depth is the depth of a term that is already in
///   the dataset.
///
/// Total, allocation-bounded by the input term, and iterative — comparison
/// operands come from literals, so a recursive walk here would be a stack overflow
/// an attacker chooses the depth of.
fn to_cdt_operand(value: &TermValue) -> CdtTerm {
    let mut jobs: Vec<OperandJob<'_>> = vec![OperandJob::Visit(value)];
    let mut done: Vec<CdtTerm> = Vec::new();
    while let Some(job) = jobs.pop() {
        match job {
            OperandJob::Triple => {
                // Pushed subject-first, so they pop object, predicate, subject. The
                // walk pushes exactly three `Visit`s before each `Triple`, so the
                // three pops are always available; `Null` is unreachable as a
                // component because no `TermValue` denotes it.
                let (Some(object), Some(predicate), Some(subject)) =
                    (done.pop(), done.pop(), done.pop())
                else {
                    return CdtTerm::Null;
                };
                done.push(CdtTerm::TripleTerm(Box::new(CdtTripleTerm {
                    subject,
                    predicate,
                    object,
                })));
            }
            OperandJob::Visit(TermValue::Iri(iri)) => done.push(CdtTerm::Iri(iri.clone())),
            OperandJob::Visit(TermValue::Blank { label, scope }) => {
                done.push(CdtTerm::Blank(scope.qualify_label(label).into_owned()));
            }
            OperandJob::Visit(TermValue::Literal {
                lexical_form,
                datatype,
                language,
                direction,
            }) => done.push(CdtTerm::Literal(cdt_literal(
                lexical_form,
                datatype,
                language.as_deref(),
                *direction,
            ))),
            OperandJob::Visit(TermValue::Triple { s, p, o }) => {
                jobs.push(OperandJob::Triple);
                jobs.push(OperandJob::Visit(o));
                jobs.push(OperandJob::Visit(p));
                jobs.push(OperandJob::Visit(s));
            }
        }
    }
    done.pop().unwrap_or(CdtTerm::Null)
}

/// An evaluator literal as a composite-element literal, verbatim.
fn cdt_literal(
    lexical: &str,
    datatype: &str,
    language: Option<&str>,
    direction: Option<RdfTextDirection>,
) -> CdtLiteral {
    CdtLiteral {
        lexical: lexical.to_owned(),
        datatype: datatype.to_owned(),
        language: language.map(str::to_owned),
        direction: direction.map(|d| match d {
            RdfTextDirection::Ltr => TextDirection::Ltr,
            RdfTextDirection::Rtl => TextDirection::Rtl,
        }),
    }
}

// ---------------------------------------------------------------------------
// interning helpers
// ---------------------------------------------------------------------------

/// Intern a value to a solution term (promoting to an existing dataset id).
fn intern<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    value: TermValue,
) -> SolutionTerm<D::Id> {
    ctx.scratch.intern(ctx.dataset, value)
}

/// Intern an `xsd:integer` literal.
fn integer_term<D: DatasetView + Sync>(
    ctx: &mut EvalCtx<'_, D>,
    value: usize,
) -> SolutionTerm<D::Id> {
    intern(
        ctx,
        TermValue::Literal {
            lexical_form: value.to_string(),
            datatype: XSD_INTEGER.to_owned(),
            language: None,
            direction: None,
        },
    )
}

/// Intern an `xsd:boolean` literal.
fn bool_term<D: DatasetView + Sync>(ctx: &mut EvalCtx<'_, D>, answer: bool) -> SolutionTerm<D::Id> {
    intern(
        ctx,
        TermValue::Literal {
            lexical_form: if answer { "true" } else { "false" }.to_owned(),
            datatype: XSD_BOOLEAN.to_owned(),
            language: None,
            direction: None,
        },
    )
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
    fn nesting_past_the_depth_bound_is_a_hard_failure() {
        // 64 nested constructors is the deepest composite that can exist; the 65th
        // has nowhere to go. The refusal is a HARD failure, not an unbound
        // variable — a `FILTER(!BOUND(?x))` must not be satisfiable by a resource
        // refusal, or a hostile query could use one to change a result set.
        let nest = |depth: usize| {
            let mut expression = "cdt:List()".to_owned();
            for _ in 1..depth {
                expression = format!("cdt:List({expression})");
            }
            format!("BIND({expression} AS ?x) FILTER(BOUND(?x))")
        };
        assert!(ask(&nest(64)), "64 levels is within the bound");
        let error = ask_err(&nest(65));
        assert!(
            matches!(error, EvalError::CompositeBound(_)),
            "got {error:?}"
        );
        assert!(error.to_string().contains("nesting"), "got {error}");
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
            matches!(error, EvalError::CompositeBound(_)),
            "got {error:?}"
        );
        assert!(error.to_string().contains("elements"), "got {error}");
    }
}

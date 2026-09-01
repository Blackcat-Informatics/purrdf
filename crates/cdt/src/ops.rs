// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The two orders this crate defines over composite values, and why there are two.
//!
//! # 1. The SEP-0009 value relations — partial, and they raise
//!
//! [`list_equal`] / [`map_equal`] / [`list_less_than`] / [`map_less_than`] are the
//! spec's own operators. They are `Result<bool, CdtTypeError>` because SPARQL
//! comparison has three outcomes, not two: a positional pair that cannot be compared
//! at all is a **type error** that propagates out of the whole expression, and must
//! never be silently read as "not equal". `Err` here is that outcome.
//!
//! # 2. The syntactic total order — total, and it never raises
//!
//! [`total_term_cmp`] / [`total_key_cmp`] / [`total_value_cmp`] give every pair of
//! elements an answer. `ORDER BY` needs one (a sort cannot raise per-comparison and
//! still terminate with an order), and so do this crate's map key order and render
//! order, which is what makes [`crate::canonical_lexical`] byte-deterministic.
//!
//! ## Why it is syntactic, and not "value order with a structural tie-break"
//!
//! The obvious composite — compare by value first, fall back to a structural
//! tie-break when the values are incomparable — is **not transitive**, so it is not
//! a total order at all, and Rust's sort is entitled to panic when handed it. A
//! three-element counterexample, all of them literals:
//!
//! * `A = "9"^^xsd:double`, `B = "P1D"^^xsd:duration`, `C = "8"^^xsd:float`.
//! * `A` and `B` are value-incomparable (a number against a duration), so the
//!   tie-break decides: `xsd:double` sorts before `xsd:duration`, so `A < B`.
//! * `B` and `C` are value-incomparable too, and `xsd:duration` sorts before
//!   `xsd:float`, so `B < C`.
//! * `A` and `C` **are** value-comparable, and `9 > 8`, so `C < A`.
//!
//! That is a cycle: `A < B < C < A`. The failure is structural, not a quirk of these
//! three constants — any value-incomparable element can be slotted between two
//! comparable ones whose value order and syntactic order disagree. The order this
//! module exports is therefore purely syntactic: a category rank, then a
//! lexicographic walk of the element's own syntactic components. That is provably
//! transitive (it is a lexicographic product of total orders), allocation-free, and
//! identical on every host — the three properties a sort comparator and a canonical
//! renderer both need. Value semantics stay where they belong: in the partial,
//! raising relations above.
//!
//! # Everything here is iterative
//!
//! Composite values are trees over attacker-controlled lexical input, and a stack
//! overflow in Rust is an `abort`, not a catchable panic. Every function in this
//! module walks the tree with an explicit heap worklist; none of them recurses.

use alloc::vec::Vec;
use core::cmp::Ordering;

use purrdf_xsd::XsdValue;

use crate::datatype::CdtDatatype;
use crate::error::CdtTypeError;
use crate::limits::MAX_NESTING_DEPTH;
use crate::term::{CdtEntry, CdtKey, CdtLiteral, CdtTerm};
use crate::value::CdtValue;

// ── 1. The syntactic total order ────────────────────────────────────────────────

/// One step of the iterative total-order comparison.
enum CmpJob<'a> {
    /// Compare two terms.
    Pair(&'a CdtTerm, &'a CdtTerm),
    /// Compare two map keys (always leaves).
    KeyPair(&'a CdtKey, &'a CdtKey),
    /// A comparison already decided by the enclosing structure (a length tie-break,
    /// or a composite-kind mismatch).
    Decided(Ordering),
}

/// The crate's syntactic total order over two literals: datatype IRI, then language
/// tag, then base direction, then lexical form — each by Unicode scalar order, with
/// `None` before `Some`.
fn literal_cmp(a: &CdtLiteral, b: &CdtLiteral) -> Ordering {
    a.datatype
        .cmp(&b.datatype)
        .then_with(|| a.language.cmp(&b.language))
        .then_with(|| a.direction.cmp(&b.direction))
        .then_with(|| a.lexical.cmp(&b.lexical))
}

/// The crate's syntactic total order over two map keys.
///
/// Strict on distinct keys: two keys compare `Equal` only when they are the same
/// term. That is what lets a map hold its entries as a sorted sequence with exactly
/// one admissible arrangement per value.
///
/// # Examples
///
/// ```rust
/// use core::cmp::Ordering;
///
/// use purrdf_cdt::{CdtKey, CdtLiteral, total_key_cmp};
///
/// let iri = CdtKey::Iri("http://example.org/a".into());
/// let lit = CdtKey::Literal(CdtLiteral::plain("a"));
/// // IRIs sort before literals.
/// assert_eq!(total_key_cmp(&iri, &lit), Ordering::Less);
/// // Distinct lexical forms of one value stay distinct.
/// let one = CdtKey::Literal(CdtLiteral::typed("1", "http://www.w3.org/2001/XMLSchema#integer"));
/// let oh_one = CdtKey::Literal(CdtLiteral::typed("01", "http://www.w3.org/2001/XMLSchema#integer"));
/// assert_ne!(total_key_cmp(&one, &oh_one), Ordering::Equal);
/// ```
#[must_use]
pub fn total_key_cmp(a: &CdtKey, b: &CdtKey) -> Ordering {
    a.rank().cmp(&b.rank()).then_with(|| match (a, b) {
        (CdtKey::Iri(x), CdtKey::Iri(y)) => x.cmp(y),
        (CdtKey::Literal(x), CdtKey::Literal(y)) => literal_cmp(x, y),
        // Equal ranks imply the same variant, so this arm is unreachable; answering
        // `Equal` keeps the function total rather than panicking on an impossible
        // input.
        _ => Ordering::Equal,
    })
}

/// The crate's syntactic total order over two elements.
///
/// Category rank first — `null` < blank node < IRI < literal < triple term <
/// composite — then the category's own order. Composites compare element-wise and
/// then by length; a list sorts before a map.
///
/// Walks the two trees with an explicit heap worklist; it never recurses.
///
/// # Examples
///
/// ```rust
/// use core::cmp::Ordering;
///
/// use purrdf_cdt::{CdtLiteral, CdtTerm, total_term_cmp};
///
/// assert_eq!(
///     total_term_cmp(&CdtTerm::Null, &CdtTerm::Iri("http://example.org/a".into())),
///     Ordering::Less
/// );
/// // Within literals the order is syntactic, so it is defined even for pairs the
/// // value order calls incomparable.
/// let nan = CdtTerm::Literal(CdtLiteral::typed("NaN", "http://www.w3.org/2001/XMLSchema#double"));
/// let one = CdtTerm::Literal(CdtLiteral::typed("1", "http://www.w3.org/2001/XMLSchema#double"));
/// assert_ne!(total_term_cmp(&nan, &one), Ordering::Equal);
/// ```
#[must_use]
pub fn total_term_cmp(a: &CdtTerm, b: &CdtTerm) -> Ordering {
    run_cmp(alloc::vec![CmpJob::Pair(a, b)])
}

/// The crate's syntactic total order over two composite values.
///
/// This is the order [`CdtValue::map`](crate::CdtValue::map) sorts entries with and
/// the order [`crate::canonical_lexical`] writes a map in.
#[must_use]
pub fn total_value_cmp(a: &CdtValue, b: &CdtValue) -> Ordering {
    let mut jobs: Vec<CmpJob<'_>> = Vec::new();
    push_value_cmp(&mut jobs, a, b);
    run_cmp(jobs)
}

/// Drive the comparison worklist to a verdict.
fn run_cmp(mut jobs: Vec<CmpJob<'_>>) -> Ordering {
    while let Some(job) = jobs.pop() {
        let decided = match job {
            CmpJob::Decided(ordering) => ordering,
            CmpJob::KeyPair(x, y) => total_key_cmp(x, y),
            CmpJob::Pair(x, y) => {
                let by_rank = x.rank().cmp(&y.rank());
                if by_rank != Ordering::Equal {
                    return by_rank;
                }
                match (x, y) {
                    (CdtTerm::Null, CdtTerm::Null) => Ordering::Equal,
                    (CdtTerm::Blank(p), CdtTerm::Blank(q)) => p.cmp(q),
                    (CdtTerm::Iri(p), CdtTerm::Iri(q)) => p.cmp(q),
                    (CdtTerm::Literal(p), CdtTerm::Literal(q)) => literal_cmp(p, q),
                    (CdtTerm::TripleTerm(p), CdtTerm::TripleTerm(q)) => {
                        jobs.push(CmpJob::Pair(&p.object, &q.object));
                        jobs.push(CmpJob::Pair(&p.predicate, &q.predicate));
                        jobs.push(CmpJob::Pair(&p.subject, &q.subject));
                        continue;
                    }
                    (CdtTerm::Composite(p), CdtTerm::Composite(q)) => {
                        push_value_cmp(&mut jobs, p.as_ref(), q.as_ref());
                        continue;
                    }
                    // Equal ranks imply the same variant; answering `Equal` keeps the
                    // function total rather than panicking on an impossible input.
                    _ => Ordering::Equal,
                }
            }
        };
        if decided != Ordering::Equal {
            return decided;
        }
    }
    Ordering::Equal
}

/// Push the jobs comparing two composites, in reverse evaluation order (the
/// worklist pops LIFO, so the length tie-break goes on first and is consulted last).
fn push_value_cmp<'a>(jobs: &mut Vec<CmpJob<'a>>, a: &'a CdtValue, b: &'a CdtValue) {
    match (a, b) {
        (CdtValue::List(_), CdtValue::Map(_)) => jobs.push(CmpJob::Decided(Ordering::Less)),
        (CdtValue::Map(_), CdtValue::List(_)) => jobs.push(CmpJob::Decided(Ordering::Greater)),
        (CdtValue::List(left), CdtValue::List(right)) => {
            jobs.push(CmpJob::Decided(left.len().cmp(&right.len())));
            for (p, q) in left.iter().zip(right.iter()).rev() {
                jobs.push(CmpJob::Pair(p, q));
            }
        }
        (CdtValue::Map(left), CdtValue::Map(right)) => {
            jobs.push(CmpJob::Decided(left.len().cmp(&right.len())));
            for (p, q) in left.iter().zip(right.iter()).rev() {
                jobs.push(CmpJob::Pair(&p.value, &q.value));
                jobs.push(CmpJob::KeyPair(&p.key, &q.key));
            }
        }
    }
}

// ── 2. The SEP-0009 value relations ─────────────────────────────────────────────

/// The XSD value a literal denotes, or `None` when it denotes none this crate can
/// reach: a language-tagged string, an unmodelled datatype, or an ill-typed lexical.
fn xsd_value(literal: &CdtLiteral) -> Option<XsdValue> {
    if literal.language.is_some() {
        return None;
    }
    purrdf_xsd::parse_by_iri(&literal.lexical, &literal.datatype)
        .ok()
        .flatten()
}

/// The composite value a `cdt:List` / `cdt:Map` **literal** denotes, or `None` when
/// the literal is not one of those.
///
/// # Why an element that *is* a literal can still be a composite
///
/// SEP-0009 admits one and the same composite element written two ways inside a
/// lexical form: as the grammar's own nested `List` / `Map` production, or as an
/// ordinary `RDFLiteral` carrying the composite datatype. The corpus writes the same
/// test both ways and demands the same answer —
/// `list-functions/contains-07.rq` nests `[2]` while `contains-08.rq` writes
/// `'[2]'^^cdt:List`, and both must find it; `contains-09.rq` and `contains-10.rq`
/// do the same for a map. So the two spellings denote the same **value**, and the
/// value relations have to see through the literal one.
///
/// Term identity does not, and must not: `list-functions/sameterm-03.rq` requires two
/// spellings of one value to be different *terms*. That is exactly why the literal
/// keeps its lexical form ([`CdtLiteral`] is verbatim), why map keys are still
/// distinguished lexically, and why this resolution lives here in the value relation
/// and nowhere else.
///
/// `Some(Err(_))` is a literal that claims a composite datatype and whose lexical
/// form does not parse: it is ill-typed and denotes nothing, so comparing it is a
/// type error rather than an inequality.
fn literal_composite(literal: &CdtLiteral) -> Option<Result<CdtValue, CdtTypeError>> {
    if literal.language.is_some() {
        return None;
    }
    let datatype = CdtDatatype::from_iri(&literal.datatype)?;
    Some(
        crate::parse::parse_cdt(&literal.lexical, datatype).map_err(|_| CdtTypeError {
            reason: "a cdt:List / cdt:Map literal whose lexical form is malformed has no value",
        }),
    )
}

/// Spend one level of the composite-literal resolution budget.
///
/// Resolving a `cdt:`-typed literal into a value is the one step in this module that
/// re-enters the comparator, because the value it yields is owned and cannot be
/// pushed onto a worklist of borrowed terms. A literal may carry a literal that
/// carries a literal, so the chain is bounded here — by the same
/// [`MAX_NESTING_DEPTH`] that bounds *syntactic* nesting, since a composite reached
/// through an embedded literal is nested just as surely as one reached through a
/// bracket. That keeps the re-entry depth at 64 frames whatever the input, which is
/// the same budget the value tree's own `Drop` glue already runs on.
fn spend(budget: usize) -> Result<usize, CdtTypeError> {
    budget.checked_sub(1).ok_or(CdtTypeError {
        reason: "cdt:List / cdt:Map literals nested deeper than the composite nesting bound",
    })
}

/// SPARQL `=` over two literals.
///
/// Same term is always `true`. Otherwise, in order: a literal that denotes a
/// composite is compared as that composite, and is never equal to a literal that does
/// not denote one; a language-tagged string is never equal to anything it is not
/// identical to; two literals whose datatypes are both in the XSD value space have a
/// definite answer (different value spaces are `false`, not an error — SPARQL's
/// `RDFterm-equal` calls them "known to be different"); anything else is a type
/// error.
fn literal_equal(a: &CdtLiteral, b: &CdtLiteral, budget: usize) -> Result<bool, CdtTypeError> {
    if a == b {
        return Ok(true);
    }
    match (literal_composite(a), literal_composite(b)) {
        (Some(left), Some(right)) => {
            return value_equal_at(&left?, &right?, spend(budget)?);
        }
        // A composite value is not an XSD value and not a language-tagged string, so
        // this is "known to be different" — but an ill-typed composite literal still
        // denotes nothing, and that is an error rather than an inequality.
        (Some(composite), None) | (None, Some(composite)) => {
            composite?;
            return Ok(false);
        }
        (None, None) => {}
    }
    if a.language.is_some() || b.language.is_some() {
        // Language-tagged strings have no value space beyond term identity, and a
        // language-tagged string is never the same value as a typed literal.
        return Ok(false);
    }
    match (xsd_value(a), xsd_value(b)) {
        (Some(x), Some(y)) => Ok(purrdf_xsd::value_eq(&x, &y)),
        _ => Err(CdtTypeError {
            reason: "cannot compare literals whose datatype is outside the XSD value space",
        }),
    }
}

/// SPARQL `=` over two elements that are not both composites and not both triple
/// terms (those two cases are driven by the iterative walkers instead).
fn leaf_equal(a: &CdtTerm, b: &CdtTerm, budget: usize) -> Result<bool, CdtTypeError> {
    match (a, b) {
        (CdtTerm::Iri(p), CdtTerm::Iri(q)) => Ok(p == q),
        (CdtTerm::Blank(p), CdtTerm::Blank(q)) => Ok(p == q),
        (CdtTerm::Literal(p), CdtTerm::Literal(q)) => literal_equal(p, q, budget),
        (CdtTerm::Null, CdtTerm::Null) => Ok(true),
        // A nested composite and a composite-typed literal are two spellings of one
        // value; see `literal_composite`.
        (CdtTerm::Composite(p), CdtTerm::Literal(q))
        | (CdtTerm::Literal(q), CdtTerm::Composite(p)) => match literal_composite(q) {
            None => Ok(false),
            Some(Err(error)) => Err(error),
            Some(Ok(value)) => value_equal_at(p.as_ref(), &value, spend(budget)?),
        },
        // Nulls are indistinguishable from each other and distinguishable from
        // everything else; different term categories are simply not equal, which is
        // `false` and not a type error.
        _ => Ok(false),
    }
}

/// SPARQL `<` over two elements that are not both composites.
fn leaf_less_than(a: &CdtTerm, b: &CdtTerm) -> Result<bool, CdtTypeError> {
    let unordered = CdtTypeError {
        reason: "SPARQL `<` is not defined for this pair of elements",
    };
    let (CdtTerm::Literal(p), CdtTerm::Literal(q)) = (a, b) else {
        return Err(unordered);
    };
    let (Some(x), Some(y)) = (xsd_value(p), xsd_value(q)) else {
        return Err(unordered);
    };
    match purrdf_xsd::value_cmp(&x, &y) {
        Some(ordering) => Ok(ordering == Ordering::Less),
        None => Err(unordered),
    }
}

/// SPARQL `=` over two elements, walking nested composites and triple terms with an
/// explicit worklist.
///
/// A pair that raises does **not** immediately abandon the walk: a definite `false`
/// found anywhere dominates a type error, exactly as SPARQL's `=` over a sequence
/// does, so the error is only reported if no pair is definitely unequal.
///
/// # Examples
///
/// ```rust
/// use purrdf_cdt::{CdtTerm, term_equal};
///
/// // Nulls are mutually indistinguishable.
/// assert_eq!(term_equal(&CdtTerm::Null, &CdtTerm::Null), Ok(true));
/// // …and distinguishable from everything else.
/// assert_eq!(
///     term_equal(&CdtTerm::Null, &CdtTerm::Iri("http://example.org/a".into())),
///     Ok(false)
/// );
/// ```
pub fn term_equal(a: &CdtTerm, b: &CdtTerm) -> Result<bool, CdtTypeError> {
    equal_worklist(alloc::vec![(a, b)], MAX_NESTING_DEPTH)
}

/// Drive an equality worklist to a verdict. Every pair on the list must be equal for
/// the answer to be `true`.
fn equal_worklist<'a>(
    mut work: Vec<(&'a CdtTerm, &'a CdtTerm)>,
    budget: usize,
) -> Result<bool, CdtTypeError> {
    let mut withheld: Option<CdtTypeError> = None;
    while let Some((x, y)) = work.pop() {
        match (x, y) {
            (CdtTerm::TripleTerm(p), CdtTerm::TripleTerm(q)) => {
                work.push((&p.object, &q.object));
                work.push((&p.predicate, &q.predicate));
                work.push((&p.subject, &q.subject));
            }
            (CdtTerm::Composite(p), CdtTerm::Composite(q)) => {
                match (p.as_ref(), q.as_ref()) {
                    (CdtValue::List(left), CdtValue::List(right)) => {
                        if left.len() != right.len() {
                            return Ok(false);
                        }
                        work.extend(left.iter().zip(right.iter()));
                    }
                    (CdtValue::Map(left), CdtValue::Map(right)) => {
                        if left.len() != right.len() {
                            return Ok(false);
                        }
                        // Map equality needs identical KEY SETS. Both entry
                        // sequences are in key order, so the sets agree exactly when
                        // the sequences of keys agree position by position.
                        if left.iter().zip(right.iter()).any(|(p, q)| p.key != q.key) {
                            return Ok(false);
                        }
                        work.extend(
                            left.iter()
                                .zip(right.iter())
                                .map(|(p, q)| (&p.value, &q.value)),
                        );
                    }
                    // A list is never equal to a map.
                    _ => return Ok(false),
                }
            }
            _ => match leaf_equal(x, y, budget) {
                Ok(true) => {}
                Ok(false) => return Ok(false),
                Err(error) => {
                    if withheld.is_none() {
                        withheld = Some(error);
                    }
                }
            },
        }
    }
    match withheld {
        Some(error) => Err(error),
        None => Ok(true),
    }
}

/// SPARQL `<` over two elements, walking nested composites with an explicit
/// worklist.
pub fn term_less_than(a: &CdtTerm, b: &CdtTerm) -> Result<bool, CdtTypeError> {
    if let (CdtTerm::Composite(p), CdtTerm::Composite(q)) = (a, b) {
        return value_less_than(p.as_ref(), q.as_ref());
    }
    leaf_less_than(a, b)
}

/// The three outcomes one position of a lexicographic walk can produce.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Verdict {
    /// The left side is strictly smaller; the whole comparison is `true`.
    Less,
    /// This position is equal; keep walking.
    Equal,
    /// The left side is not smaller; the whole comparison is `false`.
    NotLess,
}

/// One side of a lexicographic walk.
#[derive(Clone, Copy)]
enum Seq<'a> {
    List(&'a [CdtTerm]),
    Map(&'a [CdtEntry]),
}

impl<'a> Seq<'a> {
    const fn len(self) -> usize {
        match self {
            Self::List(items) => items.len(),
            Self::Map(entries) => entries.len(),
        }
    }

    const fn value(self, index: usize) -> &'a CdtTerm {
        match self {
            Self::List(items) => &items[index],
            Self::Map(entries) => &entries[index].value,
        }
    }

    const fn key(self, index: usize) -> Option<&'a CdtKey> {
        match self {
            Self::List(_) => None,
            Self::Map(entries) => Some(&entries[index].key),
        }
    }
}

/// A frame of the iterative lexicographic walk: two sequences and how far we are.
struct Frame<'a> {
    left: Seq<'a>,
    right: Seq<'a>,
    index: usize,
}

/// The lexicographic `<` walk shared by lists and maps.
///
/// The rules, stated once so both entry points read the same:
///
/// * Positions are visited in order; the walk stops at the first position that is
///   not equal.
/// * Two nulls are equal, so the walk **continues** past them.
/// * At the first unequal position the result is `<` for that pair. If `<` raises
///   there but `=` was cleanly `false`, the answer is `false` — the pair is
///   genuinely different and genuinely unordered, which is not an error about the
///   list.
/// * If `=` itself raises at a position, that error propagates.
/// * When one sequence is a prefix of the other, the shorter one is smaller.
/// * For maps the walk is in key order, and the key is compared before the value: at
///   the first position whose keys differ, the map with the smaller key is smaller.
fn sequence_less_than(left: Seq<'_>, right: Seq<'_>) -> Result<Verdict, CdtTypeError> {
    let mut stack: Vec<Frame<'_>> = Vec::new();
    stack.push(Frame {
        left,
        right,
        index: 0,
    });
    loop {
        let Some(&Frame { left, right, index }) = stack.last() else {
            return Ok(Verdict::Equal);
        };
        let shortest = left.len().min(right.len());
        if index == shortest {
            match left.len().cmp(&right.len()) {
                Ordering::Less => return Ok(Verdict::Less),
                Ordering::Greater => return Ok(Verdict::NotLess),
                Ordering::Equal => {}
            }
            stack.pop();
            match stack.last_mut() {
                None => return Ok(Verdict::Equal),
                Some(parent) => {
                    parent.index += 1;
                    continue;
                }
            }
        }

        if let (Some(left_key), Some(right_key)) = (left.key(index), right.key(index))
            && left_key != right_key
        {
            let left_term = left_key.to_term();
            let right_term = right_key.to_term();
            return Ok(match leaf_less_than(&left_term, &right_term) {
                Ok(true) => Verdict::Less,
                Ok(false) | Err(_) => Verdict::NotLess,
            });
        }

        let (x, y) = (left.value(index), right.value(index));
        if let (CdtTerm::Composite(p), CdtTerm::Composite(q)) = (x, y) {
            match (p.as_ref(), q.as_ref()) {
                (CdtValue::List(a), CdtValue::List(b)) => {
                    stack.push(Frame {
                        left: Seq::List(a),
                        right: Seq::List(b),
                        index: 0,
                    });
                    continue;
                }
                (CdtValue::Map(a), CdtValue::Map(b)) => {
                    stack.push(Frame {
                        left: Seq::Map(a),
                        right: Seq::Map(b),
                        index: 0,
                    });
                    continue;
                }
                // A list and a map are unequal and unordered.
                _ => return Ok(Verdict::NotLess),
            }
        }

        if term_equal(x, y)? {
            stack
                .last_mut()
                .expect("the frame just read is still on the stack")
                .index += 1;
        } else {
            return Ok(match leaf_less_than(x, y) {
                Ok(true) => Verdict::Less,
                Ok(false) | Err(_) => Verdict::NotLess,
            });
        }
    }
}

/// SEP-0009 `cdt:list-equal` over two lists' elements.
///
/// # Examples
///
/// ```rust
/// use purrdf_cdt::{CdtTerm, list_equal};
///
/// // Nulls are mutually indistinguishable, so `[null]` equals `[null]`.
/// assert_eq!(list_equal(&[CdtTerm::Null], &[CdtTerm::Null]), Ok(true));
/// // Different lengths are unequal, never an error.
/// assert_eq!(list_equal(&[CdtTerm::Null], &[]), Ok(false));
/// ```
pub fn list_equal(a: &[CdtTerm], b: &[CdtTerm]) -> Result<bool, CdtTypeError> {
    if a.len() != b.len() {
        return Ok(false);
    }
    equal_worklist(a.iter().zip(b.iter()).collect(), MAX_NESTING_DEPTH)
}

/// SEP-0009 `cdt:map-equal` over two maps' entries.
///
/// The entry sequences must already be in [`total_key_cmp`] order — the invariant
/// [`crate::parse_map`] and [`CdtValue::map`](crate::CdtValue::map) establish. Two
/// maps are equal when their key sets are identical and every shared key's values
/// are equal.
pub fn map_equal(a: &[CdtEntry], b: &[CdtEntry]) -> Result<bool, CdtTypeError> {
    if a.len() != b.len() || a.iter().zip(b.iter()).any(|(p, q)| p.key != q.key) {
        return Ok(false);
    }
    equal_worklist(
        a.iter()
            .zip(b.iter())
            .map(|(p, q)| (&p.value, &q.value))
            .collect(),
        MAX_NESTING_DEPTH,
    )
}

/// SEP-0009 `cdt:list-less-than` over two lists' elements.
///
/// See [`sequence_less_than`] for the exact rules, including what happens when `<`
/// raises at the first unequal position.
///
/// # Examples
///
/// ```rust
/// use purrdf_cdt::{CdtDatatype, CdtValue, list_less_than, parse_list};
///
/// let items = |lexical: &str| match parse_list(lexical).unwrap() {
///     CdtValue::List(items) => items,
///     CdtValue::Map(_) => unreachable!("parse_list yields a list"),
/// };
/// assert_eq!(list_less_than(&items("[1,2]"), &items("[1,3]")), Ok(true));
/// // A shorter list is smaller than any list it is a prefix of.
/// assert_eq!(list_less_than(&items("[1]"), &items("[1,2]")), Ok(true));
/// // Two nulls do not stop the walk.
/// assert_eq!(list_less_than(&items("[null,1]"), &items("[null,2]")), Ok(true));
/// # let _ = CdtDatatype::List;
/// ```
pub fn list_less_than(a: &[CdtTerm], b: &[CdtTerm]) -> Result<bool, CdtTypeError> {
    Ok(sequence_less_than(Seq::List(a), Seq::List(b))? == Verdict::Less)
}

/// SEP-0009 `cdt:map-less-than` over two maps' entries, walking in key order.
pub fn map_less_than(a: &[CdtEntry], b: &[CdtEntry]) -> Result<bool, CdtTypeError> {
    Ok(sequence_less_than(Seq::Map(a), Seq::Map(b))? == Verdict::Less)
}

/// SEP-0009 `=` over two composite values, dispatching on their datatypes. A list is
/// never equal to a map.
pub fn value_equal(a: &CdtValue, b: &CdtValue) -> Result<bool, CdtTypeError> {
    value_equal_at(a, b, MAX_NESTING_DEPTH)
}

/// [`value_equal`], carrying the remaining composite-literal resolution budget (see
/// [`spend`]).
fn value_equal_at(a: &CdtValue, b: &CdtValue, budget: usize) -> Result<bool, CdtTypeError> {
    match (a, b) {
        (CdtValue::List(left), CdtValue::List(right)) => {
            if left.len() != right.len() {
                return Ok(false);
            }
            equal_worklist(left.iter().zip(right.iter()).collect(), budget)
        }
        (CdtValue::Map(left), CdtValue::Map(right)) => {
            if left.len() != right.len()
                || left.iter().zip(right.iter()).any(|(p, q)| p.key != q.key)
            {
                return Ok(false);
            }
            equal_worklist(
                left.iter()
                    .zip(right.iter())
                    .map(|(p, q)| (&p.value, &q.value))
                    .collect(),
                budget,
            )
        }
        _ => Ok(false),
    }
}

/// SEP-0009 `<` over two composite values, dispatching on their datatypes. A list
/// and a map are unordered, which is `false`, not an error.
pub fn value_less_than(a: &CdtValue, b: &CdtValue) -> Result<bool, CdtTypeError> {
    match (a, b) {
        (CdtValue::List(left), CdtValue::List(right)) => list_less_than(left, right),
        (CdtValue::Map(left), CdtValue::Map(right)) => map_less_than(left, right),
        _ => Ok(false),
    }
}

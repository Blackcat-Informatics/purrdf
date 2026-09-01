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
//! ## The three ways a comparison has no answer are kept apart
//!
//! Every refusal carries a [`crate::CdtTypeErrorKind`]. A query cannot tell them
//! apart — SPARQL propagates all three identically — but a validator must, because
//! only one of them is a defect:
//!
//! * an **ill-typed** operand claims a datatype PurRDF models and then carries a
//!   lexical form outside its lexical space, so it denotes nothing in any host;
//! * an **unmodelled** operand is well-formed RDF whose datatype PurRDF has nothing
//!   to say about, so the comparison is undecided rather than wrong;
//! * an **undefined** pair is two terms that both denote perfectly well, with no
//!   relation between them — two IRIs under `<`, a blank node, a `null`.
//!
//! Every literal in this module is resolved through [`crate::parse_literal`], which
//! is the crate's single lexical-to-value choke point and the only thing that knows
//! how to tell those apart. Calling `purrdf_xsd::parse_by_iri` here directly and
//! writing `.ok().flatten()` would collapse the first two into one `None`, which is
//! precisely the distinction [`crate::LiteralValue`] exists to preserve.
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

use crate::error::CdtTypeError;
use crate::limits::MAX_NESTING_DEPTH;
use crate::literal::LiteralValue;
use crate::term::{CdtEntry, CdtKey, CdtLiteral, CdtTerm};
use crate::value::{CdtContents, CdtValue};

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

/// The crate's syntactic total order over two literals: datatype IRI, then lexical
/// form, then language tag, then base direction — each by Unicode scalar order, with
/// `None` before `Some`.
///
/// # The component order is pinned by the corpus, not chosen for looks
///
/// This order is what a map's entries are held and compared in, so SEP-0009's own
/// map-ordering tests decide it. `map-functions/map-less-than-18.rq` requires
/// `{'1'@sv: 41} < {'2'@en: 41}` to be `true`: both keys are `rdf:langString`, so the
/// datatype component ties, and only a **lexical form before language tag** order
/// answers `true` — comparing the tags first would put `sv` after `en` and answer
/// `false`. `map-less-than-19.rq` is the other half: `{'1'@sv: 41} < {'1'@en: 41}` is
/// `false`, which is the language tag deciding once the lexical forms tie.
/// `map-less-than-15.rq` (`'001'` before `'01'`, same datatype) and
/// `map-less-than-17.rq` (`xsd:integer` before `xsd:string`) pin the other two
/// components.
fn literal_cmp(a: &CdtLiteral, b: &CdtLiteral) -> Ordering {
    a.datatype
        .cmp(&b.datatype)
        .then_with(|| a.lexical.cmp(&b.lexical))
        .then_with(|| a.language.cmp(&b.language))
        .then_with(|| a.direction.cmp(&b.direction))
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
    match (a.contents(), b.contents()) {
        (CdtContents::List(_), CdtContents::Map(_)) => jobs.push(CmpJob::Decided(Ordering::Less)),
        (CdtContents::Map(_), CdtContents::List(_)) => {
            jobs.push(CmpJob::Decided(Ordering::Greater));
        }
        (CdtContents::List(left), CdtContents::List(right)) => {
            jobs.push(CmpJob::Decided(left.len().cmp(&right.len())));
            for (p, q) in left.iter().zip(right.iter()).rev() {
                jobs.push(CmpJob::Pair(p, q));
            }
        }
        (CdtContents::Map(left), CdtContents::Map(right)) => {
            jobs.push(CmpJob::Decided(left.len().cmp(&right.len())));
            for (p, q) in left.iter().zip(right.iter()).rev() {
                jobs.push(CmpJob::Pair(&p.value, &q.value));
                jobs.push(CmpJob::KeyPair(&p.key, &q.key));
            }
        }
    }
}

// ── 2. The SEP-0009 value relations ─────────────────────────────────────────────

/// What a composite element's literal denotes.
///
/// Produced by [`denotation`], which is a thin wrapper over [`crate::parse_literal`] —
/// the crate's single lexical-to-value choke point. The four cases are exactly
/// [`crate::LiteralValue`]'s, plus the one this layer has to add: a language-tagged
/// string, whose "value" is the term itself.
///
/// # Why [`Denotation::IllTyped`] and [`Denotation::Unmodelled`] are not one case
///
/// They look alike to a caller that only asks "did it parse?", and collapsing them is
/// the bug this enum exists to prevent. An **unmodelled** literal is well-formed RDF
/// whose datatype PurRDF has nothing to say about, so a comparison with it has no
/// answer — and might have had one, in a host that knows that datatype. An
/// **ill-typed** literal is a defect: it claims a datatype PurRDF *does* model and
/// then carries a lexical form outside that datatype's lexical space, so it denotes
/// nothing anywhere, in any host. Both refuse the comparison, but only the second is
/// something a validator must report, and the refusals therefore carry different
/// [`crate::CdtTypeErrorKind`]s.
enum Denotation {
    /// A `cdt:List` / `cdt:Map` literal whose lexical form parses.
    Composite(CdtValue),
    /// An XSD literal whose lexical form parses.
    Xsd(XsdValue),
    /// A language-tagged string (`rdf:langString`, or RDF 1.2's `rdf:dirLangString`).
    /// Its value space is term identity: two of them are the same value exactly when
    /// they are the same term.
    LanguageTagged,
    /// A datatype PurRDF models, with a lexical form that is not in its lexical space.
    IllTyped,
    /// A datatype outside every value space PurRDF models.
    Unmodelled,
}

/// Resolve a literal through [`crate::parse_literal`].
///
/// This is the **only** place in the crate that turns a `(lexical form, datatype IRI)`
/// pair inside a composite into a value; `parse_literal` in turn is the only place
/// that decides between the XSD and composite value spaces. Reaching for
/// `purrdf_xsd::parse_by_iri` directly here would reintroduce the collapse this
/// module documents against, because that function's `Err` (ill-typed) and `Ok(None)`
/// (unmodelled) are one `None` the moment anyone writes `.ok().flatten()`.
///
/// The language check comes first and is not redundant: the invariant on
/// [`CdtLiteral`] says a language tag implies `rdf:langString` / `rdf:dirLangString`,
/// but this function is total over the type, and a hand-built literal carrying both a
/// language tag and, say, `xsd:integer` is a language-tagged string with a confused
/// datatype rather than an ill-typed integer.
fn denotation(literal: &CdtLiteral) -> Denotation {
    if literal.language.is_some() {
        return Denotation::LanguageTagged;
    }
    match crate::literal::parse_literal(&literal.lexical, &literal.datatype) {
        LiteralValue::Cdt(value) => Denotation::Composite(value),
        LiteralValue::Xsd(value) => Denotation::Xsd(value),
        LiteralValue::IllTyped { .. } => Denotation::IllTyped,
        LiteralValue::Opaque => Denotation::Unmodelled,
    }
}

/// The error an ill-typed operand raises, whichever relation asked.
fn ill_typed() -> CdtTypeError {
    CdtTypeError::ill_typed(
        "a literal whose datatype PurRDF models and whose lexical form is not in that \
         datatype's lexical space denotes nothing, so no comparison with it has an answer",
    )
}

/// The error an unmodelled operand raises, whichever relation asked.
fn unmodelled() -> CdtTypeError {
    CdtTypeError::unmodelled(
        "a literal whose datatype is outside every value space PurRDF models may or may \
         not denote the same value as this one, so the comparison has no answer",
    )
}

/// A composite value reached either by borrowing a nested one or by parsing a
/// `cdt:`-typed literal.
///
/// The borrowed case is what keeps the common walk allocation-free; the owned case is
/// unavoidable, because a value parsed out of a literal's lexical form has no home to
/// be borrowed from.
enum MaybeOwned<'a> {
    Borrowed(&'a CdtValue),
    Owned(CdtValue),
}

impl MaybeOwned<'_> {
    fn get(&self) -> &CdtValue {
        match self {
            Self::Borrowed(value) => value,
            Self::Owned(value) => value,
        }
    }
}

/// The composite an element denotes, in either of its two spellings, or `None` when it
/// denotes no composite at all.
///
/// # Why an element that *is* a literal can still be a composite
///
/// SEP-0009 admits one and the same composite element written two ways inside a
/// lexical form: as the grammar's own nested `List` / `Map` production, or as an
/// ordinary `RDFLiteral` carrying the composite datatype. The corpus writes the same
/// test both ways and demands the same answer — `list-functions/contains-07.rq` nests
/// `[2]` while `contains-08.rq` writes `'[2]'^^cdt:List`, and both must find it;
/// `contains-09.rq` and `contains-10.rq` do the same for a map. So the two spellings
/// denote the same **value**, and the value relations have to see through the literal
/// one.
///
/// Term identity does not, and must not: `list-functions/sameterm-03.rq` requires two
/// spellings of one value to be different *terms*. That is exactly why the literal
/// keeps its lexical form ([`CdtLiteral`] is verbatim), why map keys are still
/// distinguished lexically, and why this resolution lives here in the value relations
/// and nowhere else.
///
/// An **ill-typed** `cdt:`-typed literal answers `None` here on purpose: it denotes no
/// composite, and the leaf rules then raise [`CdtTypeErrorKind::IllTyped`](crate::CdtTypeErrorKind::IllTyped)
/// for it rather than reporting an inequality.
fn as_composite(term: &CdtTerm) -> Option<MaybeOwned<'_>> {
    match term {
        CdtTerm::Composite(inner) => Some(MaybeOwned::Borrowed(inner.as_ref())),
        CdtTerm::Literal(literal) => match denotation(literal) {
            Denotation::Composite(value) => Some(MaybeOwned::Owned(value)),
            Denotation::Xsd(_)
            | Denotation::LanguageTagged
            | Denotation::IllTyped
            | Denotation::Unmodelled => None,
        },
        CdtTerm::Iri(_) | CdtTerm::Blank(_) | CdtTerm::TripleTerm(_) | CdtTerm::Null => None,
    }
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
    budget.checked_sub(1).ok_or_else(|| {
        CdtTypeError::undefined(
            "cdt:List / cdt:Map literals nested deeper than the composite nesting bound",
        )
    })
}

/// SPARQL `=` over two literals.
///
/// Same term is always `true`; after that the answer is a function of what the two
/// literals **denote**, which is what [`denotation`] reports:
///
/// * either side ill-typed — a type error, whatever the other side is. An ill-typed
///   literal denotes nothing, so there is no value to be equal or unequal to.
///   `list-functions/list-less-than-error-03.rq` pins the analogous outcome for `<`
///   on a whole `"1"^^cdt:List` operand.
/// * both composites — compared as composites (`list-functions/contains-08.rq`).
/// * a composite against an XSD value or a language-tagged string — `false`. Those
///   are three value spaces PurRDF models in full and they are pairwise disjoint, so
///   this is SPARQL's "known to be different" rather than a refusal.
///   `list-functions/contains-07.rq` requires `cdt:contains("[1,[2]]", 2)` to be
///   `false`, which is exactly this pair.
/// * anything against an **unmodelled** datatype — a type error. This is the case the
///   tri-state exists for: PurRDF cannot know whether that datatype's value space
///   overlaps this one, so `false` would be a claim it has no grounds for.
/// * a language-tagged string against an XSD value, or against another
///   language-tagged string that is not the same term — `false`.
///   `list-functions/contains-03.rq` requires a list holding `'b'@en` to answer
///   `false`, not an error, when asked for the plain string `'b'`.
/// * two XSD values — `purrdf_xsd::value_eq`, which is definite in both directions.
fn literal_equal(a: &CdtLiteral, b: &CdtLiteral, budget: usize) -> Result<bool, CdtTypeError> {
    if a == b {
        return Ok(true);
    }
    match (denotation(a), denotation(b)) {
        (Denotation::IllTyped, _) | (_, Denotation::IllTyped) => Err(ill_typed()),
        (Denotation::Composite(left), Denotation::Composite(right)) => {
            value_equal_at(&left, &right, spend(budget)?)
        }
        (Denotation::Unmodelled, _) | (_, Denotation::Unmodelled) => Err(unmodelled()),
        (Denotation::Composite(_), _) | (_, Denotation::Composite(_)) => Ok(false),
        (Denotation::LanguageTagged, _) | (_, Denotation::LanguageTagged) => Ok(false),
        (Denotation::Xsd(x), Denotation::Xsd(y)) => Ok(purrdf_xsd::value_eq(&x, &y)),
    }
}

/// SPARQL `=` over two elements that are not both composites and not both triple
/// terms (those two cases are driven by the iterative walkers instead).
///
/// # Two blank nodes are equal when they are the same node, and undecidable otherwise
///
/// `list-functions/list-equals-07.rq` compares `"[   _:b   ]"^^cdt:List` with
/// `"[_:b]"^^cdt:List` and requires `true`, and `list-equals-09.rq` requires the same
/// of one `BNODE()` compared with itself — so the same blank node is the same value.
/// But `list-equals-06.rq` compares `_:b1` with `_:b2` and `list-equals-08.rq`
/// compares two distinct `BNODE()`s, and **both require the result to be unbound**,
/// not `false`. SEP-0009 therefore treats two distinct blank nodes the way it treats
/// two unknowns: they might denote the same resource, so equality has no answer. This
/// is narrower than SPARQL's own `RDFterm-equal`, which answers `false` for any two
/// terms that are not both literals, and the corpus is the reason for the narrowing.
///
/// [`membership_equal`] is where the distinction stops applying — see there.
fn leaf_equal(a: &CdtTerm, b: &CdtTerm, budget: usize) -> Result<bool, CdtTypeError> {
    match (a, b) {
        (CdtTerm::Iri(p), CdtTerm::Iri(q)) => Ok(p == q),
        (CdtTerm::Blank(p), CdtTerm::Blank(q)) => {
            if p == q {
                Ok(true)
            } else {
                Err(CdtTypeError::undefined(
                    "two distinct blank nodes may or may not denote the same resource, so \
                     SEP-0009 equality has no answer for them",
                ))
            }
        }
        (CdtTerm::Literal(p), CdtTerm::Literal(q)) => literal_equal(p, q, budget),
        (CdtTerm::Null, CdtTerm::Null) => Ok(true),
        // A nested composite and a composite-typed literal are two spellings of one
        // value; see `as_composite`.
        (CdtTerm::Composite(p), CdtTerm::Literal(q))
        | (CdtTerm::Literal(q), CdtTerm::Composite(p)) => match denotation(q) {
            Denotation::Composite(value) => value_equal_at(p.as_ref(), &value, spend(budget)?),
            Denotation::IllTyped => Err(ill_typed()),
            Denotation::Unmodelled => Err(unmodelled()),
            Denotation::Xsd(_) | Denotation::LanguageTagged => Ok(false),
        },
        // Nulls are indistinguishable from each other and distinguishable from
        // everything else; different term categories are simply not equal, which is
        // `false` and not a type error.
        _ => Ok(false),
    }
}

/// SPARQL `<` over two elements that are not both composites.
///
/// The refusal is typed, so a consumer can tell "this data is broken" from "this
/// relation has nothing to say": an ill-typed operand is
/// [`CdtTypeErrorKind::IllTyped`](crate::CdtTypeErrorKind::IllTyped), an unmodelled
/// datatype is [`CdtTypeErrorKind::Unmodelled`](crate::CdtTypeErrorKind::Unmodelled),
/// and a pair that simply has no order — two IRIs, a `null`, a blank node, a `NaN`
/// against a number — is
/// [`CdtTypeErrorKind::Undefined`](crate::CdtTypeErrorKind::Undefined).
/// `map-functions/map-less-than-error-01.rq` pins the IRI case and
/// `map-less-than-null-01.rq` the `null` case: both require the whole comparison to be
/// unbound.
fn leaf_less_than(a: &CdtTerm, b: &CdtTerm) -> Result<bool, CdtTypeError> {
    let unordered =
        || CdtTypeError::undefined("SPARQL `<` is not defined for this pair of elements");
    let (CdtTerm::Literal(p), CdtTerm::Literal(q)) = (a, b) else {
        return Err(unordered());
    };
    match (denotation(p), denotation(q)) {
        (Denotation::IllTyped, _) | (_, Denotation::IllTyped) => Err(ill_typed()),
        (Denotation::Unmodelled, _) | (_, Denotation::Unmodelled) => Err(unmodelled()),
        (Denotation::Xsd(x), Denotation::Xsd(y)) => match purrdf_xsd::value_cmp(&x, &y) {
            Some(ordering) => Ok(ordering == Ordering::Less),
            None => Err(unordered()),
        },
        _ => Err(unordered()),
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
    term_equal_at(a, b, MAX_NESTING_DEPTH)
}

/// [`term_equal`], carrying the remaining composite-literal resolution budget.
fn term_equal_at(a: &CdtTerm, b: &CdtTerm, budget: usize) -> Result<bool, CdtTypeError> {
    equal_worklist(alloc::vec![(a, b)], budget)
}

/// `cdt:contains`'s membership test: [`term_equal`], except that two blank nodes are
/// compared by **identity** and therefore always have an answer.
///
/// `list-functions/contains-05.rq` searches `"[_:b,null,'_:b']"^^cdt:List` for a fresh
/// `BNODE()` and requires the result to be **bound** and `false` — so an unrelated
/// blank node in the list is a definite miss, not the undecidable comparison
/// [`leaf_equal`] reports for `=`. `contains-06.rq` is the other half: the very term
/// `cdt:head` just returned from that list **is** found. Membership asks "is this term
/// in the list?", which for a blank node is a question about the term; `=` asks "are
/// these the same value?", which for two unknowns has no answer.
///
/// The distinction applies to the top-level pair only. A blank node buried inside two
/// nested composites is compared by [`term_equal`]'s rule, because at that depth the
/// question is again one of value equality; no corpus test reaches that case.
pub(crate) fn membership_equal(item: &CdtTerm, term: &CdtTerm) -> Result<bool, CdtTypeError> {
    if let (CdtTerm::Blank(p), CdtTerm::Blank(q)) = (item, term) {
        return Ok(p == q);
    }
    term_equal(item, term)
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
                match (p.contents(), q.contents()) {
                    (CdtContents::List(left), CdtContents::List(right)) => {
                        if left.len() != right.len() {
                            return Ok(false);
                        }
                        work.extend(left.iter().zip(right.iter()));
                    }
                    (CdtContents::Map(left), CdtContents::Map(right)) => {
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

/// SPARQL `<` over two elements, seeing through both spellings of a composite.
pub fn term_less_than(a: &CdtTerm, b: &CdtTerm) -> Result<bool, CdtTypeError> {
    Ok(term_verdict(a, b, MAX_NESTING_DEPTH)? == Verdict::Less)
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

    fn of(value: &'a CdtValue) -> Self {
        match value.contents() {
            CdtContents::List(items) => Self::List(items),
            CdtContents::Map(entries) => Self::Map(entries),
        }
    }

    const fn is_map(self) -> bool {
        matches!(self, Self::Map(_))
    }
}

/// A frame of the iterative lexicographic walk: two sequences and how far we are.
struct Frame<'a> {
    left: Seq<'a>,
    right: Seq<'a>,
    index: usize,
}

/// `<` over two whole composite values.
fn value_verdict(a: &CdtValue, b: &CdtValue, budget: usize) -> Result<Verdict, CdtTypeError> {
    let (left, right) = (Seq::of(a), Seq::of(b));
    if left.is_map() != right.is_map() {
        return Err(CdtTypeError::undefined(
            "SPARQL `<` is not defined between a cdt:List and a cdt:Map",
        ));
    }
    sequence_less_than(left, right, budget)
}

/// `<` over two elements, at one position of a walk.
fn term_verdict(x: &CdtTerm, y: &CdtTerm, budget: usize) -> Result<Verdict, CdtTypeError> {
    if matches!(x, CdtTerm::Blank(_)) || matches!(y, CdtTerm::Blank(_)) {
        return Err(CdtTypeError::undefined(
            "SPARQL `<` has no answer where a blank node stands, not even against the very \
             same blank node",
        ));
    }
    if let (Some(p), Some(q)) = (as_composite(x), as_composite(y)) {
        return value_verdict(p.get(), q.get(), spend(budget)?);
    }
    if term_equal_at(x, y, budget)? {
        return Ok(Verdict::Equal);
    }
    Ok(if leaf_less_than(x, y)? {
        Verdict::Less
    } else {
        Verdict::NotLess
    })
}

/// The lexicographic `<` walk shared by lists and maps.
///
/// The rules, stated once so both entry points read the same, each with the corpus
/// test that pins it:
///
/// * Positions are visited in order, and the walk stops at the first position that is
///   not equal — `list-functions/list-less-than-07.rq` (`[1,2] < [1,3]` is `true`) and
///   `list-less-than-11.rq` (`[1,1,2] < [1,2,3]`, where position 1 decides).
/// * Two nulls are equal, so the walk **continues** past them:
///   `list-less-than-null-03.rq` (`[null] < [null]` is `false`, so the walk reached the
///   length tie-break) and `list-less-than-null-05.rq` (`[1,null] < [2,null]`).
/// * When one sequence is a prefix of the other the shorter one is smaller —
///   `list-less-than-09.rq` and `map-less-than-12.rq` — and this tie-break is reached
///   **before** any element of the longer sequence is examined, which is why
///   `list-less-than-26.rq` can answer `[] < [_:b]` with a clean `true` even though a
///   blank node has no order.
/// * A **blank node** at a visited position has no answer, even against itself:
///   `list-less-than-27.rq` (two distinct `BNODE()`s), `list-less-than-28.rq`
///   (`[_:b] < [_:b]`, the same label on both sides) and `list-less-than-29.rq` (one
///   `BNODE()` compared with itself) all require the result to be unbound. Note the
///   contrast with `list-less-than-31.rq`, where two *equal IRIs* are simply equal and
///   the walk carries on to a clean `false`: an IRI is a term you can know, and a blank
///   node is not.
/// * At the first unequal position the whole comparison is `<` for that pair, **and
///   that pair's error propagates**: `map-functions/map-less-than-error-01.rq` (two
///   different IRIs as map values), `map-less-than-error-02.rq` (an IRI against a
///   number) and `map-less-than-null-01.rq` (`{1:null} < {1:42}`) each require the
///   result to be unbound rather than `false`. That is the one place where `<` and `=`
///   disagree about the very same pair: `map-equals-null-02.rq` requires
///   `{1:null} = {1:44}` to be a bound `false` while `map-less-than-null-01.rq`
///   requires `{1:null} < {1:42}` to be unbound.
/// * If `=` itself raises at a position, that error propagates too.
/// * For maps the walk is in key order and the key is compared **syntactically**, with
///   [`total_key_cmp`], not with SPARQL `<`. `map-less-than-15.rq` is the test that
///   forces this: `{'001'^^xsd:integer: 41} < {'01'^^xsd:integer: 41}` must be `true`,
///   and those two keys are the *same integer value*, so a value comparison could only
///   have answered `false`. `map-less-than-17.rq` (an `xsd:integer` key against an
///   `xsd:string` key), `map-less-than-20.rq` (a literal key against an IRI key) and
///   `map-less-than-21.rq` (two IRI keys) each demand an answer where SPARQL `<` has
///   none, and each agrees with [`total_key_cmp`].
///
/// # `<=` is not `<` or `=`
///
/// A consumer must compute `a <= b` as "if `<` raised, raise; else if `<` was `true`,
/// `true`; else `a = b`" — **not** as `(a < b) || (a = b)`. `list-less-equal-28.rq`
/// requires `"[_:b]"^^cdt:List <= "[_:b]"^^cdt:List` to be unbound while
/// `list-equals-07.rq` requires the same two operands to be `=`-equal, so SPARQL's
/// `||`, under which `error || true` is `true`, would give the wrong answer.
/// `list-greater-equal-28.rq` says the same for `>=`.
fn sequence_less_than(
    left: Seq<'_>,
    right: Seq<'_>,
    budget: usize,
) -> Result<Verdict, CdtTypeError> {
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

        if let (Some(left_key), Some(right_key)) = (left.key(index), right.key(index)) {
            match total_key_cmp(left_key, right_key) {
                Ordering::Less => return Ok(Verdict::Less),
                Ordering::Greater => return Ok(Verdict::NotLess),
                Ordering::Equal => {}
            }
        }

        let (x, y) = (left.value(index), right.value(index));
        // Two nested composites keep the walk iterative: a frame costs heap, and the
        // depth of this stack is the depth of the values, which is bounded.
        if let (CdtTerm::Composite(p), CdtTerm::Composite(q)) = (x, y) {
            let (inner_left, inner_right) = (Seq::of(p.as_ref()), Seq::of(q.as_ref()));
            if inner_left.is_map() != inner_right.is_map() {
                return Err(CdtTypeError::undefined(
                    "SPARQL `<` is not defined between a cdt:List and a cdt:Map",
                ));
            }
            stack.push(Frame {
                left: inner_left,
                right: inner_right,
                index: 0,
            });
            continue;
        }

        match term_verdict(x, y, budget)? {
            Verdict::Equal => {
                stack
                    .last_mut()
                    .expect("the frame just read is still on the stack")
                    .index += 1;
            }
            decided => return Ok(decided),
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
/// let items = |lexical: &str| {
///     parse_list(lexical)
///         .unwrap()
///         .into_list()
///         .expect("parse_list yields a list")
/// };
/// assert_eq!(list_less_than(&items("[1,2]"), &items("[1,3]")), Ok(true));
/// // A shorter list is smaller than any list it is a prefix of.
/// assert_eq!(list_less_than(&items("[1]"), &items("[1,2]")), Ok(true));
/// // Two nulls do not stop the walk.
/// assert_eq!(list_less_than(&items("[null,1]"), &items("[null,2]")), Ok(true));
/// # let _ = CdtDatatype::List;
/// ```
pub fn list_less_than(a: &[CdtTerm], b: &[CdtTerm]) -> Result<bool, CdtTypeError> {
    Ok(sequence_less_than(Seq::List(a), Seq::List(b), MAX_NESTING_DEPTH)? == Verdict::Less)
}

/// SEP-0009 `cdt:map-less-than` over two maps' entries, walking in key order.
pub fn map_less_than(a: &[CdtEntry], b: &[CdtEntry]) -> Result<bool, CdtTypeError> {
    Ok(sequence_less_than(Seq::Map(a), Seq::Map(b), MAX_NESTING_DEPTH)? == Verdict::Less)
}

/// SEP-0009 `=` over two composite values, dispatching on their datatypes. A list is
/// never equal to a map.
pub fn value_equal(a: &CdtValue, b: &CdtValue) -> Result<bool, CdtTypeError> {
    value_equal_at(a, b, MAX_NESTING_DEPTH)
}

/// [`value_equal`], carrying the remaining composite-literal resolution budget (see
/// [`spend`]).
fn value_equal_at(a: &CdtValue, b: &CdtValue, budget: usize) -> Result<bool, CdtTypeError> {
    match (a.contents(), b.contents()) {
        (CdtContents::List(left), CdtContents::List(right)) => {
            if left.len() != right.len() {
                return Ok(false);
            }
            equal_worklist(left.iter().zip(right.iter()).collect(), budget)
        }
        (CdtContents::Map(left), CdtContents::Map(right)) => {
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

/// SEP-0009 `<` over two composite values, dispatching on their datatypes.
///
/// A `cdt:List` and a `cdt:Map` are two disjoint value spaces with no order between
/// them, so this **raises** rather than answering `false`. **The corpus does not
/// exercise the mixed pair**; the choice follows the pairs it does exercise, where
/// every genuinely unordered pair at a compared position — two IRIs
/// (`map-functions/map-less-than-error-01.rq`), an IRI against a number
/// (`map-less-than-error-02.rq`), a `null` against a term (`map-less-than-null-01.rq`)
/// — is required to be unbound rather than `false`. Answering `false` here would also
/// make `<` and `>` both `false` for a pair that is not equal either, which is a claim
/// about an order that does not exist.
pub fn value_less_than(a: &CdtValue, b: &CdtValue) -> Result<bool, CdtTypeError> {
    Ok(value_verdict(a, b, MAX_NESTING_DEPTH)? == Verdict::Less)
}

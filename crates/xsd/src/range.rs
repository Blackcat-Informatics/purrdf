// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Datatype-range satisfiability — **is this OWL 2 data range empty?**
//!
//! An OWL 2 data range is built over XSD datatypes with constraining facets, plus
//! enumeration, complement, intersection and union. A description-logic reasoner asks
//! three questions about such a range: is it **empty** ([`satisfiability`]), does it
//! **hold a given value** ([`contains`]), and does it hold **at least `n` distinct
//! values** ([`cardinality`]). This module answers all three over the XSD value
//! spaces [`crate`] models, using no arithmetic tower beyond the one already here.
//!
//! # Three-valued by construction
//!
//! [`Satisfiability::Empty`] and [`Satisfiability::Inhabited`] are **proofs**: an
//! `Empty` answer proves no value satisfies the range; an `Inhabited` answer rests on
//! an exhibited witness. Whatever cannot be proved either way is
//! [`Satisfiability::Undecided`]. The asymmetry is load-bearing: a consuming reasoner
//! reads `Empty` as "this ontology is inconsistent", so a wrong `Empty` is the one
//! unsound answer, and every widening in this module leans away from it.
//! [`is_exactly_decided`] reports whether a range is answered exactly, so a consumer
//! can raise a reported boundary instead of guessing.
//!
//! # The data domain is a disjoint union of value spaces
//!
//! OWL 2's datatype map makes the primitive value spaces pairwise disjoint, so a range
//! is represented space by space:
//!
//! | space | datatypes | structure |
//! |---|---|---|
//! | decimal | `xsd:decimal` + the 13 integer-family types | two strata: the integers (successor `+1`) and the non-integral decimals (dense) |
//! | float | `xsd:float` | two strata: the numbers (successor `next_up`) and the single NaN |
//! | double | `xsd:double` | as float, at double width |
//! | boolean | `xsd:boolean` | the two-element set |
//! | string | `xsd:string` | length-selected, length in CHARACTERS |
//! | hex / base64 | `xsd:hexBinary`, `xsd:base64Binary` | length-selected, length in OCTETS |
//! | one space each | `dateTime` (its zoned `dateTimeStamp` subset and timezone-less remainder), `date`, `time`, the `duration` family, and the five Gregorian types | listed sets and their complements |
//! | two term-identified spaces | `rdf:langString`, `rdf:dirLangString` | disjoint infinite sets; finite members are normalized identities supplied by the consumer |
//!
//! The `duration` family is ONE space because the `xsd:dayTimeDuration` and
//! `xsd:yearMonthDuration` value spaces overlap at the zero duration.
//!
//! Beyond those spaces lies the **remainder**: the values of datatypes this module does
//! not model (`owl:real`, `owl:rational`, `xsd:anyURI`, a
//! user-defined datatype). [`DataRange::Any`] holds the remainder, a modelled
//! [`DataRange::Datatype`] does not, and [`DataRange::Opaque`] leaves it — and every
//! modelled space — unknown, because an unmodelled value space may overlap a modelled
//! one (`owl:real` contains every `xsd:decimal` value), so assuming disjointness there
//! would be unsound.
//!
//! # Two identities, again
//!
//! [`same_value`] is XSD/OWL 2 **value-space identity**, not [`crate::value_eq`]
//! (SPARQL `=`). The two differ in exactly two places, both load-bearing:
//! `"NaN"^^xsd:double` is one value identical to itself (`value_eq` answers `false`),
//! and identity holds only WITHIN one value space, so `"5"^^xsd:float`,
//! `"5"^^xsd:double` and `"5"^^xsd:decimal` are three different values (`value_eq`
//! promotes across all of them). `"5"^^xsd:integer` and `"5.0"^^xsd:decimal` remain
//! one value: the integer value space is a subset of the decimal value space.
//!
//! # Residue — the ranges that answer `Undecided`
//!
//! * [`DataRange::Opaque`], and any intersection or complement involving it. A union
//!   with an inhabited operand still answers `Inhabited`: the witness stands whatever
//!   the opaque operand denotes.
//! * A **bound facet over a temporal space** that neither contradicts another bound nor
//!   exhibits an inclusive endpoint, and the **complement** of any temporal bound
//!   restriction. The XSD order on these spaces is partial — a timezone-less
//!   `dateTime` is incomparable with one whose offset falls in the 14-hour
//!   indeterminacy window, and `xsd:duration` has a genuinely two-component partial
//!   order — so an interval's complement there is not a union of intervals. Temporal
//!   enumerations and their complements ARE decided exactly.
//! * `xsd:dayTimeDuration` and `xsd:yearMonthDuration` as whole datatypes: each is an
//!   infinite proper subspace of the shared duration space that listed sets cannot
//!   express. Inhabited (the zero duration witnesses both), never exact.
//! * An **enumerated `xsd:float`/`xsd:double` zero**. The interval algebra works in the
//!   order these spaces carry, where `positiveZero` and `negativeZero` are one point,
//!   while OWL 2's value space holds both; an enumeration that names one zero is
//!   therefore exhibited, not exactly represented.
//! * A facet **inapplicable to its base's space** — a bound on `xsd:string`, a length
//!   on `xsd:integer`, any facet on `xsd:boolean` (XSD gives it no ordering or length
//!   facet), a bound whose value comes from another space, or a NaN bound. Silently
//!   ignoring such a facet would be unsound under a complement, so the space is left
//!   unknown instead.
//!
//! # Examples
//!
//! ```rust
//! use purrdf_xsd::range::{DataRange, Facet, Satisfiability, satisfiability};
//! use purrdf_xsd::{XsdDatatype, parse};
//!
//! // No integer is at once >= 5 and <= 3.
//! let empty = DataRange::Restriction {
//!     base: XsdDatatype::Integer,
//!     facets: vec![
//!         Facet::MinInclusive(parse("5", XsdDatatype::Integer)?),
//!         Facet::MaxInclusive(parse("3", XsdDatatype::Integer)?),
//!     ],
//! };
//! assert_eq!(satisfiability(&empty), Satisfiability::Empty);
//!
//! // No integer lies strictly between 3 and 4 — but a decimal does.
//! let gap = |base| DataRange::Restriction {
//!     base,
//!     facets: vec![
//!         Facet::MinExclusive(parse("3", XsdDatatype::Integer).expect("valid")),
//!         Facet::MaxExclusive(parse("4", XsdDatatype::Integer).expect("valid")),
//!     ],
//! };
//! assert_eq!(satisfiability(&gap(XsdDatatype::Integer)), Satisfiability::Empty);
//! assert_eq!(
//!     satisfiability(&gap(XsdDatatype::Decimal)),
//!     Satisfiability::Inhabited
//! );
//! # Ok::<(), purrdf_xsd::XsdError>(())
//! ```

use purrdf_lex::walk::VecReserve;
use std::cmp::Ordering;

use crate::datatype::XsdDatatype;
use crate::exact;
use crate::numeric::Decimal;
use crate::ops::{value_cmp, value_eq};
use crate::temporal::Time;
use crate::value::XsdValue;

mod storage;
use storage::{CloneOwned, Memory, OwnedBytes};
pub use storage::{Storage, StorageError};

fn resident<T>(owned: usize, body: impl FnOnce(&mut Memory<'_>) -> Result<T, StorageError>) -> T {
    let mut storage = storage::Resident;
    let mut memory = Memory::with_resident_input(&mut storage, owned);
    body(&mut memory).expect("resident range storage")
}

// ── Public surface ───────────────────────────────────────────────────────────────

/// A constraining facet, as OWL 2 admits it on a datatype restriction
/// (`owl:onDatatype` + `owl:withRestrictions`).
///
/// A facet that does not apply to its base datatype's value space constrains nothing
/// coherently and leaves that space unknown rather than being ignored.
#[derive(Debug, Clone)]
pub enum Facet {
    /// `xsd:minInclusive` — the value is greater than or equal to this bound.
    MinInclusive(XsdValue),
    /// `xsd:maxInclusive` — the value is less than or equal to this bound.
    MaxInclusive(XsdValue),
    /// `xsd:minExclusive` — the value is strictly greater than this bound.
    MinExclusive(XsdValue),
    /// `xsd:maxExclusive` — the value is strictly less than this bound.
    MaxExclusive(XsdValue),
    /// `xsd:length` — the value's length equals this count (characters for
    /// `xsd:string`, octets for the two binary datatypes).
    Length(u64),
    /// `xsd:minLength` — the value's length is at least this count.
    MinLength(u64),
    /// `xsd:maxLength` — the value's length is at most this count.
    MaxLength(u64),
}

/// An OWL 2 data range over the XSD value spaces.
#[derive(Debug, Clone)]
pub enum DataRange {
    /// `rdfs:Literal` — the whole data domain, remainder included.
    Any,
    /// A whole datatype value space.
    Datatype(XsdDatatype),
    /// An RDF language-string value space whose normalized identity is supplied by
    /// the term carrier; no RDF grammar or datatype dependency enters this crate.
    TermDatatype(TermSpace),
    /// A finite set of normalized term identities in one RDF value space. Identities
    /// must come from one immutable identity context, with equal values sharing an id.
    TermOneOf {
        /// The RDF value space the identities denote.
        space: TermSpace,
        /// Pairwise value identities; duplicates are removed by the shared set algebra.
        values: Vec<u32>,
    },
    /// `owl:onDatatype` + `owl:withRestrictions`.
    Restriction {
        /// The datatype being restricted.
        base: XsdDatatype,
        /// The constraining facets, applied conjunctively.
        facets: Vec<Facet>,
    },
    /// `DataOneOf` — an explicit finite set of values.
    OneOf(Vec<XsdValue>),
    /// `owl:datatypeComplementOf` — the data domain minus the operand.
    Not(Box<Self>),
    /// `DataIntersectionOf`. An empty operand list is the whole data domain (the
    /// identity of intersection).
    And(Vec<Self>),
    /// `DataUnionOf`. An empty operand list is the empty range (the identity of
    /// union).
    Or(Vec<Self>),
    /// A range this module models nothing about: a datatype outside the XSD value
    /// spaces, an `xsd:pattern` or `rdf:langRange` facet, or an n-ary data range.
    Opaque,
}

/// The two disjoint RDF language-string value spaces.
///
/// The caller's term carrier validates tags, directions and lexical identity. This
/// range algebra only needs that equality, and never interprets an arbitrary id as an
/// XSD scalar. Both spaces are infinite and disjoint from every XSD value space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermSpace {
    /// Pairs of a string and normalized language tag (`rdf:langString`).
    LangString,
    /// Triples of a string, normalized language tag and base direction (`rdf:dirLangString`).
    DirLangString,
}

/// Whether a data range is empty. `Empty` and `Inhabited` are PROVED; `Undecided`
/// means this module cannot say.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Satisfiability {
    /// Proved: no value satisfies the range.
    Empty,
    /// Proved: a witness value satisfies the range.
    Inhabited,
    /// Neither emptiness nor inhabitation is proved.
    Undecided,
}

/// How many distinct values a data range holds.
///
/// `Exactly` is a proof of the count. `AtLeast` and `Unbounded` are lower bounds — a
/// consumer can refute `>= n r.DR` only against `Exactly`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cardinality {
    /// Exactly this many distinct values.
    Exactly(u64),
    /// At least this many distinct values, possibly more.
    AtLeast(u64),
    /// Infinitely many values.
    Unbounded,
    /// The count is not determined.
    Undecided,
}

/// A three-valued answer: `Yes`/`No` are proved, `Unknown` is not determined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Known {
    /// Proved to hold.
    Yes,
    /// Proved not to hold.
    No,
    /// Not determined.
    Unknown,
}

/// Whether `range` is empty, inhabited, or beyond this module's reach.
///
/// # Examples
///
/// ```rust
/// use purrdf_xsd::XsdDatatype;
/// use purrdf_xsd::range::{DataRange, Satisfiability, satisfiability};
///
/// // The float and double value spaces are disjoint in OWL 2's datatype map.
/// let both = DataRange::And(vec![
///     DataRange::Datatype(XsdDatatype::Float),
///     DataRange::Datatype(XsdDatatype::Double),
/// ]);
/// assert_eq!(satisfiability(&both), Satisfiability::Empty);
///
/// // An opaque operand cannot defeat an exhibited witness under union.
/// let witnessed = DataRange::Or(vec![
///     DataRange::Datatype(XsdDatatype::Integer),
///     DataRange::Opaque,
/// ]);
/// assert_eq!(satisfiability(&witnessed), Satisfiability::Inhabited);
/// ```
#[must_use]
pub fn satisfiability(range: &DataRange) -> Satisfiability {
    extent(range).satisfiability()
}

/// Whether `range` holds `value`.
///
/// # Examples
///
/// ```rust
/// use purrdf_xsd::range::{DataRange, Known, contains};
/// use purrdf_xsd::{XsdDatatype, parse};
///
/// let five = parse("5", XsdDatatype::Integer)?;
/// let integers = DataRange::Datatype(XsdDatatype::Integer);
/// assert_eq!(contains(&integers, &five), Known::Yes);
///
/// // Value-space identity, not term identity: "05" denotes the same value as "5".
/// let listed = DataRange::OneOf(vec![parse("05", XsdDatatype::Integer)?]);
/// assert_eq!(contains(&listed, &five), Known::Yes);
///
/// // A float 5 is NOT a decimal 5: the value spaces are disjoint.
/// assert_eq!(
///     contains(&integers, &parse("5", XsdDatatype::Float)?),
///     Known::No
/// );
/// # Ok::<(), purrdf_xsd::XsdError>(())
/// ```
#[must_use]
pub fn contains(range: &DataRange, value: &XsdValue) -> Known {
    extent(range).contains(value)
}

/// Whether a range holds a normalized RDF language-string value identity.
/// The identity context must be the same as that of its [`DataRange::TermOneOf`] operands.
#[must_use]
pub fn contains_term(range: &DataRange, space: TermSpace, identity: u32) -> Known {
    extent(range).term[space as usize].holds(&identity)
}

/// How many distinct values `range` holds.
///
/// # Examples
///
/// ```rust
/// use purrdf_xsd::range::{Cardinality, DataRange, cardinality};
/// use purrdf_xsd::{XsdDatatype, parse};
///
/// // One value, two lexical forms.
/// let one = DataRange::OneOf(vec![
///     parse("1", XsdDatatype::Integer)?,
///     parse("01", XsdDatatype::Integer)?,
/// ]);
/// assert_eq!(cardinality(&one), Cardinality::Exactly(1));
///
/// // Two values: the float and double value spaces are disjoint.
/// let two = DataRange::OneOf(vec![
///     parse("5", XsdDatatype::Float)?,
///     parse("5", XsdDatatype::Double)?,
/// ]);
/// assert_eq!(cardinality(&two), Cardinality::Exactly(2));
/// # Ok::<(), purrdf_xsd::XsdError>(())
/// ```
#[must_use]
pub fn cardinality(range: &DataRange) -> Cardinality {
    extent(range).cardinality()
}

/// Count a range through the same native algebra with before-growth admission.
/// The borrowed input remains caller-owned; every construction product is gone
/// before the operation releases its storage total.
/// # Errors
/// Returns the original admission or physical allocation refusal, never `Undecided`.
pub fn try_cardinality(
    range: &DataRange,
    storage: &mut dyn Storage,
) -> Result<Cardinality, StorageError> {
    let mut memory = Memory::new(storage);
    memory.scope(0, |memory| {
        extent_with(range, memory)?.cardinality_with(memory)
    })
}

/// Whether every question this module answers about `range` is answered exactly — no
/// [`Satisfiability::Undecided`] can arise from `range`, nor from any boolean
/// combination of it with other exactly-decided ranges.
///
/// A consumer raises a reported boundary when this is false, so it is exactly that
/// predicate: every value space of `range` is exactly represented and the remainder
/// flag is determined.
///
/// # Examples
///
/// ```rust
/// use purrdf_xsd::XsdDatatype;
/// use purrdf_xsd::range::{DataRange, is_exactly_decided};
///
/// assert!(is_exactly_decided(&DataRange::Datatype(XsdDatatype::Integer)));
/// assert!(!is_exactly_decided(&DataRange::Opaque));
/// ```
#[must_use]
pub fn is_exactly_decided(range: &DataRange) -> bool {
    extent(range).is_exact()
}

/// Whether `sub` is contained in `sup`, answered as the satisfiability of the
/// **counterexample range** `sub ⊓ ¬sup`.
///
/// Read the three answers as:
///
/// | answer | meaning |
/// |---|---|
/// | [`Satisfiability::Empty`] | PROVED `sub ⊆ sup` — there is no value in `sub` outside `sup` |
/// | [`Satisfiability::Inhabited`] | PROVED `sub ⊄ sup` — a witness in `sub` lies outside `sup` |
/// | [`Satisfiability::Undecided`] | neither is proved |
///
/// # Why the return type is a satisfiability rather than a boolean
///
/// Because containment is the question and emptiness is the *evidence*, and the two are
/// three-valued in the same way. A boolean would have to fold `Undecided` into one of the
/// other two, and both foldings are wrong for some consumer: folding it into "contained"
/// invents an inclusion, and folding it into "not contained" turns a limit of this module
/// into a false statement about the caller's datatypes. A consumer that wants a verdict
/// gates its NEGATIVE answer on [`is_exactly_decided`] of the same counterexample range;
/// [`Satisfiability::Inhabited`] on a range that is not exactly decided is a witness that
/// may or may not survive a finer decision procedure.
///
/// # Why this exists as a named function
///
/// The computation is one line, and written inline it is the double negative
/// "is the conjunction of `sub` with the complement of `sup` empty?" — an expression whose
/// argument order carries the whole meaning and reads identically when reversed. Naming it
/// puts the direction in the call site (`containment(sub, sup)`) instead of in the reader's
/// head.
///
/// # Examples
///
/// ```rust
/// use purrdf_xsd::XsdDatatype;
/// use purrdf_xsd::range::{DataRange, Satisfiability, containment};
///
/// let byte = DataRange::Datatype(XsdDatatype::Byte);
/// let short = DataRange::Datatype(XsdDatatype::Short);
///
/// // Every byte is a short, and the containment does NOT hold the other way.
/// assert_eq!(containment(&byte, &short), Satisfiability::Empty);
/// assert_eq!(containment(&short, &byte), Satisfiability::Inhabited);
///
/// // A range this module models nothing about decides nothing, in either direction.
/// assert_eq!(
///     containment(&DataRange::Opaque, &short),
///     Satisfiability::Undecided
/// );
/// ```
#[must_use]
pub fn containment(sub: &DataRange, sup: &DataRange) -> Satisfiability {
    resident(0, |memory| containment_with(sub, sup, memory))
}

/// Exact native counterexample emptiness with every temporary admitted before birth.
/// # Errors
/// Returns a typed refusal rather than weakening containment into an unknown result.
pub fn try_containment(
    sub: &DataRange,
    sup: &DataRange,
    storage: &mut dyn Storage,
) -> Result<Satisfiability, StorageError> {
    let mut memory = Memory::new(storage);
    memory.scope(0, |memory| containment_with(sub, sup, memory))
}

fn containment_with(
    sub: &DataRange,
    sup: &DataRange,
    memory: &mut Memory<'_>,
) -> Result<Satisfiability, StorageError> {
    memory.scope(0, |memory| {
        let sub = extent_with(sub, memory)?;
        let sup = extent_with(sup, memory)?;
        let not_sup = sup.complement_with(memory)?;
        let difference = sub.intersect_with(&not_sup, memory)?;
        difference.satisfiability_with(memory)
    })
}

/// The range a counterexample to `sub ⊆ sup` would have to inhabit.
///
/// Exposed beside [`containment`] because a consumer that gates a negative answer needs
/// [`is_exactly_decided`] of exactly this range: exactness is closed under the boolean
/// combination, so asking it of the two operands separately would be the same question asked
/// in two places and able to drift.
///
/// ```rust
/// use purrdf_xsd::XsdDatatype;
/// use purrdf_xsd::range::{DataRange, counterexample, is_exactly_decided};
///
/// let short = DataRange::Datatype(XsdDatatype::Short);
/// assert!(is_exactly_decided(&counterexample(&short, &short)));
/// assert!(!is_exactly_decided(&counterexample(
///     &DataRange::Opaque,
///     &short
/// )));
/// ```
#[must_use]
pub fn counterexample(sub: &DataRange, sup: &DataRange) -> DataRange {
    DataRange::And(vec![sub.clone(), DataRange::Not(Box::new(sup.clone()))])
}

/// XSD/OWL 2 **value-space identity**: whether `a` and `b` are the same value.
///
/// This is NOT [`crate::value_eq`] (SPARQL `=`). Two differences:
///
/// 1. The value space of `xsd:double` contains exactly one NaN, so `"NaN"^^xsd:double`
///    is identical to itself; SPARQL `=` answers `false`.
/// 2. Identity holds only WITHIN one value space. OWL 2's datatype map makes the value
///    spaces of `xsd:float`, `xsd:double` and `owl:real` (hence `xsd:decimal`)
///    pairwise disjoint, so `"5"^^xsd:float`, `"5"^^xsd:double` and `"5"^^xsd:decimal`
///    are three values; `value_eq` promotes across all of them. `"5"^^xsd:integer` and
///    `"5.0"^^xsd:decimal` are still ONE value — the integer value space is a subset of
///    the decimal value space.
///
/// Within the float and double spaces this identity is the one their order carries, so
/// `positiveZero` and `negativeZero` count as the same value here even though OWL 2
/// distinguishes them; [`cardinality`] accounts for the pair where it counts.
///
/// # Examples
///
/// ```rust
/// use purrdf_xsd::range::same_value;
/// use purrdf_xsd::{XsdDatatype, parse, value_eq};
///
/// let nan = parse("NaN", XsdDatatype::Double)?;
/// assert!(same_value(&nan, &nan));
/// assert!(!value_eq(&nan, &nan)); // SPARQL `=` is a different question
///
/// let int = parse("5", XsdDatatype::Integer)?;
/// let dec = parse("5.0", XsdDatatype::Decimal)?;
/// let dbl = parse("5", XsdDatatype::Double)?;
/// assert!(same_value(&int, &dec)); // one value space
/// assert!(!same_value(&dec, &dbl)); // disjoint value spaces
/// assert!(value_eq(&dec, &dbl)); // SPARQL promotes
/// # Ok::<(), purrdf_xsd::XsdError>(())
/// ```
#[must_use]
pub fn same_value(a: &XsdValue, b: &XsdValue) -> bool {
    resident(0, |memory| same_value_with(a, b, memory))
}

fn same_value_with(
    left: &XsdValue,
    right: &XsdValue,
    memory: &mut Memory<'_>,
) -> Result<bool, StorageError> {
    memory.step()?;
    if space_of_value(left) != space_of_value(right) {
        return Ok(false);
    }
    if space_of_value(left) == Space::Decimal {
        return memory.scope(0, |memory| {
            let left = memory.decimal_of(left)?.expect("decimal space value");
            let right = memory.decimal_of(right)?.expect("decimal space value");
            Ok(memory.decimal_cmp(&left, &right)? == Ordering::Equal)
        });
    }
    Ok(match (left, right) {
        (XsdValue::Float(left), XsdValue::Float(right)) => {
            same_float(f64::from(*left), f64::from(*right))
        }
        (XsdValue::Double(left), XsdValue::Double(right)) => same_float(*left, *right),
        (XsdValue::Time(left), XsdValue::Time(right)) => same_time(left, right),
        // Every nondecimal carrier here is an inline temporal/Boolean value or
        // borrowed lexical/byte sequence; its native equality allocates nothing.
        _ => value_eq(left, right),
    })
}

/// Float identity: the value space holds exactly one NaN, and the two zeros share the
/// one point the order distinguishes.
fn same_float(x: f64, y: f64) -> bool {
    if x.is_nan() || y.is_nan() {
        x.is_nan() && y.is_nan()
    } else {
        x == y
    }
}

/// `xsd:time` identity.
///
/// Two times are the same value when both carry a timezone and denote the same instant,
/// or neither does and they agree on the time of day — a timezone is part of the value,
/// and the XSD order never resolves a mixed pair to `Equal`. The end-of-day `24:00:00`
/// lexical is read as `00:00:00`, which XSD maps to the same value.
fn same_time(a: &Time, b: &Time) -> bool {
    let (a_zoned, a_secs, a_frac) = time_instant(a);
    let (b_zoned, b_secs, b_frac) = time_instant(b);
    a_zoned == b_zoned && a_secs == b_secs && a_frac.cmp_exact(&b_frac) == Ordering::Equal
}

/// A time as `(carries a timezone, whole seconds from the day's start in UTC, fractional
/// seconds)`, with hour 24 read as hour 0.
fn time_instant(t: &Time) -> (bool, i128, Decimal) {
    let hour = if t.hour() == 24 { 0 } else { t.hour() };
    let offset = t.timezone_minutes();
    let seconds = i128::from(hour) * 3600 + i128::from(t.minute()) * 60 + t.second().whole_part()
        - i128::from(offset.unwrap_or(0)) * 60;
    (offset.is_some(), seconds, t.second().frac_part())
}

// ── Kleene and counting lattices ─────────────────────────────────────────────────

impl Known {
    /// Kleene negation.
    fn negate(self) -> Self {
        match self {
            Self::Yes => Self::No,
            Self::No => Self::Yes,
            Self::Unknown => Self::Unknown,
        }
    }

    /// Kleene conjunction: a proved `No` wins over an unknown operand.
    fn conjoin(self, other: Self) -> Self {
        match (self, other) {
            (Self::No, _) | (_, Self::No) => Self::No,
            (Self::Unknown, _) | (_, Self::Unknown) => Self::Unknown,
            (Self::Yes, Self::Yes) => Self::Yes,
        }
    }

    /// Kleene disjunction: a proved `Yes` wins over an unknown operand.
    fn disjoin(self, other: Self) -> Self {
        match (self, other) {
            (Self::Yes, _) | (_, Self::Yes) => Self::Yes,
            (Self::Unknown, _) | (_, Self::Unknown) => Self::Unknown,
            (Self::No, Self::No) => Self::No,
        }
    }
}

impl Cardinality {
    /// The count of a disjoint union: exactness survives only if both operands are
    /// exact, and an undetermined part leaves the whole undetermined.
    fn plus(self, other: Self) -> Self {
        match (self, other) {
            (Self::Undecided, _) | (_, Self::Undecided) => Self::Undecided,
            (Self::Unbounded, _) | (_, Self::Unbounded) => Self::Unbounded,
            (Self::AtLeast(a), Self::AtLeast(b) | Self::Exactly(b))
            | (Self::Exactly(a), Self::AtLeast(b)) => Self::AtLeast(a.saturating_add(b)),
            (Self::Exactly(a), Self::Exactly(b)) => match a.checked_add(b) {
                Some(n) => Self::Exactly(n),
                // Past `u64::MAX` the count is still finite, so it stays a lower bound.
                None => Self::AtLeast(u64::MAX),
            },
        }
    }

    /// Remove `n` values known to be members. Exactness survives.
    fn minus(self, n: u64) -> Self {
        match self {
            Self::Exactly(k) => Self::Exactly(k.saturating_sub(n)),
            Self::AtLeast(k) => Self::AtLeast(k.saturating_sub(n)),
            Self::Unbounded => Self::Unbounded,
            Self::Undecided => Self::Undecided,
        }
    }
}

/// A `u64` count as an exact cardinality.
fn exactly(n: usize) -> Cardinality {
    Cardinality::Exactly(u64::try_from(n).unwrap_or(u64::MAX))
}

// ── The value-space partition ────────────────────────────────────────────────────

/// A primitive value space of the data domain.
#[cfg_attr(test, derive(Debug))]
#[derive(Clone, Copy, PartialEq, Eq)]
enum Space {
    /// `xsd:decimal` and the integer family.
    Decimal,
    /// `xsd:float`.
    Float,
    /// `xsd:double`.
    Double,
    /// `xsd:boolean`.
    Boolean,
    /// One of the length-selected spaces.
    Text(TextSpace),
    /// One of the temporal spaces.
    Temporal(TemporalSpace),
}

/// A length-selected value space. The discriminant indexes [`Extent::text`].
#[cfg_attr(test, derive(Debug))]
#[derive(Clone, Copy, PartialEq, Eq)]
enum TextSpace {
    /// `xsd:string` — length in characters.
    String,
    /// `xsd:hexBinary` — length in octets.
    HexBinary,
    /// `xsd:base64Binary` — length in octets.
    Base64Binary,
}

/// A temporal value space. The discriminant indexes [`Extent::temporal`].
#[cfg_attr(test, derive(Debug))]
#[derive(Clone, Copy, PartialEq, Eq)]
enum TemporalSpace {
    /// Zoned `xsd:dateTime` values, exactly `xsd:dateTimeStamp`.
    DateTime,
    /// The remaining `xsd:dateTime` values, without a timezone.
    DateTimeUnzoned,
    /// `xsd:date`.
    Date,
    /// `xsd:time`.
    Time,
    /// `xsd:duration` and its two subtypes, which share one value space.
    Duration,
    /// `xsd:gYear`.
    GYear,
    /// `xsd:gMonth`.
    GMonth,
    /// `xsd:gDay`.
    GDay,
    /// `xsd:gYearMonth`.
    GYearMonth,
    /// `xsd:gMonthDay`.
    GMonthDay,
}

/// The length-selected spaces, in [`Extent::text`] order.
const TEXT_SPACES: [TextSpace; 3] = [
    TextSpace::String,
    TextSpace::HexBinary,
    TextSpace::Base64Binary,
];

/// The temporal spaces, in [`Extent::temporal`] order.
const TEMPORAL_SPACES: [TemporalSpace; 10] = [
    TemporalSpace::DateTime,
    TemporalSpace::DateTimeUnzoned,
    TemporalSpace::Date,
    TemporalSpace::Time,
    TemporalSpace::Duration,
    TemporalSpace::GYear,
    TemporalSpace::GMonth,
    TemporalSpace::GDay,
    TemporalSpace::GYearMonth,
    TemporalSpace::GMonthDay,
];

/// The number of distinct timezone settings an XSD temporal value can carry: absent,
/// or one of the 1681 minute offsets in `-14:00 ..= +14:00`.
const TZ_SETTINGS: u64 = 1 + (2 * 14 * 60 + 1);

impl TextSpace {
    /// Which unit this space's `length` facet counts.
    fn kind(self) -> LengthKind {
        match self {
            Self::String => LengthKind::Characters,
            Self::HexBinary | Self::Base64Binary => LengthKind::Octets,
        }
    }
}

impl TemporalSpace {
    /// The number of values in this space, or `None` when it is infinite.
    ///
    /// Three Gregorian spaces are finite (a bounded field set times the timezone
    /// settings), which is what lets the complement of an enumeration over them be
    /// decided exactly rather than assumed inhabited.
    fn size(self) -> Option<u64> {
        Some(match self {
            // 12 months, 31 days, and the 366 valid month-day pairs (Feb 29 included).
            Self::GMonth => 12 * TZ_SETTINGS,
            Self::GDay => 31 * TZ_SETTINGS,
            Self::GMonthDay => 366 * TZ_SETTINGS,
            _ => return None,
        })
    }
}

/// The value space a datatype belongs to.
fn space_of_datatype(dt: XsdDatatype) -> Space {
    use XsdDatatype as D;
    match dt {
        D::Integer
        | D::Long
        | D::Int
        | D::Short
        | D::Byte
        | D::UnsignedLong
        | D::UnsignedInt
        | D::UnsignedShort
        | D::UnsignedByte
        | D::NonNegativeInteger
        | D::PositiveInteger
        | D::NonPositiveInteger
        | D::NegativeInteger
        | D::Decimal => Space::Decimal,
        D::Float => Space::Float,
        D::Double => Space::Double,
        D::Boolean => Space::Boolean,
        D::String => Space::Text(TextSpace::String),
        D::HexBinary => Space::Text(TextSpace::HexBinary),
        D::Base64Binary => Space::Text(TextSpace::Base64Binary),
        D::DateTime | D::DateTimeStamp => Space::Temporal(TemporalSpace::DateTime),
        D::Date => Space::Temporal(TemporalSpace::Date),
        D::Time => Space::Temporal(TemporalSpace::Time),
        D::Duration | D::DayTimeDuration | D::YearMonthDuration => {
            Space::Temporal(TemporalSpace::Duration)
        }
        D::GYear => Space::Temporal(TemporalSpace::GYear),
        D::GMonth => Space::Temporal(TemporalSpace::GMonth),
        D::GDay => Space::Temporal(TemporalSpace::GDay),
        D::GYearMonth => Space::Temporal(TemporalSpace::GYearMonth),
        D::GMonthDay => Space::Temporal(TemporalSpace::GMonthDay),
    }
}

/// The value space a value belongs to.
fn space_of_value(value: &XsdValue) -> Space {
    if matches!(value, XsdValue::DateTime(value) if value.timezone_minutes().is_none()) {
        Space::Temporal(TemporalSpace::DateTimeUnzoned)
    } else {
        space_of_datatype(value.datatype())
    }
}

/// Whether a datatype's value space is the WHOLE of its [`Space`].
///
/// False only for the two duration subtypes: each is an infinite proper subspace of
/// the shared duration space, which listed sets cannot express exactly.
fn covers_whole_space(dt: XsdDatatype) -> bool {
    !matches!(
        dt,
        XsdDatatype::DayTimeDuration | XsdDatatype::YearMonthDuration
    )
}

// ── Interval sets over one erased endpoint domain ────────────────────────────────

/// One interval endpoint, in whichever of the ordered strata its set belongs to.
///
/// The three payloads are erased into ONE type deliberately. The interval algebra below
/// is purely order-based: every per-stratum decision — an integer's successor, a float's
/// `next_up`, a length's carrier — lives in an [`Algebra`] impl instead, so one
/// non-generic algebra serves all three strata.
///
/// A set's endpoints all come from the stratum that built it, so two endpoints of
/// different strata are never compared. [`Ord`] settles that impossible case by stratum
/// tag rather than pretending a cross-stratum comparison means anything, and every place
/// that reads a payload back out ([`Point::as_dec`] and its siblings) widens on it —
/// answering "more values" — so the impossible case could never invent an `Empty`.
#[derive(Clone)]
enum Point {
    /// A decimal endpoint of any size, held exactly ([`exact::Decimal`]) and ordered
    /// exactly — total, and never `NaN`.
    Dec(exact::Decimal),
    /// A float endpoint at either width, held as `f64` (every `f32` converts exactly).
    ///
    /// [`Point::float`] normalizes the two zeros onto one point, which is the order these
    /// value spaces carry: `positiveZero` and `negativeZero` are equal, so no bound facet
    /// can separate them. `NaN` never reaches this stratum — it is one of its own.
    Float(f64),
    /// A length endpoint, in the unit its space counts.
    Len(u64),
}

impl Point {
    fn cmp_with(&self, other: &Self, memory: &mut Memory<'_>) -> Result<Ordering, StorageError> {
        memory.step()?;
        Ok(match (self, other) {
            (Self::Dec(left), Self::Dec(right)) => return memory.decimal_cmp(left, right),
            (Self::Float(left), Self::Float(right)) => {
                left.partial_cmp(right).unwrap_or(Ordering::Equal)
            }
            (Self::Len(left), Self::Len(right)) => left.cmp(right),
            _ => self.stratum().cmp(&other.stratum()),
        })
    }
    /// A float endpoint with the two zeros collapsed onto one point.
    fn float(x: f64) -> Self {
        Self::Float(if x == 0.0 { 0.0 } else { x })
    }

    /// The decimal this endpoint carries, or `None` for another stratum's endpoint.
    fn as_dec(&self) -> Option<&exact::Decimal> {
        match self {
            Self::Dec(d) => Some(d),
            Self::Float(_) | Self::Len(_) => None,
        }
    }

    /// The float this endpoint carries, or `None` for another stratum's endpoint.
    fn as_float(&self) -> Option<f64> {
        match self {
            Self::Float(x) => Some(*x),
            Self::Dec(_) | Self::Len(_) => None,
        }
    }

    /// The length this endpoint carries, or `None` for another stratum's endpoint.
    fn as_len(&self) -> Option<u64> {
        match self {
            Self::Len(l) => Some(*l),
            Self::Dec(_) | Self::Float(_) => None,
        }
    }

    /// Which stratum the endpoint came from. This orders the strata apart so that [`Ord`]
    /// stays total; it never orders two endpoints that one set actually holds together.
    fn stratum(&self) -> u8 {
        match self {
            Self::Dec(_) => 0,
            Self::Float(_) => 1,
            Self::Len(_) => 2,
        }
    }
}

impl Ord for Point {
    fn cmp(&self, other: &Self) -> Ordering {
        resident(0, |memory| self.cmp_with(other, memory))
    }
}

impl PartialOrd for Point {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Point {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Point {}

impl OwnedBytes for Point {
    fn owned_bytes(&self) -> usize {
        match self {
            Self::Dec(value) => value.owned_bytes(),
            _ => 0,
        }
    }
}
impl CloneOwned for Point {
    fn clone_owned(&self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        Ok(match self {
            Self::Dec(value) => Self::Dec(value.clone_owned(memory)?),
            Self::Float(value) => Self::Float(*value),
            Self::Len(value) => Self::Len(*value),
        })
    }
}

/// The lower end of an interval.
#[derive(Clone)]
enum Lo {
    /// No lower bound.
    Unbounded,
    /// The endpoint is included.
    Incl(Point),
    /// The endpoint is excluded.
    Excl(Point),
}

/// The upper end of an interval.
#[derive(Clone)]
enum Hi {
    /// No upper bound.
    Unbounded,
    /// The endpoint is included.
    Incl(Point),
    /// The endpoint is excluded.
    Excl(Point),
}

macro_rules! endpoint_ownership { ($($ty:ident),+ $(,)?) => {$(
    impl OwnedBytes for $ty { fn owned_bytes(&self) -> usize { match self { Self::Unbounded => 0, Self::Incl(value) | Self::Excl(value) => value.owned_bytes() } } }
    impl CloneOwned for $ty {
        fn clone_owned(&self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
            Ok(match self { Self::Unbounded => Self::Unbounded, Self::Incl(value) => Self::Incl(value.clone_owned(memory)?), Self::Excl(value) => Self::Excl(value.clone_owned(memory)?) })
        }
    }
)+}; }
endpoint_ownership!(Lo, Hi);

impl Lo {
    fn cmp_with(&self, other: &Self, memory: &mut Memory<'_>) -> Result<Ordering, StorageError> {
        Ok(match (self, other) {
            (Self::Unbounded, Self::Unbounded) => Ordering::Equal,
            (Self::Unbounded, _) => Ordering::Less,
            (_, Self::Unbounded) => Ordering::Greater,
            (Self::Incl(left) | Self::Excl(left), Self::Incl(right) | Self::Excl(right)) => left
                .cmp_with(right, memory)?
                .then_with(|| self.rank().cmp(&other.rank())),
        })
    }
    fn admits_with(&self, value: &Point, memory: &mut Memory<'_>) -> Result<bool, StorageError> {
        Ok(match self {
            Self::Unbounded => true,
            Self::Incl(bound) => value.cmp_with(bound, memory)? != Ordering::Less,
            Self::Excl(bound) => value.cmp_with(bound, memory)? == Ordering::Greater,
        })
    }

    /// Sort key at one endpoint: inclusive is the lower end.
    fn rank(&self) -> u8 {
        match self {
            Self::Unbounded => 0,
            Self::Incl(_) => 1,
            Self::Excl(_) => 2,
        }
    }
}

impl Hi {
    fn cmp_with(&self, other: &Self, memory: &mut Memory<'_>) -> Result<Ordering, StorageError> {
        Ok(match (self, other) {
            (Self::Unbounded, Self::Unbounded) => Ordering::Equal,
            (Self::Unbounded, _) => Ordering::Greater,
            (_, Self::Unbounded) => Ordering::Less,
            (Self::Incl(left) | Self::Excl(left), Self::Incl(right) | Self::Excl(right)) => left
                .cmp_with(right, memory)?
                .then_with(|| self.rank().cmp(&other.rank())),
        })
    }
    fn admits_with(&self, value: &Point, memory: &mut Memory<'_>) -> Result<bool, StorageError> {
        Ok(match self {
            Self::Unbounded => true,
            Self::Incl(bound) => value.cmp_with(bound, memory)? != Ordering::Greater,
            Self::Excl(bound) => value.cmp_with(bound, memory)? == Ordering::Less,
        })
    }

    /// Sort key at one endpoint: exclusive is the lower end.
    fn rank(&self) -> u8 {
        match self {
            Self::Excl(_) => 0,
            Self::Incl(_) => 1,
            Self::Unbounded => 2,
        }
    }
}

/// One interval over the endpoint domain.
#[derive(Clone)]
struct Interval {
    /// The lower end.
    lo: Lo,
    /// The upper end.
    hi: Hi,
}

impl OwnedBytes for Interval {
    fn owned_bytes(&self) -> usize {
        self.lo.owned_bytes() + self.hi.owned_bytes()
    }
}
impl CloneOwned for Interval {
    fn clone_owned(&self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        Ok(Self {
            lo: self.lo.clone_owned(memory)?,
            hi: self.hi.clone_owned(memory)?,
        })
    }
}

impl Interval {
    /// The whole domain.
    fn full() -> Self {
        Self {
            lo: Lo::Unbounded,
            hi: Hi::Unbounded,
        }
    }

    /// A single point, both ends inclusive.
    fn point(v: Point) -> Self {
        resident(v.owned_bytes(), |memory| Self::point_with(v, memory))
    }
    fn point_with(value: Point, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        memory.scope(value.owned_bytes(), |memory| {
            Ok(Self {
                lo: Lo::Incl(value.clone_owned(memory)?),
                hi: Hi::Incl(value),
            })
        })
    }
    fn spans_with(&self, memory: &mut Memory<'_>) -> Result<bool, StorageError> {
        Ok(match (&self.lo, &self.hi) {
            (Lo::Unbounded, _) | (_, Hi::Unbounded) => true,
            (Lo::Incl(left), Hi::Incl(right)) => left.cmp_with(right, memory)? != Ordering::Greater,
            (Lo::Incl(left) | Lo::Excl(left), Hi::Excl(right))
            | (Lo::Excl(left), Hi::Incl(right)) => left.cmp_with(right, memory)? == Ordering::Less,
        })
    }
    fn degenerate_with(&self, memory: &mut Memory<'_>) -> Result<Option<&Point>, StorageError> {
        Ok(match (&self.lo, &self.hi) {
            (Lo::Incl(left), Hi::Incl(right))
                if left.cmp_with(right, memory)? == Ordering::Equal =>
            {
                Some(left)
            }
            _ => None,
        })
    }
    fn holds_with(&self, value: &Point, memory: &mut Memory<'_>) -> Result<bool, StorageError> {
        Ok(self.lo.admits_with(value, memory)? && self.hi.admits_with(value, memory)?)
    }
}

/// A canonical, sorted, pairwise order-disjoint set of intervals.
#[derive(Clone)]
struct IntervalSet {
    /// The intervals, sorted by lower end and merged where they touch.
    intervals: Vec<Interval>,
}

impl OwnedBytes for IntervalSet {
    fn owned_bytes(&self) -> usize {
        self.intervals.owned_bytes()
    }
}
impl CloneOwned for IntervalSet {
    fn clone_owned(&self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        Ok(Self {
            intervals: self.intervals.clone_owned(memory)?,
        })
    }
}

impl IntervalSet {
    fn full_with(memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        Ok(Self {
            intervals: memory.one(Interval::full())?,
        })
    }
    fn point_with(value: Point, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        let interval = Interval::point_with(value, memory)?;
        Ok(Self {
            intervals: memory.one(interval)?,
        })
    }
    fn canonical_with(
        mut intervals: Vec<Interval>,
        memory: &mut Memory<'_>,
    ) -> Result<Self, StorageError> {
        memory.scope(intervals.owned_bytes(), |memory| {
            let mut first_error = None;
            intervals.retain(|interval| match interval.spans_with(memory) {
                Ok(spans) => spans,
                Err(error) => {
                    first_error.get_or_insert(error);
                    true
                }
            });
            if let Some(error) = first_error {
                return Err(error);
            }
            memory.sort(&mut intervals, |left, right, memory| {
                let order = left.lo.cmp_with(&right.lo, memory)?;
                if order == Ordering::Equal {
                    left.hi.cmp_with(&right.hi, memory)
                } else {
                    Ok(order)
                }
            })?;
            let mut merged: Vec<Interval> = Vec::new();
            memory.reserve(&mut merged, intervals.len())?;
            for interval in intervals {
                let touching = match merged.last() {
                    Some(last) => !gap_with(last, &interval, memory)?,
                    None => false,
                };
                if touching {
                    if let Some(last) = merged.last_mut()
                        && last.hi.cmp_with(&interval.hi, memory)? == Ordering::Less
                    {
                        last.hi = interval.hi;
                    }
                } else {
                    merged.push(interval);
                }
            }
            Ok(Self { intervals: merged })
        })
    }
    fn holds_with(&self, value: &Point, memory: &mut Memory<'_>) -> Result<bool, StorageError> {
        for interval in &self.intervals {
            if interval.holds_with(value, memory)? {
                return Ok(true);
            }
        }
        Ok(false)
    }
    fn complement_with(&self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        memory.scope(0, |memory| {
            let mut out: Vec<Interval> = Vec::new();
            memory.reserve(
                &mut out,
                self.intervals
                    .len()
                    .checked_add(1)
                    .ok_or(StorageError::Allocation)?,
            )?;
            let mut cursor = Lo::Unbounded;
            let mut open = true;
            for interval in &self.intervals {
                let hi = match &interval.lo {
                    Lo::Unbounded => None,
                    Lo::Incl(value) => Some(Hi::Excl(value.clone_owned(memory)?)),
                    Lo::Excl(value) => Some(Hi::Incl(value.clone_owned(memory)?)),
                };
                if let Some(hi) = hi {
                    out.push(Interval {
                        lo: cursor.clone_owned(memory)?,
                        hi,
                    });
                }
                let next = match &interval.hi {
                    Hi::Unbounded => {
                        open = false;
                        break;
                    }
                    Hi::Incl(value) => Lo::Excl(value.clone_owned(memory)?),
                    Hi::Excl(value) => Lo::Incl(value.clone_owned(memory)?),
                };
                memory.drop_value(cursor)?;
                cursor = next;
            }
            if open {
                out.push(Interval {
                    lo: cursor,
                    hi: Hi::Unbounded,
                });
            }
            Self::canonical_with(out, memory)
        })
    }
    fn intersect_with(&self, other: &Self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        memory.scope(0, |memory| {
            let mut out = Vec::new();
            for left in &self.intervals {
                for right in &other.intervals {
                    let lo = if left.lo.cmp_with(&right.lo, memory)? == Ordering::Greater {
                        &left.lo
                    } else {
                        &right.lo
                    };
                    let hi = if left.hi.cmp_with(&right.hi, memory)? == Ordering::Less {
                        &left.hi
                    } else {
                        &right.hi
                    };
                    let interval = Interval {
                        lo: lo.clone_owned(memory)?,
                        hi: hi.clone_owned(memory)?,
                    };
                    if interval.spans_with(memory)? {
                        memory.push(&mut out, interval)?;
                    } else {
                        memory.drop_value(interval)?;
                    }
                }
            }
            Self::canonical_with(out, memory)
        })
    }
    fn union_with(&self, other: &Self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        memory.scope(0, |memory| {
            let mut out = self.intervals.clone_owned(memory)?;
            memory.reserve(&mut out, other.intervals.len())?;
            for interval in &other.intervals {
                out.push(interval.clone_owned(memory)?);
            }
            Self::canonical_with(out, memory)
        })
    }
    fn sample_with(&self, limit: usize, memory: &mut Memory<'_>) -> Result<Vec<u64>, StorageError> {
        let mut out = Vec::new();
        for interval in &self.intervals {
            let mut cursor = match &interval.lo {
                Lo::Unbounded => 0,
                Lo::Incl(value) => value.as_len().unwrap_or(0),
                Lo::Excl(value) => match value.as_len().unwrap_or(0).checked_add(1) {
                    Some(next) => next,
                    None => continue,
                },
            };
            for _ in 0..limit {
                if !interval.holds_with(&Point::Len(cursor), memory)? {
                    break;
                }
                memory.push(&mut out, cursor)?;
                match cursor.checked_add(1) {
                    Some(next) => cursor = next,
                    None => break,
                }
            }
        }
        Ok(out)
    }
    /// The empty set.
    fn empty() -> Self {
        Self {
            intervals: Vec::new(),
        }
    }

    /// The whole domain.
    #[cfg(test)]
    fn full() -> Self {
        resident(0, Self::full_with)
    }

    /// Canonicalize: drop order-empty intervals, sort, and merge every pair that leaves
    /// no gap between them.
    ///
    /// The sort is unstable, which stays deterministic here: two intervals that compare
    /// equal agree on both ends, and every reading of an endpoint afterwards is numeric
    /// (an order comparison, an integer rounding, a carrier test), so order-equal
    /// endpoints answer alike and the tie order cannot be observed.
    #[cfg(test)]
    fn canonical(intervals: Vec<Interval>) -> Self {
        resident(intervals.owned_bytes(), |memory| {
            Self::canonical_with(intervals, memory)
        })
    }

    /// Whether `v` lies in the set.
    #[cfg(test)]
    fn holds(&self, v: &Point) -> bool {
        resident(0, |memory| self.holds_with(v, memory))
    }

    /// The gaps — exactly the set-theoretic complement over the order.
    #[cfg(test)]
    fn complement(&self) -> Self {
        resident(0, |memory| self.complement_with(memory))
    }

    /// Pairwise intersection.
    #[cfg(test)]
    fn intersect(&self, other: &Self) -> Self {
        resident(0, |memory| self.intersect_with(other, memory))
    }
}

fn gap_with(
    first: &Interval,
    second: &Interval,
    memory: &mut Memory<'_>,
) -> Result<bool, StorageError> {
    let (left, left_excluded) = match &first.hi {
        Hi::Unbounded => return Ok(false),
        Hi::Incl(value) => (value, false),
        Hi::Excl(value) => (value, true),
    };
    let (right, right_excluded) = match &second.lo {
        Lo::Unbounded => return Ok(false),
        Lo::Incl(value) => (value, false),
        Lo::Excl(value) => (value, true),
    };
    Ok(match left.cmp_with(right, memory)? {
        Ordering::Less => true,
        Ordering::Equal => left_excluded && right_excluded,
        Ordering::Greater => false,
    })
}

// ── Decimal endpoint arithmetic ──────────────────────────────────────────────────

/// Whether a decimal is one of the integers.
const fn is_integral(d: &exact::Decimal) -> bool {
    d.is_integer()
}

// ── The per-space closed algebras ────────────────────────────────────────────────

/// A set exactly represented in one value space's own closed algebra.
trait Algebra: Clone + CloneOwned {
    /// The value identity this space accepts.
    type Value;
    /// The space's complement of this set.
    #[cfg(test)]
    fn complement(&self) -> Self {
        resident(0, |memory| self.complement_with(memory))
    }
    /// Whether the set holds no value. Exact — that is what makes the shape closed.
    #[cfg(test)]
    fn is_empty(&self) -> bool {
        resident(0, |memory| self.empty_with(memory))
    }
    /// How many values the set holds.
    #[cfg(test)]
    fn count(&self) -> Cardinality {
        resident(0, |memory| self.count_with(memory))
    }
    /// Whether the set holds `value`, which the caller has routed to this space.
    #[cfg(test)]
    fn holds(&self, value: &Self::Value) -> bool {
        resident(0, |memory| self.holds_with(value, memory))
    }
    fn complement_with(&self, memory: &mut Memory<'_>) -> Result<Self, StorageError>;
    fn intersect_with(&self, other: &Self, memory: &mut Memory<'_>) -> Result<Self, StorageError>;
    fn union_with(&self, other: &Self, memory: &mut Memory<'_>) -> Result<Self, StorageError>;
    fn empty_with(&self, memory: &mut Memory<'_>) -> Result<bool, StorageError>;
    fn count_with(&self, memory: &mut Memory<'_>) -> Result<Cardinality, StorageError>;
    fn holds_with(
        &self,
        value: &Self::Value,
        memory: &mut Memory<'_>,
    ) -> Result<bool, StorageError>;
}

/// The decimal space: the integers and the non-integral decimals, each an interval set
/// over one shared endpoint domain and distinguished only by its carrier.
#[derive(Clone)]
struct DecimalSet {
    /// Intervals selecting integers.
    integral: IntervalSet,
    /// Intervals selecting non-integral decimals.
    fractional: IntervalSet,
}

/// The inclusive integer window an interval selects; `None` on a side means unbounded.
#[derive(Clone)]
struct IntWindow {
    /// The least integer, or `None` when unbounded below.
    lo: Option<exact::Integer>,
    /// The greatest integer, or `None` when unbounded above.
    hi: Option<exact::Integer>,
}

impl OwnedBytes for IntWindow {
    fn owned_bytes(&self) -> usize {
        self.lo.owned_bytes() + self.hi.owned_bytes()
    }
}
impl OwnedBytes for DecimalSet {
    fn owned_bytes(&self) -> usize {
        self.integral.owned_bytes() + self.fractional.owned_bytes()
    }
}
impl CloneOwned for DecimalSet {
    fn clone_owned(&self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        Ok(Self {
            integral: self.integral.clone_owned(memory)?,
            fractional: self.fractional.clone_owned(memory)?,
        })
    }
}

impl DecimalSet {
    fn full_with(memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        Ok(Self {
            integral: IntervalSet::full_with(memory)?,
            fractional: IntervalSet::full_with(memory)?,
        })
    }
    fn point_with(value: exact::Decimal, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        let integral = is_integral(&value);
        let set = IntervalSet::point_with(Point::Dec(value), memory)?;
        Ok(if integral {
            Self {
                integral: set,
                fractional: IntervalSet::empty(),
            }
        } else {
            Self {
                integral: IntervalSet::empty(),
                fractional: set,
            }
        })
    }
    fn datatype_with(datatype: XsdDatatype, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        let Some((lo, hi)) = datatype.integer_range() else {
            return Self::full_with(memory);
        };
        let lo = if lo == i128::MIN {
            Lo::Unbounded
        } else {
            Lo::Incl(Point::Dec(exact::Decimal::from(lo)))
        };
        let hi = if hi == i128::MAX {
            Hi::Unbounded
        } else {
            Hi::Incl(Point::Dec(exact::Decimal::from(hi)))
        };
        let intervals = memory.one(Interval { lo, hi })?;
        Ok(Self {
            integral: IntervalSet::canonical_with(intervals, memory)?,
            fractional: IntervalSet::empty(),
        })
    }
    fn interval_with(interval: Interval, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        let copy = interval.clone_owned(memory)?;
        let integral = memory.one(copy)?;
        let fractional = memory.one(interval)?;
        Ok(Self {
            integral: IntervalSet::canonical_with(integral, memory)?,
            fractional: IntervalSet::canonical_with(fractional, memory)?,
        })
    }
    fn window_with(
        interval: &Interval,
        memory: &mut Memory<'_>,
    ) -> Result<Option<IntWindow>, StorageError> {
        memory.scope(0, |memory| {
            let lo = match &interval.lo {
                Lo::Unbounded => None,
                Lo::Incl(value) => match value.as_dec() {
                    Some(value) => Some(memory.round(value, exact::Rounding::Ceiling)?),
                    None => None,
                },
                Lo::Excl(value) => match value.as_dec() {
                    Some(value) => {
                        let rounded = memory.round(value, exact::Rounding::Ceiling)?;
                        Some(if is_integral(value) {
                            memory.integer_add(&rounded, &exact::Integer::ONE)?
                        } else {
                            rounded
                        })
                    }
                    None => None,
                },
            };
            let hi = match &interval.hi {
                Hi::Unbounded => None,
                Hi::Incl(value) => match value.as_dec() {
                    Some(value) => Some(memory.round(value, exact::Rounding::Floor)?),
                    None => None,
                },
                Hi::Excl(value) => match value.as_dec() {
                    Some(value) => {
                        let rounded = memory.round(value, exact::Rounding::Floor)?;
                        Some(if is_integral(value) {
                            memory.integer_sub(&rounded, &exact::Integer::ONE)?
                        } else {
                            rounded
                        })
                    }
                    None => None,
                },
            };
            Ok(
                if let (Some(left), Some(right)) = (&lo, &hi)
                    && left > right
                {
                    None
                } else {
                    Some(IntWindow { lo, hi })
                },
            )
        })
    }

    /// The empty set.
    fn empty() -> Self {
        Self {
            integral: IntervalSet::empty(),
            fractional: IntervalSet::empty(),
        }
    }

    /// The integer window of one interval, or `None` when it holds no integer.
    #[cfg(test)]
    fn window(iv: &Interval) -> Option<IntWindow> {
        resident(0, |memory| Self::window_with(iv, memory))
    }
}

impl Algebra for DecimalSet {
    type Value = XsdValue;
    fn complement_with(&self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        Ok(Self {
            integral: self.integral.complement_with(memory)?,
            fractional: self.fractional.complement_with(memory)?,
        })
    }
    fn intersect_with(&self, other: &Self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        Ok(Self {
            integral: self.integral.intersect_with(&other.integral, memory)?,
            fractional: self.fractional.intersect_with(&other.fractional, memory)?,
        })
    }
    fn union_with(&self, other: &Self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        Ok(Self {
            integral: self.integral.union_with(&other.integral, memory)?,
            fractional: self.fractional.union_with(&other.fractional, memory)?,
        })
    }
    fn empty_with(&self, memory: &mut Memory<'_>) -> Result<bool, StorageError> {
        memory.scope(0, |memory| {
            for interval in &self.integral.intervals {
                let window = Self::window_with(interval, memory)?;
                let inhabited = window.is_some();
                memory.drop_value(window)?;
                if inhabited {
                    return Ok(false);
                }
            }
            for interval in &self.fractional.intervals {
                if interval
                    .degenerate_with(memory)?
                    .and_then(Point::as_dec)
                    .is_none_or(|value| !is_integral(value))
                {
                    return Ok(false);
                }
            }
            Ok(true)
        })
    }
    fn count_with(&self, memory: &mut Memory<'_>) -> Result<Cardinality, StorageError> {
        memory.scope(0, |memory| {
            let mut total = Cardinality::Exactly(0);
            for interval in &self.integral.intervals {
                let held = memory.scope(0, |memory| {
                    let Some(window) = Self::window_with(interval, memory)? else {
                        return Ok(Cardinality::Exactly(0));
                    };
                    Ok(match (window.lo, window.hi) {
                        (Some(left), Some(right)) => {
                            let difference = memory.integer_sub(&right, &left)?;
                            let span = memory.integer_add(&difference, &exact::Integer::ONE)?;
                            span.as_i128()
                                .and_then(|span| u64::try_from(span).ok())
                                .map_or(Cardinality::AtLeast(u64::MAX), Cardinality::Exactly)
                        }
                        _ => Cardinality::Unbounded,
                    })
                })?;
                total = total.plus(held);
            }
            for interval in &self.fractional.intervals {
                total = total.plus(
                    match interval.degenerate_with(memory)?.and_then(Point::as_dec) {
                        Some(value) if is_integral(value) => Cardinality::Exactly(0),
                        Some(_) => Cardinality::Exactly(1),
                        None => Cardinality::Unbounded,
                    },
                );
            }
            Ok(total)
        })
    }
    fn holds_with(&self, value: &XsdValue, memory: &mut Memory<'_>) -> Result<bool, StorageError> {
        memory.scope(0, |memory| {
            let Some(value) = memory.decimal_of(value)? else {
                return Ok(false);
            };
            let integral = is_integral(&value);
            let point = Point::Dec(value);
            if integral {
                self.integral.holds_with(&point, memory)
            } else {
                self.fractional.holds_with(&point, memory)
            }
        })
    }
}

/// Which IEEE width a float space carries.
#[derive(Clone, Copy, PartialEq, Eq)]
enum FloatWidth {
    /// `xsd:float`.
    Single,
    /// `xsd:double`.
    Double,
}

/// A float space: the numbers (finite values and ±INF) and the single NaN.
#[derive(Clone)]
struct FloatSet {
    /// The IEEE width of the space.
    width: FloatWidth,
    /// Intervals over the numbers.
    number: IntervalSet,
    /// Whether the space's one NaN is a member.
    nan: bool,
}

impl OwnedBytes for FloatSet {
    fn owned_bytes(&self) -> usize {
        self.number.owned_bytes()
    }
}
impl CloneOwned for FloatSet {
    fn clone_owned(&self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        Ok(Self {
            width: self.width,
            number: self.number.clone_owned(memory)?,
            nan: self.nan,
        })
    }
}

impl FloatSet {
    fn full_with(width: FloatWidth, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        Ok(Self {
            width,
            number: IntervalSet::full_with(memory)?,
            nan: true,
        })
    }
    fn interval_with(
        width: FloatWidth,
        interval: Interval,
        memory: &mut Memory<'_>,
    ) -> Result<Self, StorageError> {
        let intervals = memory.one(interval)?;
        Ok(Self {
            width,
            number: IntervalSet::canonical_with(intervals, memory)?,
            nan: false,
        })
    }
    fn admits_with(
        &self,
        interval: &Interval,
        memory: &mut Memory<'_>,
    ) -> Result<bool, StorageError> {
        let candidate = match &interval.lo {
            Lo::Unbounded => f64::NEG_INFINITY,
            Lo::Incl(value) => match value.as_float() {
                Some(value) => value,
                None => return Ok(true),
            },
            Lo::Excl(value) => {
                let Some(value) = value.as_float() else {
                    return Ok(true);
                };
                let next = self.next_up(value);
                if next <= value {
                    return Ok(false);
                }
                next
            }
        };
        interval.hi.admits_with(&Point::float(candidate), memory)
    }

    /// The empty set.
    fn empty(width: FloatWidth) -> Self {
        Self {
            width,
            number: IntervalSet::empty(),
            nan: false,
        }
    }

    /// The immediate successor of `x` at this space's width.
    fn next_up(&self, x: f64) -> f64 {
        match self.width {
            FloatWidth::Single => f64::from((x as f32).next_up()),
            FloatWidth::Double => x.next_up(),
        }
    }
}

impl Algebra for FloatSet {
    type Value = XsdValue;
    fn complement_with(&self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        Ok(Self {
            width: self.width,
            number: self.number.complement_with(memory)?,
            nan: !self.nan,
        })
    }
    fn intersect_with(&self, other: &Self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        Ok(Self {
            width: self.width,
            number: self.number.intersect_with(&other.number, memory)?,
            nan: self.nan && other.nan,
        })
    }
    fn union_with(&self, other: &Self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        Ok(Self {
            width: self.width,
            number: self.number.union_with(&other.number, memory)?,
            nan: self.nan || other.nan,
        })
    }
    fn empty_with(&self, memory: &mut Memory<'_>) -> Result<bool, StorageError> {
        if self.nan {
            return Ok(false);
        }
        for interval in &self.number.intervals {
            if self.admits_with(interval, memory)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
    fn count_with(&self, memory: &mut Memory<'_>) -> Result<Cardinality, StorageError> {
        let mut exact = true;
        let mut total: u64 = u64::from(self.nan);
        for interval in &self.number.intervals {
            if !self.admits_with(interval, memory)? {
                continue;
            }
            match interval.degenerate_with(memory)?.and_then(Point::as_float) {
                Some(value) => total = total.saturating_add(if value == 0.0 { 2 } else { 1 }),
                None => {
                    exact = false;
                    total = total.saturating_add(1);
                }
            }
        }
        Ok(if exact {
            Cardinality::Exactly(total)
        } else {
            Cardinality::AtLeast(total)
        })
    }
    fn holds_with(&self, value: &XsdValue, memory: &mut Memory<'_>) -> Result<bool, StorageError> {
        let value = match value {
            XsdValue::Float(value) => f64::from(*value),
            XsdValue::Double(value) => *value,
            _ => return Ok(false),
        };
        if value.is_nan() {
            Ok(self.nan)
        } else {
            self.number.holds_with(&Point::float(value), memory)
        }
    }
}

/// The boolean space. XSD gives `xsd:boolean` no ordering or length facet, so the only
/// sets over it are the four subsets of `{false, true}`.
#[derive(Clone, Copy)]
struct BoolSet {
    /// Whether `false` is a member.
    has_false: bool,
    /// Whether `true` is a member.
    has_true: bool,
}

impl OwnedBytes for BoolSet {
    fn owned_bytes(&self) -> usize {
        0
    }
}
impl CloneOwned for BoolSet {
    fn clone_owned(&self, _: &mut Memory<'_>) -> Result<Self, StorageError> {
        Ok(*self)
    }
}

impl BoolSet {
    /// The whole space.
    fn full() -> Self {
        Self {
            has_false: true,
            has_true: true,
        }
    }

    /// The empty set.
    fn empty() -> Self {
        Self {
            has_false: false,
            has_true: false,
        }
    }

    /// A single value.
    fn point(b: bool) -> Self {
        Self {
            has_false: !b,
            has_true: b,
        }
    }
}

impl Algebra for BoolSet {
    type Value = XsdValue;
    fn complement_with(&self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        memory.step()?;
        Ok(Self {
            has_false: !self.has_false,
            has_true: !self.has_true,
        })
    }
    fn intersect_with(&self, other: &Self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        memory.step()?;
        Ok(Self {
            has_false: self.has_false && other.has_false,
            has_true: self.has_true && other.has_true,
        })
    }
    fn union_with(&self, other: &Self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        memory.step()?;
        Ok(Self {
            has_false: self.has_false || other.has_false,
            has_true: self.has_true || other.has_true,
        })
    }
    fn empty_with(&self, memory: &mut Memory<'_>) -> Result<bool, StorageError> {
        memory.step()?;
        Ok(!self.has_false && !self.has_true)
    }
    fn count_with(&self, memory: &mut Memory<'_>) -> Result<Cardinality, StorageError> {
        memory.step()?;
        Ok(Cardinality::Exactly(
            u64::from(self.has_false) + u64::from(self.has_true),
        ))
    }
    fn holds_with(&self, value: &XsdValue, memory: &mut Memory<'_>) -> Result<bool, StorageError> {
        memory.step()?;
        Ok(match value {
            XsdValue::Boolean(true) => self.has_true,
            XsdValue::Boolean(false) => self.has_false,
            _ => false,
        })
    }
}

/// Which unit a length-selected space counts.
#[derive(Clone, Copy, PartialEq, Eq)]
enum LengthKind {
    /// Unicode scalar values (`xsd:string`).
    Characters,
    /// Octets (`xsd:hexBinary`, `xsd:base64Binary`).
    Octets,
}

/// A lower bound on how many characters the `xsd:string` value space admits at one
/// position: the XML 1.0 `Char` production allows at least this many code points
/// (`#x9`, `#xA`, `#xD`, `[#x20-#xD7FF]`, `[#xE000-#xFFFD]`, `[#x10000-#x10FFFF]`).
/// Only ever used to LOWER-bound a count.
const STRING_ALPHABET_FLOOR: u64 = 3 + 55_264 + 8_190 + 1_048_576;

/// The alphabet of one octet.
const OCTET_ALPHABET: u64 = 256;

/// The octet length past which a space holds more values than `u64` can count.
const OCTET_SATURATION_LENGTH: u64 = 8;

/// A length-selected space: the values whose length is in `lengths`, plus a finite
/// `extras` set whose lengths are NOT, minus a finite `exceptions` set whose lengths
/// ARE.
///
/// The shape is closed under complement — swap `lengths` for its complement and
/// `extras` for `exceptions` — which is what makes it exact; intersection and union
/// follow from that closure.
#[derive(Clone)]
struct LengthSet {
    /// The unit lengths are counted in.
    kind: LengthKind,
    /// The admitted lengths.
    lengths: IntervalSet,
    /// Members whose length is not admitted.
    extras: Vec<XsdValue>,
    /// Non-members whose length is admitted.
    exceptions: Vec<XsdValue>,
}

impl OwnedBytes for LengthSet {
    fn owned_bytes(&self) -> usize {
        self.lengths.owned_bytes() + self.extras.owned_bytes() + self.exceptions.owned_bytes()
    }
}
impl CloneOwned for LengthSet {
    fn clone_owned(&self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        Ok(Self {
            kind: self.kind,
            lengths: self.lengths.clone_owned(memory)?,
            extras: self.extras.clone_owned(memory)?,
            exceptions: self.exceptions.clone_owned(memory)?,
        })
    }
}

impl LengthSet {
    fn full_with(kind: LengthKind, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        Ok(Self {
            kind,
            lengths: IntervalSet::full_with(memory)?,
            extras: Vec::new(),
            exceptions: Vec::new(),
        })
    }
    fn singleton_with(
        kind: LengthKind,
        value: XsdValue,
        memory: &mut Memory<'_>,
    ) -> Result<Self, StorageError> {
        Ok(Self {
            kind,
            lengths: IntervalSet::empty(),
            extras: memory.one(value)?,
            exceptions: Vec::new(),
        })
    }
    fn interval_with(
        kind: LengthKind,
        interval: Interval,
        memory: &mut Memory<'_>,
    ) -> Result<Self, StorageError> {
        let intervals = memory.one(interval)?;
        Ok(Self {
            kind,
            lengths: IntervalSet::canonical_with(intervals, memory)?,
            extras: Vec::new(),
            exceptions: Vec::new(),
        })
    }
    fn membership_with(
        &self,
        value: &XsdValue,
        memory: &mut Memory<'_>,
    ) -> Result<bool, StorageError> {
        let Some(length) = Self::length_of(value) else {
            return Ok(false);
        };
        if self.lengths.holds_with(&Point::Len(length), memory)? {
            Ok(!holds_same_with(&self.exceptions, value, memory)?)
        } else {
            holds_same_with(&self.extras, value, memory)
        }
    }
    fn probe_with(&self, memory: &mut Memory<'_>) -> Result<Vec<u64>, StorageError> {
        memory.scope(0, |memory| {
            let mut distinct = Vec::new();
            for length in self.exceptions.iter().filter_map(Self::length_of) {
                memory.step()?;
                if !distinct.contains(&length) {
                    memory.push(&mut distinct, length)?;
                }
            }
            self.lengths.sample_with(
                distinct
                    .len()
                    .checked_add(1)
                    .ok_or(StorageError::Allocation)?,
                memory,
            )
        })
    }
    fn selected_with(&self, memory: &mut Memory<'_>) -> Result<Cardinality, StorageError> {
        memory.scope(0, |memory| {
            if self.lengths_unbounded() {
                return Ok(Cardinality::Unbounded);
            }
            let Some(maximum) = self.max_length() else {
                return Ok(Cardinality::Exactly(0));
            };
            Ok(match self.kind {
                LengthKind::Characters => {
                    if maximum == 0 {
                        Cardinality::Exactly(1)
                    } else {
                        Cardinality::AtLeast(saturating_pow(STRING_ALPHABET_FLOOR, maximum))
                    }
                }
                LengthKind::Octets => {
                    if maximum >= OCTET_SATURATION_LENGTH {
                        Cardinality::AtLeast(u64::MAX)
                    } else {
                        let mut total = Cardinality::Exactly(0);
                        for length in self.lengths.sample_with(
                            usize::try_from(maximum).expect("below eight") + 1,
                            memory,
                        )? {
                            total = total
                                .plus(Cardinality::Exactly(saturating_pow(OCTET_ALPHABET, length)));
                        }
                        total
                    }
                }
            })
        })
    }

    /// The empty set.
    fn empty(kind: LengthKind) -> Self {
        Self {
            kind,
            lengths: IntervalSet::empty(),
            extras: Vec::new(),
            exceptions: Vec::new(),
        }
    }

    /// The length of a value in this space's unit, or `None` for a value from another
    /// space (which the caller's routing prevents).
    fn length_of(value: &XsdValue) -> Option<u64> {
        let n = match value {
            XsdValue::String(s) => s.chars().count(),
            XsdValue::Binary { bytes, .. } => bytes.len(),
            _ => return None,
        };
        Some(u64::try_from(n).unwrap_or(u64::MAX))
    }

    /// Whether the space holds more than `n` values of length `length`.
    ///
    /// Deliberately generous for characters: one character position admits over a
    /// million values, so only a length-0 string can be exhausted by a finite exception
    /// list. Over-reporting here can only withhold an `Empty` answer, never invent one.
    fn holds_more_than(&self, length: u64, n: usize) -> bool {
        match self.kind {
            LengthKind::Characters => length > 0 || n < 1,
            LengthKind::Octets => u128::from(saturating_pow(OCTET_ALPHABET, length)) > n as u128,
        }
    }

    /// The greatest admitted length, or `None` when the set admits none.
    fn max_length(&self) -> Option<u64> {
        let last = self.lengths.intervals.last()?;
        // An endpoint from another stratum cannot occur; reading it as the greatest
        // length keeps the impossible case on the widening side.
        match &last.hi {
            Hi::Unbounded => Some(u64::MAX),
            Hi::Incl(p) => Some(p.as_len().unwrap_or(u64::MAX)),
            Hi::Excl(p) => p.as_len().unwrap_or(u64::MAX).checked_sub(1),
        }
    }

    /// Whether the admitted lengths run without an upper bound.
    fn lengths_unbounded(&self) -> bool {
        self.lengths
            .intervals
            .last()
            .is_some_and(|iv| matches!(iv.hi, Hi::Unbounded))
    }
}

impl Algebra for LengthSet {
    type Value = XsdValue;
    fn complement_with(&self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        Ok(Self {
            kind: self.kind,
            lengths: self.lengths.complement_with(memory)?,
            extras: self.exceptions.clone_owned(memory)?,
            exceptions: self.extras.clone_owned(memory)?,
        })
    }
    fn intersect_with(&self, other: &Self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        memory.scope(0, |memory| {
            let lengths = self.lengths.intersect_with(&other.lengths, memory)?;
            let mut exceptions = Vec::new();
            for value in self.exceptions.iter().chain(&other.exceptions) {
                if length_admitted_with(&lengths, value, memory)?
                    && !holds_same_with(&exceptions, value, memory)?
                {
                    let value = value.clone_owned(memory)?;
                    memory.push(&mut exceptions, value)?;
                }
            }
            let mut extras = Vec::new();
            for value in self.extras.iter().chain(&other.extras) {
                if !length_admitted_with(&lengths, value, memory)?
                    && self.membership_with(value, memory)?
                    && other.membership_with(value, memory)?
                    && !holds_same_with(&extras, value, memory)?
                {
                    let value = value.clone_owned(memory)?;
                    memory.push(&mut extras, value)?;
                }
            }
            Ok(Self {
                kind: self.kind,
                lengths,
                extras,
                exceptions,
            })
        })
    }
    fn union_with(&self, other: &Self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        memory.scope(0, |memory| {
            let lengths = self.lengths.union_with(&other.lengths, memory)?;
            let mut extras = Vec::new();
            for value in self.extras.iter().chain(&other.extras) {
                if !length_admitted_with(&lengths, value, memory)?
                    && !holds_same_with(&extras, value, memory)?
                {
                    let value = value.clone_owned(memory)?;
                    memory.push(&mut extras, value)?;
                }
            }
            let mut exceptions = Vec::new();
            for value in self.exceptions.iter().chain(&other.exceptions) {
                if length_admitted_with(&lengths, value, memory)?
                    && !self.membership_with(value, memory)?
                    && !other.membership_with(value, memory)?
                    && !holds_same_with(&exceptions, value, memory)?
                {
                    let value = value.clone_owned(memory)?;
                    memory.push(&mut exceptions, value)?;
                }
            }
            Ok(Self {
                kind: self.kind,
                lengths,
                extras,
                exceptions,
            })
        })
    }
    fn empty_with(&self, memory: &mut Memory<'_>) -> Result<bool, StorageError> {
        if !self.extras.is_empty() {
            return Ok(false);
        }
        memory.scope(0, |memory| {
            for length in self.probe_with(memory)? {
                memory.step()?;
                let excluded = self
                    .exceptions
                    .iter()
                    .filter(|value| Self::length_of(value) == Some(length))
                    .count();
                if self.holds_more_than(length, excluded) {
                    return Ok(false);
                }
            }
            Ok(true)
        })
    }
    fn count_with(&self, memory: &mut Memory<'_>) -> Result<Cardinality, StorageError> {
        Ok(self
            .selected_with(memory)?
            .minus(u64::try_from(self.exceptions.len()).unwrap_or(u64::MAX))
            .plus(exactly(self.extras.len())))
    }
    fn holds_with(&self, value: &XsdValue, memory: &mut Memory<'_>) -> Result<bool, StorageError> {
        self.membership_with(value, memory)
    }
}

/// Whether a list already holds a value identical to `value`.
/// Equality law of one listed value space.
trait ListedValue: Clone + CloneOwned {
    fn same_with(&self, other: &Self, memory: &mut Memory<'_>) -> Result<bool, StorageError>;
}

impl ListedValue for XsdValue {
    fn same_with(&self, other: &Self, memory: &mut Memory<'_>) -> Result<bool, StorageError> {
        same_value_with(self, other, memory)
    }
}

impl ListedValue for u32 {
    fn same_with(&self, other: &Self, memory: &mut Memory<'_>) -> Result<bool, StorageError> {
        memory.step()?;
        Ok(self == other)
    }
}

fn holds_same_with<T: ListedValue>(
    values: &[T],
    value: &T,
    memory: &mut Memory<'_>,
) -> Result<bool, StorageError> {
    for held in values {
        if held.same_with(value, memory)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// `base.pow(exp)`, saturating at `u64::MAX`.
fn saturating_pow(base: u64, exp: u64) -> u64 {
    u32::try_from(exp).map_or(u64::MAX, |e| base.checked_pow(e).unwrap_or(u64::MAX))
}

/// A temporal space's set: a finite listed set of values, or the complement of one.
///
/// Those two shapes close under complement, intersection and union, so a temporal
/// enumeration and its complement are decided exactly. A bound facet over a temporal
/// space does NOT produce one of these shapes: the XSD order there is partial, so an
/// interval's complement is not a union of intervals.
#[derive(Clone)]
struct ListedSet<T> {
    /// The finite space size, or `None` for an infinite space.
    size: Option<u64>,
    /// Whether `values` lists the NON-members.
    negated: bool,
    /// The listed values, pairwise distinct under [`same_value`].
    values: Vec<T>,
}

impl<T: OwnedBytes> OwnedBytes for ListedSet<T> {
    fn owned_bytes(&self) -> usize {
        self.values.owned_bytes()
    }
}
impl<T: CloneOwned> CloneOwned for ListedSet<T> {
    fn clone_owned(&self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        Ok(Self {
            size: self.size,
            negated: self.negated,
            values: self.values.clone_owned(memory)?,
        })
    }
}

impl<T: ListedValue> ListedSet<T> {
    fn listed_with(
        size: Option<u64>,
        values: Vec<T>,
        memory: &mut Memory<'_>,
    ) -> Result<Self, StorageError> {
        memory.scope(values.owned_bytes(), |memory| {
            let mut deduped = Vec::new();
            memory.reserve(&mut deduped, values.len())?;
            for value in values {
                if !holds_same_with(&deduped, &value, memory)? {
                    deduped.push(value);
                }
            }
            Ok(Self {
                size,
                negated: false,
                values: deduped,
            })
        })
    }
    /// The whole space.
    fn full(size: Option<u64>) -> Self {
        Self {
            size,
            negated: true,
            values: Vec::new(),
        }
    }

    /// The empty set.
    fn empty(size: Option<u64>) -> Self {
        Self {
            size,
            negated: false,
            values: Vec::new(),
        }
    }

    /// A set over the same space with the given polarity and listed values.
    fn with(&self, negated: bool, values: Vec<T>) -> Self {
        Self {
            size: self.size,
            negated,
            values,
        }
    }
}

impl<T: ListedValue> Algebra for ListedSet<T> {
    type Value = T;
    fn complement_with(&self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        Ok(self.with(!self.negated, self.values.clone_owned(memory)?))
    }
    fn intersect_with(&self, other: &Self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        let (negated, left, right, relation) = match (self.negated, other.negated) {
            (false, false) => (false, &self.values, &other.values, ValueSelection::Common),
            (false, true) => (
                false,
                &self.values,
                &other.values,
                ValueSelection::Difference,
            ),
            (true, false) => (
                false,
                &other.values,
                &self.values,
                ValueSelection::Difference,
            ),
            (true, true) => (true, &self.values, &other.values, ValueSelection::Union),
        };
        Ok(self.with(negated, select_values(left, right, relation, memory)?))
    }
    fn union_with(&self, other: &Self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        let (negated, left, right, relation) = match (self.negated, other.negated) {
            (false, false) => (false, &self.values, &other.values, ValueSelection::Union),
            (false, true) => (
                true,
                &other.values,
                &self.values,
                ValueSelection::Difference,
            ),
            (true, false) => (
                true,
                &self.values,
                &other.values,
                ValueSelection::Difference,
            ),
            (true, true) => (true, &self.values, &other.values, ValueSelection::Common),
        };
        Ok(self.with(negated, select_values(left, right, relation, memory)?))
    }
    fn empty_with(&self, memory: &mut Memory<'_>) -> Result<bool, StorageError> {
        memory.step()?;
        Ok(if self.negated {
            self.size
                .is_some_and(|size| size <= u64::try_from(self.values.len()).unwrap_or(u64::MAX))
        } else {
            self.values.is_empty()
        })
    }
    fn count_with(&self, memory: &mut Memory<'_>) -> Result<Cardinality, StorageError> {
        memory.step()?;
        Ok(if self.negated {
            match self.size {
                None => Cardinality::Unbounded,
                Some(size) => Cardinality::Exactly(
                    size.saturating_sub(u64::try_from(self.values.len()).unwrap_or(u64::MAX)),
                ),
            }
        } else {
            exactly(self.values.len())
        })
    }
    fn holds_with(&self, value: &T, memory: &mut Memory<'_>) -> Result<bool, StorageError> {
        Ok(holds_same_with(&self.values, value, memory)? != self.negated)
    }
}

#[derive(Clone, Copy)]
enum ValueSelection {
    Union,
    Common,
    Difference,
}

fn select_values<T: ListedValue>(
    left: &[T],
    right: &[T],
    selection: ValueSelection,
    memory: &mut Memory<'_>,
) -> Result<Vec<T>, StorageError> {
    let mut out = Vec::new();
    memory.reserve(&mut out, left.len())?;
    for value in left {
        let keep = match selection {
            ValueSelection::Union => true,
            ValueSelection::Common => holds_same_with(right, value, memory)?,
            ValueSelection::Difference => !holds_same_with(right, value, memory)?,
        };
        if keep {
            out.push(value.clone_owned(memory)?);
        }
    }
    if matches!(selection, ValueSelection::Union) {
        for value in right {
            if !holds_same_with(&out, value, memory)? {
                let value = value.clone_owned(memory)?;
                memory.push(&mut out, value)?;
            }
        }
    }
    Ok(out)
}

fn length_admitted_with(
    lengths: &IntervalSet,
    value: &XsdValue,
    memory: &mut Memory<'_>,
) -> Result<bool, StorageError> {
    match LengthSet::length_of(value) {
        Some(length) => lengths.holds_with(&Point::Len(length), memory),
        None => Ok(false),
    }
}

// ── What is known about one space ─────────────────────────────────────────────────

/// What is known about a range's content in one value space.
#[derive(Clone)]
enum SpaceSet<A> {
    /// The set is exactly represented in that space's closed algebra.
    Exact(A),
    /// A member has been exhibited, but the set is not exactly represented.
    Inhabited,
    /// Nothing is known.
    Unknown,
}

impl<A: OwnedBytes> OwnedBytes for SpaceSet<A> {
    fn owned_bytes(&self) -> usize {
        match self {
            Self::Exact(value) => value.owned_bytes(),
            _ => 0,
        }
    }
}
impl<A: CloneOwned> CloneOwned for SpaceSet<A> {
    fn clone_owned(&self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        Ok(match self {
            Self::Exact(value) => Self::Exact(value.clone_owned(memory)?),
            Self::Inhabited => Self::Inhabited,
            Self::Unknown => Self::Unknown,
        })
    }
}

impl<A: Algebra> SpaceSet<A> {
    fn inhabited_with(&self, memory: &mut Memory<'_>) -> Result<bool, StorageError> {
        Ok(match self {
            Self::Exact(value) => !value.empty_with(memory)?,
            Self::Inhabited => true,
            Self::Unknown => false,
        })
    }
    fn complement_with(&self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        Ok(match self {
            Self::Exact(value) => Self::Exact(value.complement_with(memory)?),
            Self::Inhabited | Self::Unknown => Self::Unknown,
        })
    }
    fn intersect_with(&self, other: &Self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        Ok(match (self, other) {
            (Self::Exact(left), Self::Exact(right)) => {
                Self::Exact(left.intersect_with(right, memory)?)
            }
            (Self::Exact(left), _) if left.empty_with(memory)? => {
                Self::Exact(left.clone_owned(memory)?)
            }
            (_, Self::Exact(right)) if right.empty_with(memory)? => {
                Self::Exact(right.clone_owned(memory)?)
            }
            _ => Self::Unknown,
        })
    }
    fn union_with(&self, other: &Self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        if let (Self::Exact(left), Self::Exact(right)) = (self, other) {
            return Ok(Self::Exact(left.union_with(right, memory)?));
        }
        Ok(
            if self.inhabited_with(memory)? || other.inhabited_with(memory)? {
                Self::Inhabited
            } else {
                Self::Unknown
            },
        )
    }
    fn satisfiability_with(&self, memory: &mut Memory<'_>) -> Result<Satisfiability, StorageError> {
        Ok(match self {
            Self::Exact(value) if value.empty_with(memory)? => Satisfiability::Empty,
            Self::Exact(_) | Self::Inhabited => Satisfiability::Inhabited,
            Self::Unknown => Satisfiability::Undecided,
        })
    }
    fn count_with(&self, memory: &mut Memory<'_>) -> Result<Cardinality, StorageError> {
        match self {
            Self::Exact(value) => value.count_with(memory),
            Self::Inhabited => Ok(Cardinality::AtLeast(1)),
            Self::Unknown => Ok(Cardinality::Undecided),
        }
    }
    fn holds_with(&self, value: &A::Value, memory: &mut Memory<'_>) -> Result<Known, StorageError> {
        Ok(match self {
            Self::Exact(set) => {
                if set.holds_with(value, memory)? {
                    Known::Yes
                } else {
                    Known::No
                }
            }
            Self::Inhabited | Self::Unknown => Known::Unknown,
        })
    }

    /// Whether the set is exactly represented.
    fn is_exact(&self) -> bool {
        matches!(self, Self::Exact(_))
    }

    /// Whether this space's part of the range holds `value`.
    fn holds(&self, value: &A::Value) -> Known {
        resident(0, |memory| self.holds_with(value, memory))
    }
}

// ── The extent of a range across the whole data domain ───────────────────────────

/// What is known about a range in every value space, plus the remainder.
#[derive(Clone)]
struct Extent {
    /// The decimal space.
    decimal: SpaceSet<DecimalSet>,
    /// The `xsd:float` space.
    single: SpaceSet<FloatSet>,
    /// The `xsd:double` space.
    double: SpaceSet<FloatSet>,
    /// The `xsd:boolean` space.
    boolean: SpaceSet<BoolSet>,
    /// The length-selected spaces, in [`TEXT_SPACES`] order.
    text: [SpaceSet<LengthSet>; 3],
    /// The temporal spaces, in [`TEMPORAL_SPACES`] order.
    temporal: [SpaceSet<ListedSet<XsdValue>>; 10],
    /// The two disjoint, infinite RDF language-string spaces.
    term: [SpaceSet<ListedSet<u32>>; 2],
    /// Whether the range holds the values of the datatypes this module does not model.
    remainder: Known,
}

impl OwnedBytes for Extent {
    fn owned_bytes(&self) -> usize {
        self.decimal.owned_bytes()
            + self.single.owned_bytes()
            + self.double.owned_bytes()
            + self.boolean.owned_bytes()
            + self.text.owned_bytes()
            + self.temporal.owned_bytes()
            + self.term.owned_bytes()
    }
}

fn zip_spaces_with<A: Algebra, const N: usize>(
    left: &[SpaceSet<A>; N],
    right: &[SpaceSet<A>; N],
    mut operation: impl FnMut(
        &SpaceSet<A>,
        &SpaceSet<A>,
        &mut Memory<'_>,
    ) -> Result<SpaceSet<A>, StorageError>,
    memory: &mut Memory<'_>,
) -> Result<[SpaceSet<A>; N], StorageError> {
    memory.array(|index, memory| operation(&left[index], &right[index], memory))
}

impl Extent {
    fn full_with(memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        Ok(Self {
            decimal: SpaceSet::Exact(DecimalSet::full_with(memory)?),
            single: SpaceSet::Exact(FloatSet::full_with(FloatWidth::Single, memory)?),
            double: SpaceSet::Exact(FloatSet::full_with(FloatWidth::Double, memory)?),
            boolean: SpaceSet::Exact(BoolSet::full()),
            text: memory.array(|index, memory| {
                Ok(SpaceSet::Exact(LengthSet::full_with(
                    TEXT_SPACES[index].kind(),
                    memory,
                )?))
            })?,
            temporal: std::array::from_fn(|index| {
                SpaceSet::Exact(ListedSet::full(TEMPORAL_SPACES[index].size()))
            }),
            term: std::array::from_fn(|_| SpaceSet::Exact(ListedSet::full(None))),
            remainder: Known::Yes,
        })
    }
    fn complement_with(&self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        Ok(Self {
            decimal: self.decimal.complement_with(memory)?,
            single: self.single.complement_with(memory)?,
            double: self.double.complement_with(memory)?,
            boolean: self.boolean.complement_with(memory)?,
            text: memory.array(|index, memory| self.text[index].complement_with(memory))?,
            temporal: memory.array(|index, memory| self.temporal[index].complement_with(memory))?,
            term: memory.array(|index, memory| self.term[index].complement_with(memory))?,
            remainder: self.remainder.negate(),
        })
    }
    fn intersect_with(&self, other: &Self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        Ok(Self {
            decimal: self.decimal.intersect_with(&other.decimal, memory)?,
            single: self.single.intersect_with(&other.single, memory)?,
            double: self.double.intersect_with(&other.double, memory)?,
            boolean: self.boolean.intersect_with(&other.boolean, memory)?,
            text: zip_spaces_with(&self.text, &other.text, SpaceSet::intersect_with, memory)?,
            temporal: zip_spaces_with(
                &self.temporal,
                &other.temporal,
                SpaceSet::intersect_with,
                memory,
            )?,
            term: zip_spaces_with(&self.term, &other.term, SpaceSet::intersect_with, memory)?,
            remainder: self.remainder.conjoin(other.remainder),
        })
    }
    fn union_with(&self, other: &Self, memory: &mut Memory<'_>) -> Result<Self, StorageError> {
        Ok(Self {
            decimal: self.decimal.union_with(&other.decimal, memory)?,
            single: self.single.union_with(&other.single, memory)?,
            double: self.double.union_with(&other.double, memory)?,
            boolean: self.boolean.union_with(&other.boolean, memory)?,
            text: zip_spaces_with(&self.text, &other.text, SpaceSet::union_with, memory)?,
            temporal: zip_spaces_with(
                &self.temporal,
                &other.temporal,
                SpaceSet::union_with,
                memory,
            )?,
            term: zip_spaces_with(&self.term, &other.term, SpaceSet::union_with, memory)?,
            remainder: self.remainder.disjoin(other.remainder),
        })
    }
    fn satisfiability_with(&self, memory: &mut Memory<'_>) -> Result<Satisfiability, StorageError> {
        let verdicts = [
            self.decimal.satisfiability_with(memory)?,
            self.single.satisfiability_with(memory)?,
            self.double.satisfiability_with(memory)?,
            self.boolean.satisfiability_with(memory)?,
            match self.remainder {
                Known::Yes => Satisfiability::Inhabited,
                Known::No => Satisfiability::Empty,
                Known::Unknown => Satisfiability::Undecided,
            },
        ];
        let mut undecided = false;
        for verdict in verdicts {
            match verdict {
                Satisfiability::Inhabited => return Ok(Satisfiability::Inhabited),
                Satisfiability::Undecided => undecided = true,
                Satisfiability::Empty => {}
            }
        }
        macro_rules! spaces {
            ($sets:expr) => {
                for set in &$sets {
                    match set.satisfiability_with(memory)? {
                        Satisfiability::Inhabited => return Ok(Satisfiability::Inhabited),
                        Satisfiability::Undecided => undecided = true,
                        Satisfiability::Empty => {}
                    }
                }
            };
        }
        spaces!(self.text);
        spaces!(self.temporal);
        spaces!(self.term);
        Ok(if undecided {
            Satisfiability::Undecided
        } else {
            Satisfiability::Empty
        })
    }
    fn cardinality_with(&self, memory: &mut Memory<'_>) -> Result<Cardinality, StorageError> {
        let mut total = self
            .decimal
            .count_with(memory)?
            .plus(self.single.count_with(memory)?)
            .plus(self.double.count_with(memory)?)
            .plus(self.boolean.count_with(memory)?);
        macro_rules! spaces {
            ($sets:expr) => {
                for set in &$sets {
                    total = total.plus(set.count_with(memory)?);
                }
            };
        }
        spaces!(self.text);
        spaces!(self.temporal);
        spaces!(self.term);
        Ok(total.plus(match self.remainder {
            Known::Yes => Cardinality::Unbounded,
            Known::No => Cardinality::Exactly(0),
            Known::Unknown => Cardinality::Undecided,
        }))
    }
    fn contains_with(
        &self,
        value: &XsdValue,
        memory: &mut Memory<'_>,
    ) -> Result<Known, StorageError> {
        match space_of_value(value) {
            Space::Decimal => self.decimal.holds_with(value, memory),
            Space::Float => self.single.holds_with(value, memory),
            Space::Double => self.double.holds_with(value, memory),
            Space::Boolean => self.boolean.holds_with(value, memory),
            Space::Text(space) => self.text[space as usize].holds_with(value, memory),
            Space::Temporal(space) => self.temporal[space as usize].holds_with(value, memory),
        }
    }

    /// The empty range.
    fn empty() -> Self {
        Self {
            decimal: SpaceSet::Exact(DecimalSet::empty()),
            single: SpaceSet::Exact(FloatSet::empty(FloatWidth::Single)),
            double: SpaceSet::Exact(FloatSet::empty(FloatWidth::Double)),
            boolean: SpaceSet::Exact(BoolSet::empty()),
            text: std::array::from_fn(|i| SpaceSet::Exact(LengthSet::empty(TEXT_SPACES[i].kind()))),
            temporal: std::array::from_fn(|i| {
                SpaceSet::Exact(ListedSet::empty(TEMPORAL_SPACES[i].size()))
            }),
            term: std::array::from_fn(|_| SpaceSet::Exact(ListedSet::empty(None))),
            remainder: Known::No,
        }
    }

    /// Nothing known anywhere — an unmodelled value space may overlap a modelled one,
    /// so no space may be assumed empty.
    fn unknown() -> Self {
        Self {
            decimal: SpaceSet::Unknown,
            single: SpaceSet::Unknown,
            double: SpaceSet::Unknown,
            boolean: SpaceSet::Unknown,
            text: std::array::from_fn(|_| SpaceSet::Unknown),
            temporal: std::array::from_fn(|_| SpaceSet::Unknown),
            term: std::array::from_fn(|_| SpaceSet::Unknown),
            remainder: Known::Unknown,
        }
    }

    /// Whether the range is empty: every space empty AND the remainder excluded. One
    /// witness anywhere settles the answer regardless of what stays unknown.
    fn satisfiability(&self) -> Satisfiability {
        resident(0, |memory| self.satisfiability_with(memory))
    }

    /// The number of values, summed over the disjoint spaces.
    fn cardinality(&self) -> Cardinality {
        resident(0, |memory| self.cardinality_with(memory))
    }

    /// Whether every space is exactly represented and the remainder is determined.
    fn is_exact(&self) -> bool {
        self.decimal.is_exact()
            && self.single.is_exact()
            && self.double.is_exact()
            && self.boolean.is_exact()
            && self.text.iter().all(SpaceSet::is_exact)
            && self.temporal.iter().all(SpaceSet::is_exact)
            && self.term.iter().all(SpaceSet::is_exact)
            && self.remainder != Known::Unknown
    }

    /// Whether the range holds `value`. A value belongs to exactly one space, so only
    /// that space's set is consulted.
    fn contains(&self, value: &XsdValue) -> Known {
        resident(0, |memory| self.contains_with(value, memory))
    }
}

// ── From a data range to its extent ──────────────────────────────────────────────

fn extent_with(range: &DataRange, memory: &mut Memory<'_>) -> Result<Extent, StorageError> {
    memory.scope(0, |memory| {
        memory.step()?;
        Ok(match range {
            DataRange::Any => Extent::full_with(memory)?,
            DataRange::Opaque => Extent::unknown(),
            DataRange::Datatype(datatype) => datatype_with(*datatype, memory)?,
            DataRange::TermDatatype(space) => {
                let mut extent = Extent::empty();
                extent.term[*space as usize] = SpaceSet::Exact(ListedSet::full(None));
                extent
            }
            DataRange::TermOneOf { space, values } => {
                let mut extent = Extent::empty();
                let values = memory.copy(values)?;
                extent.term[*space as usize] =
                    SpaceSet::Exact(ListedSet::listed_with(None, values, memory)?);
                extent
            }
            DataRange::Restriction { base, facets } => restriction_with(*base, facets, memory)?,
            DataRange::OneOf(values) => {
                let mut accumulator = Extent::empty();
                for value in values {
                    let part = value_with(value, memory)?;
                    let next = accumulator.union_with(&part, memory)?;
                    memory.drop_value(accumulator)?;
                    memory.drop_value(part)?;
                    accumulator = next;
                }
                accumulator
            }
            DataRange::Not(inner) => {
                let inner = extent_with(inner, memory)?;
                let result = inner.complement_with(memory)?;
                memory.drop_value(inner)?;
                result
            }
            DataRange::And(operands) | DataRange::Or(operands) => {
                let conjunction = matches!(range, DataRange::And(_));
                let mut accumulator = if conjunction {
                    Extent::full_with(memory)?
                } else {
                    Extent::empty()
                };
                for operand in operands {
                    let part = extent_with(operand, memory)?;
                    let next = if conjunction {
                        accumulator.intersect_with(&part, memory)?
                    } else {
                        accumulator.union_with(&part, memory)?
                    };
                    memory.drop_value(accumulator)?;
                    memory.drop_value(part)?;
                    accumulator = next;
                }
                accumulator
            }
        })
    })
}

fn datatype_with(datatype: XsdDatatype, memory: &mut Memory<'_>) -> Result<Extent, StorageError> {
    let mut extent = Extent::empty();
    match space_of_datatype(datatype) {
        Space::Decimal => {
            extent.decimal = SpaceSet::Exact(DecimalSet::datatype_with(datatype, memory)?);
        }
        Space::Float => {
            extent.single = SpaceSet::Exact(FloatSet::full_with(FloatWidth::Single, memory)?);
        }
        Space::Double => {
            extent.double = SpaceSet::Exact(FloatSet::full_with(FloatWidth::Double, memory)?);
        }
        Space::Boolean => extent.boolean = SpaceSet::Exact(BoolSet::full()),
        Space::Text(space) => {
            extent.text[space as usize] =
                SpaceSet::Exact(LengthSet::full_with(space.kind(), memory)?);
        }
        Space::Temporal(space) => {
            extent.temporal[space as usize] = if covers_whole_space(datatype) {
                SpaceSet::Exact(ListedSet::full(space.size()))
            } else {
                SpaceSet::Inhabited
            }
        }
    }
    if datatype == XsdDatatype::DateTime {
        extent.temporal[TemporalSpace::DateTimeUnzoned as usize] =
            SpaceSet::Exact(ListedSet::full(None));
    }
    Ok(extent)
}

fn value_with(value: &XsdValue, memory: &mut Memory<'_>) -> Result<Extent, StorageError> {
    let mut extent = Extent::empty();
    match space_of_value(value) {
        Space::Decimal => match memory.decimal_of(value)? {
            Some(value) => extent.decimal = SpaceSet::Exact(DecimalSet::point_with(value, memory)?),
            None => return Ok(Extent::unknown()),
        },
        Space::Float => extent.single = float_point_with(FloatWidth::Single, value, memory)?,
        Space::Double => extent.double = float_point_with(FloatWidth::Double, value, memory)?,
        Space::Boolean => match value {
            XsdValue::Boolean(value) => extent.boolean = SpaceSet::Exact(BoolSet::point(*value)),
            _ => return Ok(Extent::unknown()),
        },
        Space::Text(space) => {
            let value = value.clone_owned(memory)?;
            extent.text[space as usize] =
                SpaceSet::Exact(LengthSet::singleton_with(space.kind(), value, memory)?);
        }
        Space::Temporal(space) => {
            let value = value.clone_owned(memory)?;
            let values = memory.one(value)?;
            extent.temporal[space as usize] =
                SpaceSet::Exact(ListedSet::listed_with(space.size(), values, memory)?);
        }
    }
    Ok(extent)
}

fn float_point_with(
    width: FloatWidth,
    value: &XsdValue,
    memory: &mut Memory<'_>,
) -> Result<SpaceSet<FloatSet>, StorageError> {
    let value = match value {
        XsdValue::Float(value) => f64::from(*value),
        XsdValue::Double(value) => *value,
        _ => return Ok(SpaceSet::Unknown),
    };
    Ok(if value.is_nan() {
        SpaceSet::Exact(FloatSet {
            width,
            number: IntervalSet::empty(),
            nan: true,
        })
    } else if value == 0.0 {
        SpaceSet::Inhabited
    } else {
        SpaceSet::Exact(FloatSet {
            width,
            number: IntervalSet::point_with(Point::float(value), memory)?,
            nan: false,
        })
    })
}

fn restriction_with(
    base: XsdDatatype,
    facets: &[Facet],
    memory: &mut Memory<'_>,
) -> Result<Extent, StorageError> {
    let mut extent = Extent::empty();
    match space_of_datatype(base) {
        Space::Decimal => extent.decimal = decimal_restriction_with(base, facets, memory)?,
        Space::Float => extent.single = float_restriction_with(FloatWidth::Single, facets, memory)?,
        Space::Double => {
            extent.double = float_restriction_with(FloatWidth::Double, facets, memory)?;
        }
        Space::Boolean => {
            extent.boolean = if facets.is_empty() {
                SpaceSet::Exact(BoolSet::full())
            } else {
                SpaceSet::Unknown
            }
        }
        Space::Text(space) => {
            extent.text[space as usize] = length_restriction_with(space.kind(), facets, memory)?;
        }
        Space::Temporal(space) => {
            extent.temporal[space as usize] =
                temporal_restriction_with(base, space, facets, memory)?;
        }
    }
    if base == XsdDatatype::DateTime {
        extent.temporal[TemporalSpace::DateTimeUnzoned as usize] =
            temporal_restriction_with(base, TemporalSpace::DateTimeUnzoned, facets, memory)?;
    }
    Ok(extent)
}

fn decimal_restriction_with(
    base: XsdDatatype,
    facets: &[Facet],
    memory: &mut Memory<'_>,
) -> Result<SpaceSet<DecimalSet>, StorageError> {
    memory.scope(0, |memory| {
        let mut set = DecimalSet::datatype_with(base, memory)?;
        for facet in facets {
            memory.step()?;
            let Some((side, value)) = BoundSide::of(facet) else {
                return Ok(SpaceSet::Unknown);
            };
            let Some(value) = memory.decimal_of(value)? else {
                return Ok(SpaceSet::Unknown);
            };
            let part = DecimalSet::interval_with(side.interval(Point::Dec(value)), memory)?;
            let next = set.intersect_with(&part, memory)?;
            memory.drop_value(set)?;
            memory.drop_value(part)?;
            set = next;
        }
        Ok(SpaceSet::Exact(set))
    })
}

fn float_restriction_with(
    width: FloatWidth,
    facets: &[Facet],
    memory: &mut Memory<'_>,
) -> Result<SpaceSet<FloatSet>, StorageError> {
    memory.scope(0, |memory| {
        let mut set = FloatSet::full_with(width, memory)?;
        for facet in facets {
            memory.step()?;
            let Some((side, value)) = BoundSide::of(facet) else {
                return Ok(SpaceSet::Unknown);
            };
            let bound = match (width, value) {
                (FloatWidth::Single, XsdValue::Float(value)) => f64::from(*value),
                (FloatWidth::Double, XsdValue::Double(value)) => *value,
                _ => return Ok(SpaceSet::Unknown),
            };
            if bound.is_nan() {
                return Ok(SpaceSet::Unknown);
            }
            let part = FloatSet::interval_with(width, side.interval(Point::float(bound)), memory)?;
            let next = set.intersect_with(&part, memory)?;
            memory.drop_value(set)?;
            memory.drop_value(part)?;
            set = next;
        }
        Ok(SpaceSet::Exact(set))
    })
}

fn length_restriction_with(
    kind: LengthKind,
    facets: &[Facet],
    memory: &mut Memory<'_>,
) -> Result<SpaceSet<LengthSet>, StorageError> {
    memory.scope(0, |memory| {
        let mut set = LengthSet::full_with(kind, memory)?;
        for facet in facets {
            memory.step()?;
            let Some(interval) = length_interval(facet) else {
                return Ok(SpaceSet::Unknown);
            };
            let part = LengthSet::interval_with(kind, interval, memory)?;
            let next = set.intersect_with(&part, memory)?;
            memory.drop_value(set)?;
            memory.drop_value(part)?;
            set = next;
        }
        Ok(SpaceSet::Exact(set))
    })
}

/// The extent of a data range across the whole data domain.
fn extent(range: &DataRange) -> Extent {
    resident(0, |memory| extent_with(range, memory))
}

/// Which side of an interval a bound facet constrains.
#[derive(Clone, Copy, PartialEq, Eq)]
enum BoundSide {
    /// A lower bound, inclusive.
    LowerInclusive,
    /// A lower bound, exclusive.
    LowerExclusive,
    /// An upper bound, inclusive.
    UpperInclusive,
    /// An upper bound, exclusive.
    UpperExclusive,
}

impl BoundSide {
    /// The side and value a bound facet carries, or `None` for a length facet.
    fn of(facet: &Facet) -> Option<(Self, &XsdValue)> {
        Some(match facet {
            Facet::MinInclusive(v) => (Self::LowerInclusive, v),
            Facet::MinExclusive(v) => (Self::LowerExclusive, v),
            Facet::MaxInclusive(v) => (Self::UpperInclusive, v),
            Facet::MaxExclusive(v) => (Self::UpperExclusive, v),
            Facet::Length(_) | Facet::MinLength(_) | Facet::MaxLength(_) => return None,
        })
    }

    /// Whether this bound is a lower one.
    fn is_lower(self) -> bool {
        matches!(self, Self::LowerInclusive | Self::LowerExclusive)
    }

    /// Whether this bound admits its own endpoint.
    fn is_inclusive(self) -> bool {
        matches!(self, Self::LowerInclusive | Self::UpperInclusive)
    }

    /// The half-line this bound admits over an endpoint domain.
    fn interval(self, v: Point) -> Interval {
        match self {
            Self::LowerInclusive => Interval {
                lo: Lo::Incl(v),
                hi: Hi::Unbounded,
            },
            Self::LowerExclusive => Interval {
                lo: Lo::Excl(v),
                hi: Hi::Unbounded,
            },
            Self::UpperInclusive => Interval {
                lo: Lo::Unbounded,
                hi: Hi::Incl(v),
            },
            Self::UpperExclusive => Interval {
                lo: Lo::Unbounded,
                hi: Hi::Excl(v),
            },
        }
    }
}

/// The length interval a length facet admits, or `None` for a bound facet.
fn length_interval(facet: &Facet) -> Option<Interval> {
    Some(match facet {
        Facet::Length(n) => Interval::point(Point::Len(*n)),
        Facet::MinLength(n) => Interval {
            lo: Lo::Incl(Point::Len(*n)),
            hi: Hi::Unbounded,
        },
        Facet::MaxLength(n) => Interval {
            lo: Lo::Unbounded,
            hi: Hi::Incl(Point::Len(*n)),
        },
        Facet::MinInclusive(_)
        | Facet::MinExclusive(_)
        | Facet::MaxInclusive(_)
        | Facet::MaxExclusive(_) => return None,
    })
}

/// What a set of bound facets over a temporal space settles.
#[derive(Clone, Copy, PartialEq, Eq)]
enum TemporalBounds {
    /// No facet constrains the space.
    Unconstrained,
    /// Two bounds contradict: the range is empty, exactly.
    Contradiction,
    /// An inclusive endpoint satisfies every bound.
    Witnessed,
    /// The partial order settles neither emptiness nor inhabitation.
    Indeterminate,
}

fn temporal_restriction_with(
    base: XsdDatatype,
    space: TemporalSpace,
    facets: &[Facet],
    memory: &mut Memory<'_>,
) -> Result<SpaceSet<ListedSet<XsdValue>>, StorageError> {
    Ok(match temporal_bounds_with(space, facets, memory)? {
        TemporalBounds::Contradiction => SpaceSet::Exact(ListedSet::empty(space.size())),
        TemporalBounds::Unconstrained => {
            if covers_whole_space(base) {
                SpaceSet::Exact(ListedSet::full(space.size()))
            } else {
                SpaceSet::Inhabited
            }
        }
        TemporalBounds::Witnessed if covers_whole_space(base) => SpaceSet::Inhabited,
        TemporalBounds::Witnessed | TemporalBounds::Indeterminate => SpaceSet::Unknown,
    })
}

fn temporal_bounds_with(
    space: TemporalSpace,
    facets: &[Facet],
    memory: &mut Memory<'_>,
) -> Result<TemporalBounds, StorageError> {
    // The original bound list contains only borrowed source facets. Scan those
    // facets directly; no copied value or metadata vector is needed.
    for facet in facets {
        memory.step()?;
        let Some((_, value)) = BoundSide::of(facet) else {
            return Ok(TemporalBounds::Indeterminate);
        };
        if space_of_value(value) != Space::Temporal(space)
            && !(matches!(
                space,
                TemporalSpace::DateTime | TemporalSpace::DateTimeUnzoned
            ) && matches!(value, XsdValue::DateTime(_)))
        {
            return Ok(TemporalBounds::Indeterminate);
        }
    }
    if facets.is_empty() {
        return Ok(TemporalBounds::Unconstrained);
    }
    for (lower_side, lower) in facets
        .iter()
        .filter_map(BoundSide::of)
        .filter(|(side, _)| side.is_lower())
    {
        for (upper_side, upper) in facets
            .iter()
            .filter_map(BoundSide::of)
            .filter(|(side, _)| !side.is_lower())
        {
            memory.step()?;
            match value_cmp(lower, upper) {
                Some(Ordering::Greater) => return Ok(TemporalBounds::Contradiction),
                Some(Ordering::Equal)
                    if !(lower_side.is_inclusive() && upper_side.is_inclusive()) =>
                {
                    return Ok(TemporalBounds::Contradiction);
                }
                _ => {}
            }
        }
    }
    for (_, candidate) in facets
        .iter()
        .filter_map(BoundSide::of)
        .filter(|(side, candidate)| {
            side.is_inclusive() && space_of_value(candidate) == Space::Temporal(space)
        })
    {
        let mut admitted = true;
        for (side, bound) in facets.iter().filter_map(BoundSide::of) {
            memory.step()?;
            if satisfies_bound(candidate, side, bound) != Some(true) {
                admitted = false;
                break;
            }
        }
        if admitted {
            return Ok(TemporalBounds::Witnessed);
        }
    }
    Ok(TemporalBounds::Indeterminate)
}

/// Whether `candidate` satisfies one bound, or `None` when the two are incomparable.
fn satisfies_bound(candidate: &XsdValue, side: BoundSide, bound: &XsdValue) -> Option<bool> {
    let ordering = value_cmp(candidate, bound)?;
    Some(match side {
        BoundSide::LowerInclusive => ordering != Ordering::Less,
        BoundSide::LowerExclusive => ordering == Ordering::Greater,
        BoundSide::UpperInclusive => ordering != Ordering::Greater,
        BoundSide::UpperExclusive => ordering == Ordering::Less,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::parse;

    /// Parse a lexical form that the test knows is valid.
    fn v(lexical: &str, dt: XsdDatatype) -> XsdValue {
        parse(lexical, dt).unwrap_or_else(|e| panic!("parse({lexical:?}, {dt:?}) failed: {e}"))
    }

    /// A decimal endpoint from an integer.
    fn dec(n: i128) -> Point {
        Point::Dec(exact::Decimal::from(n))
    }

    // ── The interval algebra ──────────────────────────────────────────────────────

    #[test]
    fn complement_of_a_closed_interval_is_its_two_gaps() {
        let set = IntervalSet::canonical(vec![Interval {
            lo: Lo::Incl(dec(1)),
            hi: Hi::Incl(dec(3)),
        }]);
        let gaps = set.complement();
        assert_eq!(gaps.intervals.len(), 2);
        assert!(gaps.holds(&dec(0)));
        assert!(!gaps.holds(&dec(1)));
        assert!(!gaps.holds(&dec(2)));
        assert!(!gaps.holds(&dec(3)));
        assert!(gaps.holds(&dec(4)));
        // Complement is an involution on this shape.
        let round_trip = gaps.complement();
        assert!(round_trip.holds(&dec(2)));
        assert!(!round_trip.holds(&dec(4)));
    }

    #[test]
    fn complement_of_the_full_set_is_empty_and_back() {
        let full = IntervalSet::full();
        assert!(full.complement().intervals.is_empty());
        assert!(IntervalSet::empty().complement().holds(&dec(7)));
    }

    #[test]
    fn touching_intervals_merge_and_separated_ones_do_not() {
        let merged = IntervalSet::canonical(vec![
            Interval {
                lo: Lo::Incl(dec(1)),
                hi: Hi::Excl(dec(2)),
            },
            Interval {
                lo: Lo::Incl(dec(2)),
                hi: Hi::Incl(dec(3)),
            },
        ]);
        assert_eq!(merged.intervals.len(), 1);
        let separated = IntervalSet::canonical(vec![
            Interval {
                lo: Lo::Incl(dec(1)),
                hi: Hi::Incl(dec(2)),
            },
            Interval {
                lo: Lo::Incl(dec(4)),
                hi: Hi::Incl(dec(5)),
            },
        ]);
        assert_eq!(separated.intervals.len(), 2);
    }

    #[test]
    fn intersection_takes_the_tighter_end_of_each_side() {
        let a = IntervalSet::canonical(vec![Interval {
            lo: Lo::Incl(dec(1)),
            hi: Hi::Incl(dec(10)),
        }]);
        let b = IntervalSet::canonical(vec![Interval {
            lo: Lo::Excl(dec(5)),
            hi: Hi::Unbounded,
        }]);
        let both = a.intersect(&b);
        assert!(!both.holds(&dec(5)));
        assert!(both.holds(&dec(6)));
        assert!(both.holds(&dec(10)));
        assert!(!both.holds(&dec(11)));
    }

    // ── The integral stratum's window arithmetic ──────────────────────────────────

    #[test]
    fn integral_window_rounds_bounds_inward() {
        let half = Point::Dec("1.5".parse().expect("a decimal"));
        let window = DecimalSet::window(&Interval {
            lo: Lo::Incl(half),
            hi: Hi::Incl(dec(4)),
        })
        .expect("1.5 ..= 4 holds integers");
        assert_eq!(
            (window.lo, window.hi),
            (Some(exact::Integer::from(2)), Some(exact::Integer::from(4)))
        );
    }

    #[test]
    fn exclusive_integer_bounds_step_past_the_endpoint() {
        let window = DecimalSet::window(&Interval {
            lo: Lo::Excl(dec(3)),
            hi: Hi::Excl(dec(5)),
        })
        .expect("3 < n < 5 holds 4");
        assert_eq!(
            (window.lo, window.hi),
            (Some(exact::Integer::from(4)), Some(exact::Integer::from(4)))
        );
        // Nothing lies strictly between 3 and 4.
        assert!(
            DecimalSet::window(&Interval {
                lo: Lo::Excl(dec(3)),
                hi: Hi::Excl(dec(4)),
            })
            .is_none()
        );
    }

    #[test]
    fn unbounded_integer_ends_stay_unbounded() {
        let window = DecimalSet::window(&Interval::full()).expect("all integers");
        assert_eq!((window.lo, window.hi), (None, None));
    }

    // ── The length-selected shape ─────────────────────────────────────────────────

    #[test]
    fn length_set_complement_swaps_lengths_and_the_two_finite_lists() {
        let set = LengthSet {
            kind: LengthKind::Characters,
            lengths: IntervalSet::canonical(vec![Interval::point(Point::Len(1))]),
            extras: vec![v("ab", XsdDatatype::String)],
            exceptions: vec![v("x", XsdDatatype::String)],
        };
        assert!(set.holds(&v("y", XsdDatatype::String)));
        assert!(!set.holds(&v("x", XsdDatatype::String)));
        assert!(set.holds(&v("ab", XsdDatatype::String)));

        let other = set.complement();
        assert!(!other.holds(&v("y", XsdDatatype::String)));
        assert!(other.holds(&v("x", XsdDatatype::String)));
        assert!(!other.holds(&v("ab", XsdDatatype::String)));
    }

    #[test]
    fn a_length_zero_string_space_holds_exactly_the_empty_string() {
        let set = LengthSet {
            kind: LengthKind::Characters,
            lengths: IntervalSet::canonical(vec![Interval::point(Point::Len(0))]),
            extras: Vec::new(),
            exceptions: Vec::new(),
        };
        assert_eq!(set.count(), Cardinality::Exactly(1));
        assert!(!set.is_empty());
        let minus_empty = LengthSet {
            exceptions: vec![v("", XsdDatatype::String)],
            ..set
        };
        assert_eq!(minus_empty.count(), Cardinality::Exactly(0));
        assert!(minus_empty.is_empty());
    }

    #[test]
    fn octet_lengths_count_exactly() {
        let set = LengthSet {
            kind: LengthKind::Octets,
            lengths: IntervalSet::canonical(vec![Interval {
                lo: Lo::Incl(Point::Len(0)),
                hi: Hi::Incl(Point::Len(1)),
            }]),
            extras: Vec::new(),
            exceptions: Vec::new(),
        };
        // The empty sequence plus every single octet.
        assert_eq!(set.count(), Cardinality::Exactly(257));
    }

    // ── The lattices ──────────────────────────────────────────────────────────────

    #[test]
    fn kleene_lattice_keeps_proofs() {
        assert_eq!(Known::No.conjoin(Known::Unknown), Known::No);
        assert_eq!(Known::Yes.conjoin(Known::Unknown), Known::Unknown);
        assert_eq!(Known::Yes.disjoin(Known::Unknown), Known::Yes);
        assert_eq!(Known::No.disjoin(Known::Unknown), Known::Unknown);
        assert_eq!(Known::Unknown.negate(), Known::Unknown);
        assert_eq!(Known::Yes.negate(), Known::No);
    }

    #[test]
    fn counting_lattice_loses_exactness_but_not_soundness() {
        assert_eq!(
            Cardinality::Exactly(2).plus(Cardinality::Exactly(3)),
            Cardinality::Exactly(5)
        );
        assert_eq!(
            Cardinality::Exactly(2).plus(Cardinality::AtLeast(3)),
            Cardinality::AtLeast(5)
        );
        assert_eq!(
            Cardinality::Unbounded.plus(Cardinality::Exactly(3)),
            Cardinality::Unbounded
        );
        assert_eq!(
            Cardinality::Undecided.plus(Cardinality::Unbounded),
            Cardinality::Undecided
        );
        assert_eq!(
            Cardinality::Exactly(u64::MAX).plus(Cardinality::Exactly(2)),
            Cardinality::AtLeast(u64::MAX)
        );
    }

    /// A span past `i64::MAX` and inside `u64` is counted exactly: every
    /// `xsd:unsignedLong` but zero is `2^64 − 1` values, and the whole datatype is
    /// past `u64::MAX` by one.
    #[test]
    fn an_unsigned_long_span_past_i64_is_counted_exactly() {
        let at_least_one = DataRange::Restriction {
            base: XsdDatatype::UnsignedLong,
            facets: vec![Facet::MinInclusive(v("1", XsdDatatype::UnsignedLong))],
        };
        assert_eq!(cardinality(&at_least_one), Cardinality::Exactly(u64::MAX));
        assert_eq!(
            cardinality(&DataRange::Datatype(XsdDatatype::UnsignedLong)),
            Cardinality::AtLeast(u64::MAX)
        );
        // Neighbour: a span inside i64 is exact, as it always was.
        let small = DataRange::Restriction {
            base: XsdDatatype::UnsignedLong,
            facets: vec![
                Facet::MinInclusive(v("1", XsdDatatype::UnsignedLong)),
                Facet::MaxInclusive(v("10", XsdDatatype::UnsignedLong)),
            ],
        };
        assert_eq!(cardinality(&small), Cardinality::Exactly(10));
    }

    // ── Value-space identity ──────────────────────────────────────────────────────

    #[test]
    fn one_nan_per_value_space() {
        let nan = v("NaN", XsdDatatype::Double);
        assert!(same_value(&nan, &nan));
        assert!(!value_eq(&nan, &nan));
        // The float NaN and the double NaN are values of DIFFERENT spaces.
        assert!(!same_value(&nan, &v("NaN", XsdDatatype::Float)));
    }

    #[test]
    fn identity_holds_only_within_one_value_space() {
        let integer = v("5", XsdDatatype::Integer);
        let byte = v("5", XsdDatatype::Byte);
        let decimal = v("5.0", XsdDatatype::Decimal);
        let float = v("5", XsdDatatype::Float);
        let double = v("5", XsdDatatype::Double);

        // The integer value space is a subset of the decimal value space.
        assert!(same_value(&integer, &decimal));
        assert!(same_value(&integer, &byte));
        // float, double and decimal are pairwise disjoint in OWL 2's datatype map.
        assert!(!same_value(&decimal, &float));
        assert!(!same_value(&decimal, &double));
        assert!(!same_value(&float, &double));
        // SPARQL `=` promotes across all four; that is the other question.
        for other in [&decimal, &float, &double] {
            assert!(value_eq(&integer, other));
        }
    }

    #[test]
    fn identity_agrees_with_value_eq_inside_one_space() {
        let rows = [
            ("1", "01", XsdDatatype::Integer, true),
            ("1", "2", XsdDatatype::Integer, false),
            ("abc", "abc", XsdDatatype::String, true),
            ("abc", "abd", XsdDatatype::String, false),
            ("true", "1", XsdDatatype::Boolean, true),
            ("1.5", "1.50", XsdDatatype::Decimal, true),
            ("2000-01-01", "2000-01-01", XsdDatatype::Date, true),
            ("2000-01-01", "2000-01-02", XsdDatatype::Date, false),
            ("2.5", "2.5", XsdDatatype::Float, true),
            ("2.5", "2.6", XsdDatatype::Float, false),
        ];
        for (left, right, dt, want) in rows {
            let (a, b) = (v(left, dt), v(right, dt));
            assert_eq!(
                same_value(&a, &b),
                want,
                "same_value({left}, {right}, {dt:?})"
            );
            assert_eq!(
                value_eq(&a, &b),
                want,
                "value_eq disagrees for ({left}, {right}, {dt:?})"
            );
        }
    }

    #[test]
    fn timezone_shifted_datetimes_are_one_value() {
        let utc = v("2002-10-10T17:00:00Z", XsdDatatype::DateTime);
        let offset = v("2002-10-10T12:00:00-05:00", XsdDatatype::DateTime);
        assert!(same_value(&utc, &offset));
    }

    #[test]
    fn end_of_day_time_is_midnight() {
        let midnight = v("00:00:00Z", XsdDatatype::Time);
        let end_of_day = v("24:00:00Z", XsdDatatype::Time);
        assert!(same_value(&midnight, &end_of_day));
        assert!(same_value(
            &v("00:00:00", XsdDatatype::Time),
            &v("24:00:00", XsdDatatype::Time)
        ));
    }

    #[test]
    fn the_two_zeros_of_a_float_space_share_one_ordered_point() {
        let positive = v("0.0", XsdDatatype::Double);
        let negative = v("-0.0", XsdDatatype::Double);
        // The order these spaces carry cannot separate them.
        assert!(same_value(&positive, &negative));
        // A bound facet pinning zero therefore holds BOTH values of the value space.
        let pinned = DataRange::Restriction {
            base: XsdDatatype::Double,
            facets: vec![
                Facet::MinInclusive(positive.clone()),
                Facet::MaxInclusive(positive),
            ],
        };
        assert_eq!(cardinality(&pinned), Cardinality::Exactly(2));
    }

    // ── Space routing ─────────────────────────────────────────────────────────────

    #[test]
    fn every_datatype_lands_in_a_space() {
        assert_eq!(space_of_datatype(XsdDatatype::Byte), Space::Decimal);
        assert_eq!(space_of_datatype(XsdDatatype::Decimal), Space::Decimal);
        assert_eq!(
            space_of_datatype(XsdDatatype::HexBinary),
            Space::Text(TextSpace::HexBinary)
        );
        // The duration family shares ONE space: the value spaces overlap at zero.
        for dt in [
            XsdDatatype::Duration,
            XsdDatatype::DayTimeDuration,
            XsdDatatype::YearMonthDuration,
        ] {
            assert_eq!(
                space_of_datatype(dt),
                Space::Temporal(TemporalSpace::Duration)
            );
        }
        assert!(same_value(
            &v("P0M", XsdDatatype::YearMonthDuration),
            &v("PT0S", XsdDatatype::DayTimeDuration)
        ));
    }

    #[test]
    fn the_finite_gregorian_spaces_have_exact_sizes() {
        assert_eq!(TZ_SETTINGS, 1682);
        assert_eq!(TemporalSpace::GMonth.size(), Some(12 * 1682));
        assert_eq!(TemporalSpace::GDay.size(), Some(31 * 1682));
        assert_eq!(TemporalSpace::GMonthDay.size(), Some(366 * 1682));
        assert_eq!(TemporalSpace::Date.size(), None);
        assert_eq!(TemporalSpace::Duration.size(), None);
    }

    // ── Containment ──────────────────────────────────────────────────────────────

    /// The XSD integer tower NESTS, and containment is asymmetric along it.
    #[test]
    fn containment_follows_the_integer_tower() {
        let nested = [
            (XsdDatatype::Byte, XsdDatatype::Short),
            (XsdDatatype::Short, XsdDatatype::Int),
            (XsdDatatype::Int, XsdDatatype::Long),
            (XsdDatatype::Long, XsdDatatype::Integer),
            (XsdDatatype::Integer, XsdDatatype::Decimal),
            (XsdDatatype::UnsignedByte, XsdDatatype::UnsignedShort),
            (XsdDatatype::NonNegativeInteger, XsdDatatype::Integer),
        ];
        for (sub, sup) in nested {
            assert_eq!(
                containment(&DataRange::Datatype(sub), &DataRange::Datatype(sup)),
                Satisfiability::Empty,
                "{sub:?} is contained in {sup:?}"
            );
            assert_eq!(
                containment(&DataRange::Datatype(sup), &DataRange::Datatype(sub)),
                Satisfiability::Inhabited,
                "{sup:?} is NOT contained in {sub:?}"
            );
        }
    }

    /// An INTERSECTION of two datatypes can be contained in a third that contains neither.
    #[test]
    fn an_intersection_can_be_contained_where_no_operand_is() {
        // short ⊓ unsignedInt ⊆ unsignedShort, while neither operand is.
        let both = DataRange::And(vec![
            DataRange::Datatype(XsdDatatype::Short),
            DataRange::Datatype(XsdDatatype::UnsignedInt),
        ]);
        let target = DataRange::Datatype(XsdDatatype::UnsignedShort);
        assert_eq!(containment(&both, &target), Satisfiability::Empty);
        assert_eq!(
            containment(&DataRange::Datatype(XsdDatatype::Short), &target),
            Satisfiability::Inhabited
        );
        assert_eq!(
            containment(&DataRange::Datatype(XsdDatatype::UnsignedInt), &target),
            Satisfiability::Inhabited
        );

        // nonNegativeInteger ⊓ nonPositiveInteger = {0} ⊆ short.
        let zero = DataRange::And(vec![
            DataRange::Datatype(XsdDatatype::NonNegativeInteger),
            DataRange::Datatype(XsdDatatype::NonPositiveInteger),
        ]);
        assert_eq!(
            containment(&zero, &DataRange::Datatype(XsdDatatype::Short)),
            Satisfiability::Empty
        );
    }

    /// A range contains itself, and the empty range is contained in everything.
    #[test]
    fn containment_is_reflexive_and_the_empty_range_is_least() {
        let short = DataRange::Datatype(XsdDatatype::Short);
        assert_eq!(containment(&short, &short), Satisfiability::Empty);
        let empty = DataRange::Or(Vec::new());
        assert_eq!(containment(&empty, &short), Satisfiability::Empty);
        // …and nothing but the whole domain contains `rdfs:Literal`.
        assert_eq!(
            containment(&DataRange::Any, &short),
            Satisfiability::Inhabited
        );
        assert_eq!(
            containment(&DataRange::Any, &DataRange::Any),
            Satisfiability::Empty
        );
    }

    /// DISJOINT value spaces are not contained in one another, in either direction.
    #[test]
    fn disjoint_value_spaces_contain_nothing_of_each_other() {
        let float = DataRange::Datatype(XsdDatatype::Float);
        let decimal = DataRange::Datatype(XsdDatatype::Decimal);
        assert_eq!(containment(&float, &decimal), Satisfiability::Inhabited);
        assert_eq!(containment(&decimal, &float), Satisfiability::Inhabited);
    }

    /// An OPAQUE operand decides nothing, on either side — and `is_exactly_decided` of the
    /// counterexample range is the predicate that says so, which is what a consumer gates a
    /// negative answer on.
    #[test]
    fn an_opaque_operand_is_undecided_and_says_so() {
        let short = DataRange::Datatype(XsdDatatype::Short);
        for (sub, sup) in [(&DataRange::Opaque, &short), (&short, &DataRange::Opaque)] {
            assert_eq!(containment(sub, sup), Satisfiability::Undecided);
            assert!(!is_exactly_decided(&counterexample(sub, sup)));
        }
        assert!(is_exactly_decided(&counterexample(&short, &short)));
    }

    /// An `xsd:pattern` facet is modelled as [`DataRange::Opaque`], so a range carrying one
    /// is UNDECIDED against anything rather than reported as uncontained.
    #[test]
    fn a_pattern_facet_leaves_containment_undecided() {
        let patterned = DataRange::Opaque;
        let strings = DataRange::Datatype(XsdDatatype::String);
        assert_eq!(containment(&patterned, &strings), Satisfiability::Undecided);
    }
}

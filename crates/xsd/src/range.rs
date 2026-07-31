// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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
//! | one space each | `dateTime`, `date`, `time`, the `duration` family, and the five Gregorian types | listed sets and their complements |
//!
//! The `duration` family is ONE space because the `xsd:dayTimeDuration` and
//! `xsd:yearMonthDuration` value spaces overlap at the zero duration.
//!
//! Beyond those spaces lies the **remainder**: the values of datatypes this module does
//! not model (`owl:real`, `owl:rational`, `xsd:anyURI`, `rdf:langString`, a
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

use std::cmp::Ordering;

use crate::datatype::XsdDatatype;
use crate::numeric::Decimal;
use crate::ops::{value_cmp, value_eq};
use crate::temporal::Time;
use crate::value::XsdValue;

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
    if space_of_value(a) != space_of_value(b) {
        return false;
    }
    match (a, b) {
        (XsdValue::Float(x), XsdValue::Float(y)) => same_float(f64::from(*x), f64::from(*y)),
        (XsdValue::Double(x), XsdValue::Double(y)) => same_float(*x, *y),
        (XsdValue::Time(x), XsdValue::Time(y)) => same_time(x, y),
        _ => value_eq(a, b),
    }
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
    /// `xsd:dateTime`.
    DateTime,
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
const TEMPORAL_SPACES: [TemporalSpace; 9] = [
    TemporalSpace::DateTime,
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
        D::DateTime => Space::Temporal(TemporalSpace::DateTime),
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
    space_of_datatype(value.datatype())
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
#[derive(Clone, Copy)]
enum Point {
    /// A decimal endpoint, ordered by [`Decimal::cmp_exact`] — total, and never `NaN`.
    Dec(Decimal),
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
    /// A float endpoint with the two zeros collapsed onto one point.
    fn float(x: f64) -> Self {
        Self::Float(if x == 0.0 { 0.0 } else { x })
    }

    /// The decimal this endpoint carries, or `None` for another stratum's endpoint.
    fn as_dec(self) -> Option<Decimal> {
        match self {
            Self::Dec(d) => Some(d),
            Self::Float(_) | Self::Len(_) => None,
        }
    }

    /// The float this endpoint carries, or `None` for another stratum's endpoint.
    fn as_float(self) -> Option<f64> {
        match self {
            Self::Float(x) => Some(x),
            Self::Dec(_) | Self::Len(_) => None,
        }
    }

    /// The length this endpoint carries, or `None` for another stratum's endpoint.
    fn as_len(self) -> Option<u64> {
        match self {
            Self::Len(l) => Some(l),
            Self::Dec(_) | Self::Float(_) => None,
        }
    }

    /// Which stratum the endpoint came from. This orders the strata apart so that [`Ord`]
    /// stays total; it never orders two endpoints that one set actually holds together.
    fn stratum(self) -> u8 {
        match self {
            Self::Dec(_) => 0,
            Self::Float(_) => 1,
            Self::Len(_) => 2,
        }
    }
}

impl Ord for Point {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Dec(a), Self::Dec(b)) => a.cmp_exact(b),
            // `NaN` is excluded by construction, so `partial_cmp` is total here.
            (Self::Float(a), Self::Float(b)) => a.partial_cmp(b).unwrap_or(Ordering::Equal),
            (Self::Len(a), Self::Len(b)) => a.cmp(b),
            _ => self.stratum().cmp(&other.stratum()),
        }
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

/// The lower end of an interval.
#[derive(Clone, Copy)]
enum Lo {
    /// No lower bound.
    Unbounded,
    /// The endpoint is included.
    Incl(Point),
    /// The endpoint is excluded.
    Excl(Point),
}

/// The upper end of an interval.
#[derive(Clone, Copy)]
enum Hi {
    /// No upper bound.
    Unbounded,
    /// The endpoint is included.
    Incl(Point),
    /// The endpoint is excluded.
    Excl(Point),
}

impl Lo {
    /// Where this lower end sits relative to another: `Unbounded` is least, and at one
    /// endpoint the inclusive end is the lower of the two.
    fn cmp_lo(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Unbounded, Self::Unbounded) => Ordering::Equal,
            (Self::Unbounded, _) => Ordering::Less,
            (_, Self::Unbounded) => Ordering::Greater,
            (Self::Incl(a) | Self::Excl(a), Self::Incl(b) | Self::Excl(b)) => {
                a.cmp(b).then_with(|| self.rank().cmp(&other.rank()))
            }
        }
    }

    /// Sort key at one endpoint: inclusive is the lower end.
    fn rank(&self) -> u8 {
        match self {
            Self::Unbounded => 0,
            Self::Incl(_) => 1,
            Self::Excl(_) => 2,
        }
    }

    /// Whether `v` clears this lower end.
    fn admits(&self, v: &Point) -> bool {
        match self {
            Self::Unbounded => true,
            Self::Incl(a) => v >= a,
            Self::Excl(a) => v > a,
        }
    }
}

impl Hi {
    /// Where this upper end sits relative to another: `Unbounded` is greatest, and at
    /// one endpoint the exclusive end is the lower of the two.
    fn cmp_hi(&self, other: &Self) -> Ordering {
        match (self, other) {
            (Self::Unbounded, Self::Unbounded) => Ordering::Equal,
            (Self::Unbounded, _) => Ordering::Greater,
            (_, Self::Unbounded) => Ordering::Less,
            (Self::Incl(a) | Self::Excl(a), Self::Incl(b) | Self::Excl(b)) => {
                a.cmp(b).then_with(|| self.rank().cmp(&other.rank()))
            }
        }
    }

    /// Sort key at one endpoint: exclusive is the lower end.
    fn rank(&self) -> u8 {
        match self {
            Self::Excl(_) => 0,
            Self::Incl(_) => 1,
            Self::Unbounded => 2,
        }
    }

    /// Whether `v` clears this upper end.
    fn admits(&self, v: &Point) -> bool {
        match self {
            Self::Unbounded => true,
            Self::Incl(a) => v <= a,
            Self::Excl(a) => v < a,
        }
    }
}

/// One interval over the endpoint domain.
#[derive(Clone, Copy)]
struct Interval {
    /// The lower end.
    lo: Lo,
    /// The upper end.
    hi: Hi,
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
        Self {
            lo: Lo::Incl(v),
            hi: Hi::Incl(v),
        }
    }

    /// Whether the interval spans any of the ORDER — a weaker property than holding a
    /// member of a stratum's carrier, which each stratum decides for itself.
    fn spans_order(&self) -> bool {
        match (&self.lo, &self.hi) {
            (Lo::Unbounded, _) | (_, Hi::Unbounded) => true,
            (Lo::Incl(a), Hi::Incl(b)) => a <= b,
            (Lo::Incl(a) | Lo::Excl(a), Hi::Excl(b)) | (Lo::Excl(a), Hi::Incl(b)) => a < b,
        }
    }

    /// The single point this interval pins down, if it is a closed degenerate one.
    fn degenerate_point(&self) -> Option<Point> {
        match (self.lo, self.hi) {
            (Lo::Incl(a), Hi::Incl(b)) if a == b => Some(a),
            _ => None,
        }
    }

    /// Whether `v` lies in the interval.
    fn holds(&self, v: &Point) -> bool {
        self.lo.admits(v) && self.hi.admits(v)
    }
}

/// A canonical, sorted, pairwise order-disjoint set of intervals.
#[derive(Clone)]
struct IntervalSet {
    /// The intervals, sorted by lower end and merged where they touch.
    intervals: Vec<Interval>,
}

impl IntervalSet {
    /// The empty set.
    fn empty() -> Self {
        Self {
            intervals: Vec::new(),
        }
    }

    /// The whole domain.
    fn full() -> Self {
        Self {
            intervals: vec![Interval::full()],
        }
    }

    /// A single point.
    fn point(v: Point) -> Self {
        Self {
            intervals: vec![Interval::point(v)],
        }
    }

    /// Canonicalize: drop order-empty intervals, sort, and merge every pair that leaves
    /// no gap between them.
    ///
    /// The sort is unstable, which stays deterministic here: two intervals that compare
    /// equal agree on both ends, and every reading of an endpoint afterwards is numeric
    /// (an order comparison, an integer rounding, a carrier test), so order-equal
    /// endpoints answer alike and the tie order cannot be observed.
    fn canonical(mut intervals: Vec<Interval>) -> Self {
        intervals.retain(Interval::spans_order);
        intervals.sort_unstable_by(|a, b| a.lo.cmp_lo(&b.lo).then_with(|| a.hi.cmp_hi(&b.hi)));
        let mut merged: Vec<Interval> = Vec::with_capacity(intervals.len());
        for iv in intervals {
            let touching = merged.last().is_some_and(|last| !gap_between(last, &iv));
            if touching {
                if let Some(last) = merged.last_mut()
                    && last.hi.cmp_hi(&iv.hi) == Ordering::Less
                {
                    last.hi = iv.hi;
                }
            } else {
                merged.push(iv);
            }
        }
        Self { intervals: merged }
    }

    /// Whether `v` lies in the set.
    fn holds(&self, v: &Point) -> bool {
        self.intervals.iter().any(|iv| iv.holds(v))
    }

    /// The gaps — exactly the set-theoretic complement over the order.
    fn complement(&self) -> Self {
        let mut out: Vec<Interval> = Vec::with_capacity(self.intervals.len() + 1);
        let mut cursor = Lo::Unbounded;
        let mut open = true;
        for iv in &self.intervals {
            match iv.lo {
                Lo::Unbounded => {}
                Lo::Incl(v) => out.push(Interval {
                    lo: cursor,
                    hi: Hi::Excl(v),
                }),
                Lo::Excl(v) => out.push(Interval {
                    lo: cursor,
                    hi: Hi::Incl(v),
                }),
            }
            cursor = match iv.hi {
                Hi::Unbounded => {
                    open = false;
                    break;
                }
                Hi::Incl(v) => Lo::Excl(v),
                Hi::Excl(v) => Lo::Incl(v),
            };
        }
        if open {
            out.push(Interval {
                lo: cursor,
                hi: Hi::Unbounded,
            });
        }
        Self::canonical(out)
    }

    /// Pairwise intersection.
    fn intersect(&self, other: &Self) -> Self {
        let mut out = Vec::new();
        for a in &self.intervals {
            for b in &other.intervals {
                let lo = if a.lo.cmp_lo(&b.lo) == Ordering::Greater {
                    a.lo
                } else {
                    b.lo
                };
                let hi = if a.hi.cmp_hi(&b.hi) == Ordering::Less {
                    a.hi
                } else {
                    b.hi
                };
                let iv = Interval { lo, hi };
                if iv.spans_order() {
                    out.push(iv);
                }
            }
        }
        Self::canonical(out)
    }

    /// Merge of the two sets.
    fn union(&self, other: &Self) -> Self {
        let mut out = self.intervals.clone();
        out.extend_from_slice(&other.intervals);
        Self::canonical(out)
    }

    /// Up to `limit` least lengths from each interval.
    ///
    /// Only a length-selected space samples its endpoints this way; the ordered strata
    /// read theirs through their own [`Algebra`] impl.
    fn sample(&self, limit: usize) -> Vec<u64> {
        let mut out = Vec::new();
        for iv in &self.intervals {
            let mut cursor = match iv.lo {
                Lo::Unbounded => 0,
                Lo::Incl(p) => p.as_len().unwrap_or(0),
                Lo::Excl(p) => match p.as_len().unwrap_or(0).checked_add(1) {
                    Some(next) => next,
                    None => continue,
                },
            };
            for _ in 0..limit {
                if !iv.holds(&Point::Len(cursor)) {
                    break;
                }
                out.push(cursor);
                match cursor.checked_add(1) {
                    Some(next) => cursor = next,
                    None => break,
                }
            }
        }
        out
    }
}

/// Whether an order gap separates `first` (the lower interval) from `second`.
fn gap_between(first: &Interval, second: &Interval) -> bool {
    let gap_lo = match first.hi {
        Hi::Unbounded => return false,
        Hi::Incl(v) => Lo::Excl(v),
        Hi::Excl(v) => Lo::Incl(v),
    };
    let gap_hi = match second.lo {
        Lo::Unbounded => return false,
        Lo::Incl(v) => Hi::Excl(v),
        Lo::Excl(v) => Hi::Incl(v),
    };
    Interval {
        lo: gap_lo,
        hi: gap_hi,
    }
    .spans_order()
}

// ── Decimal endpoint arithmetic ──────────────────────────────────────────────────

/// Whether a decimal is one of the integers.
fn is_integral(d: &Decimal) -> bool {
    d.frac_part().is_zero()
}

/// The least integer greater than or equal to `d`, or `None` past `i128::MAX`.
fn dec_ceil(d: &Decimal) -> Option<i128> {
    let whole = d.whole_part();
    if is_integral(d) {
        Some(whole)
    } else if d.mantissa() > 0 {
        whole.checked_add(1)
    } else {
        Some(whole)
    }
}

/// The greatest integer less than or equal to `d`, or `None` past `i128::MIN`.
fn dec_floor(d: &Decimal) -> Option<i128> {
    let whole = d.whole_part();
    if is_integral(d) {
        Some(whole)
    } else if d.mantissa() < 0 {
        whole.checked_sub(1)
    } else {
        Some(whole)
    }
}

/// The decimal-space endpoint a value denotes, or `None` when it is from another space.
fn decimal_point(value: &XsdValue) -> Option<Decimal> {
    match value {
        XsdValue::Integer { value, .. } => Some(Decimal::from_parts(*value, 0)),
        XsdValue::Decimal(d) => Some(*d),
        _ => None,
    }
}

// ── The per-space closed algebras ────────────────────────────────────────────────

/// A set exactly represented in one value space's own closed algebra.
trait Algebra: Clone {
    /// The space's complement of this set.
    fn complement(&self) -> Self;
    /// Intersection.
    fn intersect(&self, other: &Self) -> Self;
    /// Union.
    fn union(&self, other: &Self) -> Self;
    /// Whether the set holds no value. Exact — that is what makes the shape closed.
    fn is_empty(&self) -> bool;
    /// How many values the set holds.
    fn count(&self) -> Cardinality;
    /// Whether the set holds `value`, which the caller has routed to this space.
    fn holds(&self, value: &XsdValue) -> bool;
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
#[derive(Clone, Copy)]
struct IntWindow {
    /// The least integer, or `None` when unbounded below.
    lo: Option<i128>,
    /// The greatest integer, or `None` when unbounded above.
    hi: Option<i128>,
}

impl DecimalSet {
    /// The whole space.
    fn full() -> Self {
        Self {
            integral: IntervalSet::full(),
            fractional: IntervalSet::full(),
        }
    }

    /// The empty set.
    fn empty() -> Self {
        Self {
            integral: IntervalSet::empty(),
            fractional: IntervalSet::empty(),
        }
    }

    /// A single value.
    fn point(d: Decimal) -> Self {
        let set = IntervalSet::point(Point::Dec(d));
        if is_integral(&d) {
            Self {
                integral: set,
                fractional: IntervalSet::empty(),
            }
        } else {
            Self {
                integral: IntervalSet::empty(),
                fractional: set,
            }
        }
    }

    /// The value space of a decimal-space datatype.
    ///
    /// The `i128` extremes in [`XsdDatatype::integer_range`] mark the datatypes XSD
    /// leaves unbounded (`xsd:integer`, `xsd:nonNegativeInteger`, …), so they become
    /// unbounded ends rather than endpoints — the complement of `xsd:integer` inside the
    /// integers is then exactly empty.
    fn for_datatype(dt: XsdDatatype) -> Self {
        match dt.integer_range() {
            None => Self::full(),
            Some((lo, hi)) => {
                let lo = if lo == i128::MIN {
                    Lo::Unbounded
                } else {
                    Lo::Incl(Point::Dec(Decimal::from_parts(lo, 0)))
                };
                let hi = if hi == i128::MAX {
                    Hi::Unbounded
                } else {
                    Hi::Incl(Point::Dec(Decimal::from_parts(hi, 0)))
                };
                Self {
                    integral: IntervalSet::canonical(vec![Interval { lo, hi }]),
                    fractional: IntervalSet::empty(),
                }
            }
        }
    }

    /// The half-line a bound facet admits, in both strata.
    fn from_interval(iv: Interval) -> Self {
        Self {
            integral: IntervalSet::canonical(vec![iv]),
            fractional: IntervalSet::canonical(vec![iv]),
        }
    }

    /// The integer window of one interval, or `None` when it holds no integer.
    fn window(iv: &Interval) -> Option<IntWindow> {
        // An endpoint from another stratum cannot occur; reading it as unbounded keeps
        // the impossible case on the widening side.
        let lo = match iv.lo {
            Lo::Unbounded => None,
            Lo::Incl(p) => match p.as_dec() {
                Some(d) => Some(dec_ceil(&d)?),
                None => None,
            },
            Lo::Excl(p) => match p.as_dec() {
                Some(d) if is_integral(&d) => Some(d.whole_part().checked_add(1)?),
                Some(d) => Some(dec_ceil(&d)?),
                None => None,
            },
        };
        let hi = match iv.hi {
            Hi::Unbounded => None,
            Hi::Incl(p) => match p.as_dec() {
                Some(d) => Some(dec_floor(&d)?),
                None => None,
            },
            Hi::Excl(p) => match p.as_dec() {
                Some(d) if is_integral(&d) => Some(d.whole_part().checked_sub(1)?),
                Some(d) => Some(dec_floor(&d)?),
                None => None,
            },
        };
        if let (Some(a), Some(b)) = (lo, hi)
            && a > b
        {
            return None;
        }
        Some(IntWindow { lo, hi })
    }

    /// Whether the integral stratum holds an integer.
    fn integral_inhabited(&self) -> bool {
        self.integral
            .intervals
            .iter()
            .any(|iv| Self::window(iv).is_some())
    }

    /// Whether the fractional stratum holds a non-integral decimal.
    ///
    /// The stratum is dense with holes at the integers: an interval spanning more than
    /// one point holds infinitely many decimals and only finitely many integers, so it
    /// always holds a non-integral one; a degenerate interval holds its point alone.
    fn fractional_inhabited(&self) -> bool {
        self.fractional.intervals.iter().any(|iv| {
            match iv.degenerate_point().and_then(Point::as_dec) {
                Some(d) => !is_integral(&d),
                // Non-degenerate (or, impossibly, another stratum's endpoint).
                None => true,
            }
        })
    }
}

impl Algebra for DecimalSet {
    fn complement(&self) -> Self {
        Self {
            integral: self.integral.complement(),
            fractional: self.fractional.complement(),
        }
    }

    fn intersect(&self, other: &Self) -> Self {
        Self {
            integral: self.integral.intersect(&other.integral),
            fractional: self.fractional.intersect(&other.fractional),
        }
    }

    fn union(&self, other: &Self) -> Self {
        Self {
            integral: self.integral.union(&other.integral),
            fractional: self.fractional.union(&other.fractional),
        }
    }

    fn is_empty(&self) -> bool {
        !self.integral_inhabited() && !self.fractional_inhabited()
    }

    fn count(&self) -> Cardinality {
        let mut total = Cardinality::Exactly(0);
        for iv in &self.integral.intervals {
            let Some(window) = Self::window(iv) else {
                continue;
            };
            total = total.plus(match (window.lo, window.hi) {
                (Some(a), Some(b)) => {
                    // `b >= a` holds by construction, so the span fits `u128`.
                    let span = b.wrapping_sub(a) as u128 + 1;
                    u64::try_from(span).map_or(Cardinality::AtLeast(u64::MAX), Cardinality::Exactly)
                }
                _ => Cardinality::Unbounded,
            });
        }
        for iv in &self.fractional.intervals {
            total = total.plus(match iv.degenerate_point().and_then(Point::as_dec) {
                Some(d) if is_integral(&d) => Cardinality::Exactly(0),
                Some(_) => Cardinality::Exactly(1),
                // Dense and non-degenerate: infinitely many non-integral decimals.
                None => Cardinality::Unbounded,
            });
        }
        total
    }

    fn holds(&self, value: &XsdValue) -> bool {
        let Some(d) = decimal_point(value) else {
            return false;
        };
        let point = Point::Dec(d);
        if is_integral(&d) {
            self.integral.holds(&point)
        } else {
            self.fractional.holds(&point)
        }
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

impl FloatSet {
    /// The whole space.
    fn full(width: FloatWidth) -> Self {
        Self {
            width,
            number: IntervalSet::full(),
            nan: true,
        }
    }

    /// The empty set.
    fn empty(width: FloatWidth) -> Self {
        Self {
            width,
            number: IntervalSet::empty(),
            nan: false,
        }
    }

    /// The numbers an interval admits, with NaN excluded — a bound facet cannot admit
    /// NaN, which is not `>=` anything.
    fn from_interval(width: FloatWidth, iv: Interval) -> Self {
        Self {
            width,
            number: IntervalSet::canonical(vec![iv]),
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

    /// Whether an interval admits a number, witnessed by the least one it admits.
    fn admits_number(&self, iv: &Interval) -> bool {
        // An endpoint from another stratum cannot occur; reading it as admitting keeps
        // the impossible case on the widening side.
        let candidate = match iv.lo {
            Lo::Unbounded => f64::NEG_INFINITY,
            Lo::Incl(p) => match p.as_float() {
                Some(x) => x,
                None => return true,
            },
            Lo::Excl(p) => {
                let Some(x) = p.as_float() else {
                    return true;
                };
                let next = self.next_up(x);
                if next <= x {
                    // `x` is already the greatest value: nothing lies above it.
                    return false;
                }
                next
            }
        };
        iv.hi.admits(&Point::float(candidate))
    }
}

impl Algebra for FloatSet {
    fn complement(&self) -> Self {
        Self {
            width: self.width,
            number: self.number.complement(),
            nan: !self.nan,
        }
    }

    fn intersect(&self, other: &Self) -> Self {
        Self {
            width: self.width,
            number: self.number.intersect(&other.number),
            nan: self.nan && other.nan,
        }
    }

    fn union(&self, other: &Self) -> Self {
        Self {
            width: self.width,
            number: self.number.union(&other.number),
            nan: self.nan || other.nan,
        }
    }

    fn is_empty(&self) -> bool {
        !self.nan
            && !self
                .number
                .intervals
                .iter()
                .any(|iv| self.admits_number(iv))
    }

    fn count(&self) -> Cardinality {
        let mut exact = true;
        let mut total: u64 = u64::from(self.nan);
        for iv in &self.number.intervals {
            if !self.admits_number(iv) {
                continue;
            }
            match iv.degenerate_point().and_then(Point::as_float) {
                // The zero point stands for BOTH zeros of the value space.
                Some(x) => total = total.saturating_add(if x == 0.0 { 2 } else { 1 }),
                None => {
                    exact = false;
                    total = total.saturating_add(1);
                }
            }
        }
        if exact {
            Cardinality::Exactly(total)
        } else {
            Cardinality::AtLeast(total)
        }
    }

    fn holds(&self, value: &XsdValue) -> bool {
        let x = match value {
            XsdValue::Float(x) => f64::from(*x),
            XsdValue::Double(x) => *x,
            _ => return false,
        };
        if x.is_nan() {
            self.nan
        } else {
            self.number.holds(&Point::float(x))
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
    fn complement(&self) -> Self {
        Self {
            has_false: !self.has_false,
            has_true: !self.has_true,
        }
    }

    fn intersect(&self, other: &Self) -> Self {
        Self {
            has_false: self.has_false && other.has_false,
            has_true: self.has_true && other.has_true,
        }
    }

    fn union(&self, other: &Self) -> Self {
        Self {
            has_false: self.has_false || other.has_false,
            has_true: self.has_true || other.has_true,
        }
    }

    fn is_empty(&self) -> bool {
        !self.has_false && !self.has_true
    }

    fn count(&self) -> Cardinality {
        Cardinality::Exactly(u64::from(self.has_false) + u64::from(self.has_true))
    }

    fn holds(&self, value: &XsdValue) -> bool {
        match value {
            XsdValue::Boolean(true) => self.has_true,
            XsdValue::Boolean(false) => self.has_false,
            _ => false,
        }
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

impl LengthSet {
    /// The whole space.
    fn full(kind: LengthKind) -> Self {
        Self {
            kind,
            lengths: IntervalSet::full(),
            extras: Vec::new(),
            exceptions: Vec::new(),
        }
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

    /// A single value, carried as an extra over an empty length set.
    fn singleton(kind: LengthKind, value: XsdValue) -> Self {
        Self {
            kind,
            lengths: IntervalSet::empty(),
            extras: vec![value],
            exceptions: Vec::new(),
        }
    }

    /// The values whose length the interval admits.
    fn from_interval(kind: LengthKind, iv: Interval) -> Self {
        Self {
            kind,
            lengths: IntervalSet::canonical(vec![iv]),
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

    /// Whether `value` is a member.
    fn membership(&self, value: &XsdValue) -> bool {
        let Some(length) = Self::length_of(value) else {
            return false;
        };
        if self.lengths.holds(&Point::Len(length)) {
            !holds_same(&self.exceptions, value)
        } else {
            holds_same(&self.extras, value)
        }
    }

    /// The admitted lengths that could still be exhausted by the exception list, plus
    /// one admitted length beyond them if the length set reaches that far.
    fn probe_lengths(&self) -> Vec<u64> {
        let mut distinct: Vec<u64> = Vec::new();
        for length in self.exceptions.iter().filter_map(Self::length_of) {
            if !distinct.contains(&length) {
                distinct.push(length);
            }
        }
        self.lengths.sample(distinct.len() + 1)
    }

    /// The greatest admitted length, or `None` when the set admits none.
    fn max_length(&self) -> Option<u64> {
        let last = self.lengths.intervals.last()?;
        // An endpoint from another stratum cannot occur; reading it as the greatest
        // length keeps the impossible case on the widening side.
        match last.hi {
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

    /// The count of the length-selected part alone, before extras and exceptions.
    fn selected_count(&self) -> Cardinality {
        if self.lengths_unbounded() {
            // Every admitted length holds at least one value, and there are infinitely
            // many admitted lengths.
            return Cardinality::Unbounded;
        }
        let Some(max) = self.max_length() else {
            return Cardinality::Exactly(0);
        };
        match self.kind {
            LengthKind::Characters => {
                if max == 0 {
                    // Only the empty string has length 0.
                    Cardinality::Exactly(1)
                } else {
                    Cardinality::AtLeast(saturating_pow(STRING_ALPHABET_FLOOR, max))
                }
            }
            LengthKind::Octets => {
                if max >= OCTET_SATURATION_LENGTH {
                    Cardinality::AtLeast(u64::MAX)
                } else {
                    // At most eight admitted lengths remain, so the sum is exact.
                    let mut total = Cardinality::Exactly(0);
                    for length in self.lengths.sample(usize::try_from(max).unwrap_or(0) + 1) {
                        total = total
                            .plus(Cardinality::Exactly(saturating_pow(OCTET_ALPHABET, length)));
                    }
                    total
                }
            }
        }
    }
}

impl Algebra for LengthSet {
    fn complement(&self) -> Self {
        Self {
            kind: self.kind,
            lengths: self.lengths.complement(),
            extras: self.exceptions.clone(),
            exceptions: self.extras.clone(),
        }
    }

    fn intersect(&self, other: &Self) -> Self {
        let lengths = self.lengths.intersect(&other.lengths);
        let mut exceptions: Vec<XsdValue> = Vec::new();
        // A non-member of the intersection at an admitted length must be an exception of
        // one operand, because each operand admits that length.
        for value in self.exceptions.iter().chain(&other.exceptions) {
            if length_admitted(&lengths, value) && !holds_same(&exceptions, value) {
                exceptions.push(value.clone());
            }
        }
        let mut extras: Vec<XsdValue> = Vec::new();
        // A member at a length the intersection does not admit is an extra of the
        // operand that does not admit it, so the candidates are the two extra lists.
        for value in self.extras.iter().chain(&other.extras) {
            if !length_admitted(&lengths, value)
                && self.membership(value)
                && other.membership(value)
                && !holds_same(&extras, value)
            {
                extras.push(value.clone());
            }
        }
        Self {
            kind: self.kind,
            lengths,
            extras,
            exceptions,
        }
    }

    fn union(&self, other: &Self) -> Self {
        let lengths = self.lengths.union(&other.lengths);
        let mut extras: Vec<XsdValue> = Vec::new();
        for value in self.extras.iter().chain(&other.extras) {
            if !length_admitted(&lengths, value) && !holds_same(&extras, value) {
                extras.push(value.clone());
            }
        }
        let mut exceptions: Vec<XsdValue> = Vec::new();
        for value in self.exceptions.iter().chain(&other.exceptions) {
            if length_admitted(&lengths, value)
                && !self.membership(value)
                && !other.membership(value)
                && !holds_same(&exceptions, value)
            {
                exceptions.push(value.clone());
            }
        }
        Self {
            kind: self.kind,
            lengths,
            extras,
            exceptions,
        }
    }

    fn is_empty(&self) -> bool {
        if !self.extras.is_empty() {
            return false;
        }
        !self.probe_lengths().into_iter().any(|length| {
            let excluded = self
                .exceptions
                .iter()
                .filter(|v| Self::length_of(v) == Some(length))
                .count();
            self.holds_more_than(length, excluded)
        })
    }

    fn count(&self) -> Cardinality {
        self.selected_count()
            .minus(u64::try_from(self.exceptions.len()).unwrap_or(u64::MAX))
            .plus(exactly(self.extras.len()))
    }

    fn holds(&self, value: &XsdValue) -> bool {
        self.membership(value)
    }
}

/// Whether a value's length is admitted by a length set.
fn length_admitted(lengths: &IntervalSet, value: &XsdValue) -> bool {
    LengthSet::length_of(value).is_some_and(|l| lengths.holds(&Point::Len(l)))
}

/// Whether a list already holds a value identical to `value`.
fn holds_same(values: &[XsdValue], value: &XsdValue) -> bool {
    values.iter().any(|v| same_value(v, value))
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
struct ListedSet {
    /// Which temporal space this set lives in.
    space: TemporalSpace,
    /// Whether `values` lists the NON-members.
    negated: bool,
    /// The listed values, pairwise distinct under [`same_value`].
    values: Vec<XsdValue>,
}

impl ListedSet {
    /// The whole space.
    fn full(space: TemporalSpace) -> Self {
        Self {
            space,
            negated: true,
            values: Vec::new(),
        }
    }

    /// The empty set.
    fn empty(space: TemporalSpace) -> Self {
        Self {
            space,
            negated: false,
            values: Vec::new(),
        }
    }

    /// The set listing exactly `values`.
    fn listed(space: TemporalSpace, values: Vec<XsdValue>) -> Self {
        let mut deduped: Vec<XsdValue> = Vec::with_capacity(values.len());
        for value in values {
            if !holds_same(&deduped, &value) {
                deduped.push(value);
            }
        }
        Self {
            space,
            negated: false,
            values: deduped,
        }
    }

    /// A set over the same space with the given polarity and listed values.
    fn with(&self, negated: bool, values: Vec<XsdValue>) -> Self {
        Self {
            space: self.space,
            negated,
            values,
        }
    }
}

/// The union of two value lists, deduplicated under [`same_value`].
fn union_values(a: &[XsdValue], b: &[XsdValue]) -> Vec<XsdValue> {
    let mut out: Vec<XsdValue> = a.to_vec();
    for value in b {
        if !holds_same(&out, value) {
            out.push(value.clone());
        }
    }
    out
}

/// The values of `a` that also appear in `b`.
fn common_values(a: &[XsdValue], b: &[XsdValue]) -> Vec<XsdValue> {
    a.iter()
        .filter(|value| holds_same(b, value))
        .cloned()
        .collect()
}

/// The values of `a` that do not appear in `b`.
fn other_values(a: &[XsdValue], b: &[XsdValue]) -> Vec<XsdValue> {
    a.iter()
        .filter(|value| !holds_same(b, value))
        .cloned()
        .collect()
}

impl Algebra for ListedSet {
    fn complement(&self) -> Self {
        self.with(!self.negated, self.values.clone())
    }

    fn intersect(&self, other: &Self) -> Self {
        match (self.negated, other.negated) {
            (false, false) => self.with(false, common_values(&self.values, &other.values)),
            (false, true) => self.with(false, other_values(&self.values, &other.values)),
            (true, false) => self.with(false, other_values(&other.values, &self.values)),
            (true, true) => self.with(true, union_values(&self.values, &other.values)),
        }
    }

    fn union(&self, other: &Self) -> Self {
        match (self.negated, other.negated) {
            (false, false) => self.with(false, union_values(&self.values, &other.values)),
            (false, true) => self.with(true, other_values(&other.values, &self.values)),
            (true, false) => self.with(true, other_values(&self.values, &other.values)),
            (true, true) => self.with(true, common_values(&self.values, &other.values)),
        }
    }

    fn is_empty(&self) -> bool {
        if self.negated {
            // Only a finite space can be exhausted by a finite list of non-members.
            self.space
                .size()
                .is_some_and(|size| size <= u64::try_from(self.values.len()).unwrap_or(u64::MAX))
        } else {
            self.values.is_empty()
        }
    }

    fn count(&self) -> Cardinality {
        if self.negated {
            match self.space.size() {
                None => Cardinality::Unbounded,
                Some(size) => Cardinality::Exactly(
                    size.saturating_sub(u64::try_from(self.values.len()).unwrap_or(u64::MAX)),
                ),
            }
        } else {
            exactly(self.values.len())
        }
    }

    fn holds(&self, value: &XsdValue) -> bool {
        holds_same(&self.values, value) != self.negated
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

impl<A: Algebra> SpaceSet<A> {
    /// Whether a member is proved to exist.
    fn is_inhabited(&self) -> bool {
        match self {
            Self::Exact(set) => !set.is_empty(),
            Self::Inhabited => true,
            Self::Unknown => false,
        }
    }

    /// Whether the set is exactly represented.
    fn is_exact(&self) -> bool {
        matches!(self, Self::Exact(_))
    }

    /// Complement. Only an exact set has an exact complement — an exhibited member says
    /// nothing about what the complement holds.
    fn complement(&self) -> Self {
        match self {
            Self::Exact(set) => Self::Exact(set.complement()),
            Self::Inhabited | Self::Unknown => Self::Unknown,
        }
    }

    /// Intersection. An empty operand carries the answer whatever the other side is.
    fn intersect(&self, other: &Self) -> Self {
        match (self, other) {
            (Self::Exact(a), Self::Exact(b)) => Self::Exact(a.intersect(b)),
            (Self::Exact(a), _) if a.is_empty() => Self::Exact(a.clone()),
            (_, Self::Exact(b)) if b.is_empty() => Self::Exact(b.clone()),
            _ => Self::Unknown,
        }
    }

    /// Union. A witness on either side survives an unknown operand.
    fn union(&self, other: &Self) -> Self {
        if let (Self::Exact(a), Self::Exact(b)) = (self, other) {
            return Self::Exact(a.union(b));
        }
        if self.is_inhabited() || other.is_inhabited() {
            return Self::Inhabited;
        }
        Self::Unknown
    }

    /// Whether this space's part of the range is empty.
    fn satisfiability(&self) -> Satisfiability {
        match self {
            Self::Exact(set) if set.is_empty() => Satisfiability::Empty,
            Self::Exact(_) | Self::Inhabited => Satisfiability::Inhabited,
            Self::Unknown => Satisfiability::Undecided,
        }
    }

    /// How many values this space contributes.
    fn count(&self) -> Cardinality {
        match self {
            Self::Exact(set) => set.count(),
            Self::Inhabited => Cardinality::AtLeast(1),
            Self::Unknown => Cardinality::Undecided,
        }
    }

    /// Whether this space's part of the range holds `value`.
    fn holds(&self, value: &XsdValue) -> Known {
        match self {
            Self::Exact(set) => {
                if set.holds(value) {
                    Known::Yes
                } else {
                    Known::No
                }
            }
            Self::Inhabited | Self::Unknown => Known::Unknown,
        }
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
    temporal: [SpaceSet<ListedSet>; 9],
    /// Whether the range holds the values of the datatypes this module does not model.
    remainder: Known,
}

/// Apply a binary set operation across two equal-length arrays of space sets.
fn zip_spaces<A: Algebra, const N: usize>(
    a: &[SpaceSet<A>; N],
    b: &[SpaceSet<A>; N],
    op: fn(&SpaceSet<A>, &SpaceSet<A>) -> SpaceSet<A>,
) -> [SpaceSet<A>; N] {
    std::array::from_fn(|i| op(&a[i], &b[i]))
}

impl Extent {
    /// The empty range.
    fn empty() -> Self {
        Self {
            decimal: SpaceSet::Exact(DecimalSet::empty()),
            single: SpaceSet::Exact(FloatSet::empty(FloatWidth::Single)),
            double: SpaceSet::Exact(FloatSet::empty(FloatWidth::Double)),
            boolean: SpaceSet::Exact(BoolSet::empty()),
            text: std::array::from_fn(|i| SpaceSet::Exact(LengthSet::empty(TEXT_SPACES[i].kind()))),
            temporal: std::array::from_fn(|i| {
                SpaceSet::Exact(ListedSet::empty(TEMPORAL_SPACES[i]))
            }),
            remainder: Known::No,
        }
    }

    /// The whole data domain, remainder included.
    fn full() -> Self {
        Self {
            decimal: SpaceSet::Exact(DecimalSet::full()),
            single: SpaceSet::Exact(FloatSet::full(FloatWidth::Single)),
            double: SpaceSet::Exact(FloatSet::full(FloatWidth::Double)),
            boolean: SpaceSet::Exact(BoolSet::full()),
            text: std::array::from_fn(|i| SpaceSet::Exact(LengthSet::full(TEXT_SPACES[i].kind()))),
            temporal: std::array::from_fn(|i| SpaceSet::Exact(ListedSet::full(TEMPORAL_SPACES[i]))),
            remainder: Known::Yes,
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
            remainder: Known::Unknown,
        }
    }

    /// The complement of the range within the whole data domain.
    fn complement(&self) -> Self {
        Self {
            decimal: self.decimal.complement(),
            single: self.single.complement(),
            double: self.double.complement(),
            boolean: self.boolean.complement(),
            text: std::array::from_fn(|i| self.text[i].complement()),
            temporal: std::array::from_fn(|i| self.temporal[i].complement()),
            remainder: self.remainder.negate(),
        }
    }

    /// Intersection, space by space.
    fn intersect(&self, other: &Self) -> Self {
        Self {
            decimal: self.decimal.intersect(&other.decimal),
            single: self.single.intersect(&other.single),
            double: self.double.intersect(&other.double),
            boolean: self.boolean.intersect(&other.boolean),
            text: zip_spaces(&self.text, &other.text, SpaceSet::intersect),
            temporal: zip_spaces(&self.temporal, &other.temporal, SpaceSet::intersect),
            remainder: self.remainder.conjoin(other.remainder),
        }
    }

    /// Union, space by space.
    fn union(&self, other: &Self) -> Self {
        Self {
            decimal: self.decimal.union(&other.decimal),
            single: self.single.union(&other.single),
            double: self.double.union(&other.double),
            boolean: self.boolean.union(&other.boolean),
            text: zip_spaces(&self.text, &other.text, SpaceSet::union),
            temporal: zip_spaces(&self.temporal, &other.temporal, SpaceSet::union),
            remainder: self.remainder.disjoin(other.remainder),
        }
    }

    /// Whether the range is empty: every space empty AND the remainder excluded. One
    /// witness anywhere settles the answer regardless of what stays unknown.
    fn satisfiability(&self) -> Satisfiability {
        let verdicts = [
            self.decimal.satisfiability(),
            self.single.satisfiability(),
            self.double.satisfiability(),
            self.boolean.satisfiability(),
            match self.remainder {
                Known::Yes => Satisfiability::Inhabited,
                Known::No => Satisfiability::Empty,
                Known::Unknown => Satisfiability::Undecided,
            },
        ];
        let mut undecided = false;
        for verdict in verdicts
            .into_iter()
            .chain(self.text.iter().map(SpaceSet::satisfiability))
            .chain(self.temporal.iter().map(SpaceSet::satisfiability))
        {
            match verdict {
                Satisfiability::Inhabited => return Satisfiability::Inhabited,
                Satisfiability::Undecided => undecided = true,
                Satisfiability::Empty => {}
            }
        }
        if undecided {
            Satisfiability::Undecided
        } else {
            Satisfiability::Empty
        }
    }

    /// The number of values, summed over the disjoint spaces.
    fn cardinality(&self) -> Cardinality {
        let mut total = self
            .decimal
            .count()
            .plus(self.single.count())
            .plus(self.double.count())
            .plus(self.boolean.count());
        for set in &self.text {
            total = total.plus(set.count());
        }
        for set in &self.temporal {
            total = total.plus(set.count());
        }
        total.plus(match self.remainder {
            // The unmodelled part of the data domain is infinite (`xsd:anyURI` alone).
            Known::Yes => Cardinality::Unbounded,
            Known::No => Cardinality::Exactly(0),
            Known::Unknown => Cardinality::Undecided,
        })
    }

    /// Whether every space is exactly represented and the remainder is determined.
    fn is_exact(&self) -> bool {
        self.decimal.is_exact()
            && self.single.is_exact()
            && self.double.is_exact()
            && self.boolean.is_exact()
            && self.text.iter().all(SpaceSet::is_exact)
            && self.temporal.iter().all(SpaceSet::is_exact)
            && self.remainder != Known::Unknown
    }

    /// Whether the range holds `value`. A value belongs to exactly one space, so only
    /// that space's set is consulted.
    fn contains(&self, value: &XsdValue) -> Known {
        match space_of_value(value) {
            Space::Decimal => self.decimal.holds(value),
            Space::Float => self.single.holds(value),
            Space::Double => self.double.holds(value),
            Space::Boolean => self.boolean.holds(value),
            Space::Text(space) => self.text[space as usize].holds(value),
            Space::Temporal(space) => self.temporal[space as usize].holds(value),
        }
    }
}

// ── From a data range to its extent ──────────────────────────────────────────────

/// The extent of a data range across the whole data domain.
fn extent(range: &DataRange) -> Extent {
    match range {
        DataRange::Any => Extent::full(),
        DataRange::Opaque => Extent::unknown(),
        DataRange::Datatype(dt) => datatype_extent(*dt),
        DataRange::Restriction { base, facets } => restriction_extent(*base, facets),
        DataRange::OneOf(values) => values
            .iter()
            .fold(Extent::empty(), |acc, v| acc.union(&value_extent(v))),
        DataRange::Not(inner) => extent(inner).complement(),
        DataRange::And(operands) => operands
            .iter()
            .fold(Extent::full(), |acc, r| acc.intersect(&extent(r))),
        DataRange::Or(operands) => operands
            .iter()
            .fold(Extent::empty(), |acc, r| acc.union(&extent(r))),
    }
}

/// The extent of a whole datatype value space.
fn datatype_extent(dt: XsdDatatype) -> Extent {
    let mut ex = Extent::empty();
    match space_of_datatype(dt) {
        Space::Decimal => ex.decimal = SpaceSet::Exact(DecimalSet::for_datatype(dt)),
        Space::Float => ex.single = SpaceSet::Exact(FloatSet::full(FloatWidth::Single)),
        Space::Double => ex.double = SpaceSet::Exact(FloatSet::full(FloatWidth::Double)),
        Space::Boolean => ex.boolean = SpaceSet::Exact(BoolSet::full()),
        Space::Text(space) => {
            ex.text[space as usize] = SpaceSet::Exact(LengthSet::full(space.kind()));
        }
        Space::Temporal(space) => {
            ex.temporal[space as usize] = if covers_whole_space(dt) {
                SpaceSet::Exact(ListedSet::full(space))
            } else {
                // A duration subtype is an infinite proper subspace of the shared
                // duration space; the zero duration witnesses it.
                SpaceSet::Inhabited
            };
        }
    }
    ex
}

/// The extent of a single value.
fn value_extent(value: &XsdValue) -> Extent {
    let mut ex = Extent::empty();
    match space_of_value(value) {
        Space::Decimal => match decimal_point(value) {
            Some(d) => ex.decimal = SpaceSet::Exact(DecimalSet::point(d)),
            None => return Extent::unknown(),
        },
        Space::Float => ex.single = float_point(FloatWidth::Single, value),
        Space::Double => ex.double = float_point(FloatWidth::Double, value),
        Space::Boolean => match value {
            XsdValue::Boolean(b) => ex.boolean = SpaceSet::Exact(BoolSet::point(*b)),
            _ => return Extent::unknown(),
        },
        Space::Text(space) => {
            ex.text[space as usize] =
                SpaceSet::Exact(LengthSet::singleton(space.kind(), value.clone()));
        }
        Space::Temporal(space) => {
            ex.temporal[space as usize] =
                SpaceSet::Exact(ListedSet::listed(space, vec![value.clone()]));
        }
    }
    ex
}

/// The set holding exactly one float value.
///
/// A named zero is exhibited rather than exactly represented: the order this space
/// carries cannot separate `positiveZero` from `negativeZero`, so a set that must hold
/// one and not the other is outside the interval algebra.
fn float_point(width: FloatWidth, value: &XsdValue) -> SpaceSet<FloatSet> {
    let x = match value {
        XsdValue::Float(x) => f64::from(*x),
        XsdValue::Double(x) => *x,
        _ => return SpaceSet::Unknown,
    };
    if x.is_nan() {
        SpaceSet::Exact(FloatSet {
            width,
            number: IntervalSet::empty(),
            nan: true,
        })
    } else if x == 0.0 {
        SpaceSet::Inhabited
    } else {
        SpaceSet::Exact(FloatSet {
            width,
            number: IntervalSet::point(Point::float(x)),
            nan: false,
        })
    }
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

/// The extent of `owl:onDatatype` + `owl:withRestrictions`.
fn restriction_extent(base: XsdDatatype, facets: &[Facet]) -> Extent {
    let mut ex = Extent::empty();
    match space_of_datatype(base) {
        Space::Decimal => ex.decimal = decimal_restriction(base, facets),
        Space::Float => ex.single = float_restriction(FloatWidth::Single, facets),
        Space::Double => ex.double = float_restriction(FloatWidth::Double, facets),
        Space::Boolean => {
            // XSD gives `xsd:boolean` no ordering or length facet, so any facet here
            // constrains nothing coherently.
            ex.boolean = if facets.is_empty() {
                SpaceSet::Exact(BoolSet::full())
            } else {
                SpaceSet::Unknown
            };
        }
        Space::Text(space) => ex.text[space as usize] = length_restriction(space.kind(), facets),
        Space::Temporal(space) => {
            ex.temporal[space as usize] = temporal_restriction(base, space, facets);
        }
    }
    ex
}

/// A restriction over the decimal space.
fn decimal_restriction(base: XsdDatatype, facets: &[Facet]) -> SpaceSet<DecimalSet> {
    let mut set = DecimalSet::for_datatype(base);
    for facet in facets {
        let Some((side, value)) = BoundSide::of(facet) else {
            // A length facet does not apply to a number.
            return SpaceSet::Unknown;
        };
        let Some(bound) = decimal_point(value) else {
            // The bound is from another value space.
            return SpaceSet::Unknown;
        };
        set = set.intersect(&DecimalSet::from_interval(side.interval(Point::Dec(bound))));
    }
    SpaceSet::Exact(set)
}

/// A restriction over a float space. A bound facet restricts the numbers and empties
/// the NaN stratum — NaN is not `>=` anything.
fn float_restriction(width: FloatWidth, facets: &[Facet]) -> SpaceSet<FloatSet> {
    let mut set = FloatSet::full(width);
    for facet in facets {
        let Some((side, value)) = BoundSide::of(facet) else {
            return SpaceSet::Unknown;
        };
        let bound = match (width, value) {
            (FloatWidth::Single, XsdValue::Float(x)) => f64::from(*x),
            (FloatWidth::Double, XsdValue::Double(x)) => *x,
            // A bound from another value space, at the other width included.
            _ => return SpaceSet::Unknown,
        };
        if bound.is_nan() {
            // Nothing compares with NaN, so a NaN bound constrains nothing coherently.
            return SpaceSet::Unknown;
        }
        set = set.intersect(&FloatSet::from_interval(
            width,
            side.interval(Point::float(bound)),
        ));
    }
    SpaceSet::Exact(set)
}

/// A restriction over a length-selected space.
fn length_restriction(kind: LengthKind, facets: &[Facet]) -> SpaceSet<LengthSet> {
    let mut set = LengthSet::full(kind);
    for facet in facets {
        let Some(interval) = length_interval(facet) else {
            // A bound facet does not apply to a string or a byte sequence.
            return SpaceSet::Unknown;
        };
        set = set.intersect(&LengthSet::from_interval(kind, interval));
    }
    SpaceSet::Exact(set)
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

/// A restriction over a temporal space.
///
/// Bound facets here do NOT yield an exactly represented set: the XSD order on these
/// spaces is partial, so an interval's complement is not a union of intervals. What
/// remains decidable without assuming order completeness is a contradiction between two
/// bounds (an exact empty set, which does close under complement) and an inclusive
/// endpoint that satisfies every bound (a witness).
fn temporal_restriction(
    base: XsdDatatype,
    space: TemporalSpace,
    facets: &[Facet],
) -> SpaceSet<ListedSet> {
    match temporal_bounds(space, facets) {
        TemporalBounds::Contradiction => SpaceSet::Exact(ListedSet::empty(space)),
        TemporalBounds::Unconstrained => {
            if covers_whole_space(base) {
                SpaceSet::Exact(ListedSet::full(space))
            } else {
                SpaceSet::Inhabited
            }
        }
        TemporalBounds::Witnessed if covers_whole_space(base) => SpaceSet::Inhabited,
        TemporalBounds::Witnessed | TemporalBounds::Indeterminate => SpaceSet::Unknown,
    }
}

/// Read a temporal space's bound facets for a contradiction or a witness.
fn temporal_bounds(space: TemporalSpace, facets: &[Facet]) -> TemporalBounds {
    let mut bounds: Vec<(BoundSide, &XsdValue)> = Vec::with_capacity(facets.len());
    for facet in facets {
        let Some((side, value)) = BoundSide::of(facet) else {
            // A length facet does not apply to a temporal value.
            return TemporalBounds::Indeterminate;
        };
        if space_of_value(value) != Space::Temporal(space) {
            return TemporalBounds::Indeterminate;
        }
        bounds.push((side, value));
    }
    if bounds.is_empty() {
        return TemporalBounds::Unconstrained;
    }
    for (lower_side, lower) in bounds.iter().filter(|(side, _)| side.is_lower()) {
        for (upper_side, upper) in bounds.iter().filter(|(side, _)| !side.is_lower()) {
            match value_cmp(lower, upper) {
                Some(Ordering::Greater) => return TemporalBounds::Contradiction,
                Some(Ordering::Equal)
                    if !(lower_side.is_inclusive() && upper_side.is_inclusive()) =>
                {
                    return TemporalBounds::Contradiction;
                }
                _ => {}
            }
        }
    }
    let witnessed = bounds
        .iter()
        .filter(|(side, _)| side.is_inclusive())
        .any(|(_, candidate)| {
            bounds
                .iter()
                .all(|(side, bound)| satisfies_bound(candidate, *side, bound) == Some(true))
        });
    if witnessed {
        TemporalBounds::Witnessed
    } else {
        TemporalBounds::Indeterminate
    }
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
    use pretty_assertions::assert_eq;

    /// Parse a lexical form that the test knows is valid.
    fn v(lexical: &str, dt: XsdDatatype) -> XsdValue {
        parse(lexical, dt).unwrap_or_else(|e| panic!("parse({lexical:?}, {dt:?}) failed: {e}"))
    }

    /// A decimal endpoint from an integer.
    fn dec(n: i128) -> Point {
        Point::Dec(Decimal::from_parts(n, 0))
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
        let half = Point::Dec(Decimal::from_parts(15, 1)); // 1.5
        let window = DecimalSet::window(&Interval {
            lo: Lo::Incl(half),
            hi: Hi::Incl(dec(4)),
        })
        .expect("1.5 ..= 4 holds integers");
        assert_eq!((window.lo, window.hi), (Some(2), Some(4)));
    }

    #[test]
    fn exclusive_integer_bounds_step_past_the_endpoint() {
        let window = DecimalSet::window(&Interval {
            lo: Lo::Excl(dec(3)),
            hi: Hi::Excl(dec(5)),
        })
        .expect("3 < n < 5 holds 4");
        assert_eq!((window.lo, window.hi), (Some(4), Some(4)));
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
}

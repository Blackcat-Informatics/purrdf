// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Strict comparison of RDF literal operands selected by rule and constraint callers.
//!
//! This law is separate from XSD partial order and RDF term identity. It neither
//! normalizes the original operands nor assigns digest meaning to an undeclared
//! datatype.

use std::cmp::Ordering;

use purrdf_lex::allocation::{Admission, Memory, StorageError};
use purrdf_xsd::datatype::XsdDatatype;
use purrdf_xsd::value::{ParsedValue, XsdValue, try_parse_with_memory};

use crate::{RdfLiteral, RdfTextDirection};

/// The unchanged components of one comparison operand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiteralOperand<'a> {
    /// The original lexical form.
    pub lexical_form: &'a str,
    /// The original datatype, if explicitly available at this boundary.
    pub datatype: Option<&'a str>,
    /// The original language tag.
    pub language: Option<&'a str>,
    /// The original RDF base direction.
    pub direction: Option<RdfTextDirection>,
}

impl<'a> From<&'a RdfLiteral> for LiteralOperand<'a> {
    fn from(value: &'a RdfLiteral) -> Self {
        Self {
            lexical_form: &value.lexical_form,
            datatype: value.datatype.as_deref(),
            language: value.language.as_deref(),
            direction: value.direction,
        }
    }
}

/// An explicitly supported digest family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DigestAlgorithm {
    /// The 32-byte BLAKE3 digest.
    Blake3,
    /// The 32-byte SHA-256 digest.
    Sha256,
}

impl DigestAlgorithm {
    /// The stable algorithm name for encoded comparison profiles.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Blake3 => "blake3",
            Self::Sha256 => "sha-256",
        }
    }
}

/// A caller's assignment of digest meaning to an RDF datatype.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DigestDatatype {
    /// The caller's datatype IRI.
    pub datatype: String,
    /// The digest algorithm named by that datatype.
    pub algorithm: DigestAlgorithm,
}

/// An invalid comparison profile, distinct from refusal of an operand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiteralComparisonProfileError {
    /// The same datatype has more than one declaration.
    DuplicateDatatype,
    /// A digest declaration tries to replace a built-in time datatype.
    TimeDatatype,
}

impl purrdf_hash::error::StructuredError for LiteralComparisonProfileError {
    purrdf_hash::error_visit! {
        DuplicateDatatype => "literal-comparison-profile-duplicate-datatype" {},
        TimeDatatype => "literal-comparison-profile-time-datatype" {},
    }
}

/// Explicit caller vocabulary for comparable digest literals.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LiteralComparisonProfile {
    digests: Vec<DigestDatatype>,
}

impl LiteralComparisonProfile {
    /// Use the built-in time family with no digest vocabulary.
    pub const TIME_ONLY: Self = Self {
        digests: Vec::new(),
    };

    /// Build a profile over unambiguous digest declarations.
    ///
    /// # Errors
    /// Rejects duplicate datatype declarations and replacements for the time family.
    pub fn new(mut digests: Vec<DigestDatatype>) -> Result<Self, LiteralComparisonProfileError> {
        if digests
            .iter()
            .any(|digest| time_datatype(&digest.datatype).is_some())
        {
            return Err(LiteralComparisonProfileError::TimeDatatype);
        }
        digests.sort_unstable_by(|left, right| left.datatype.cmp(&right.datatype));
        if digests
            .windows(2)
            .any(|pair| pair[0].datatype == pair[1].datatype)
        {
            return Err(LiteralComparisonProfileError::DuplicateDatatype);
        }
        Ok(Self { digests })
    }

    /// The exact declarations of this profile, ordered by datatype IRI.
    pub fn digests(&self) -> &[DigestDatatype] {
        &self.digests
    }
}

/// A relation over a successfully established order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LiteralComparisonOp {
    /// Strict order.
    Less,
    /// Order or equality.
    LessOrEqual,
    /// Equality of comparable values.
    Equal,
}

impl purrdf_hash::error::StructuredError for LiteralComparisonOp {
    purrdf_hash::error_visit! {
        Less => "literal-less" {},
        LessOrEqual => "literal-less-or-equal" {},
        Equal => "literal-equal" {},
    }
}

/// The position of an operand in the original invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiteralOperandSide {
    /// The left operand.
    Left,
    /// The right operand.
    Right,
}

impl purrdf_hash::error::StructuredError for LiteralOperandSide {
    purrdf_hash::error_visit! {
        Left => "left-operand" {},
        Right => "right-operand" {},
    }
}

/// The specific law that prevents ordering two otherwise valid operands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiteralIncomparability {
    /// The operands belong to different value families.
    Families,
    /// Only one time declares a timezone.
    TimezoneStatus,
    /// The two digests name different algorithms.
    DigestAlgorithms,
}

impl purrdf_hash::error::StructuredError for LiteralIncomparability {
    purrdf_hash::error_visit! {
        Families => "literal-value-families" {},
        TimezoneStatus => "literal-timezone-status" {},
        DigestAlgorithms => "literal-digest-algorithms" {},
    }
}

/// The three evidence classes for an unanswerable comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiteralComparisonRefusal {
    /// A declared datatype has malformed components or lexical data.
    Invalid(LiteralOperandSide),
    /// The operand has no declared order or exceeds the supported value range.
    Unsupported(LiteralOperandSide),
    /// Both operands are valid but have no common order.
    Incomparable(LiteralIncomparability),
}

impl purrdf_hash::error::StructuredError for LiteralComparisonRefusal {
    purrdf_hash::error_visit! {
        Invalid(side) => "literal-comparison-invalid" { "side" => Record(side) },
        Unsupported(side) => "literal-comparison-unsupported" { "side" => Record(side) },
        Incomparable(reason) => "literal-comparison-incomparable" { "reason" => Record(reason) },
    }
}

/// A settled answer or an explicit undetermined answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiteralComparisonVerdict {
    /// A common order exists and settles the relation.
    Decided(bool),
    /// The operation does not settle either polarity.
    Undetermined(LiteralComparisonRefusal),
}

impl LiteralComparisonVerdict {
    /// Negate a settled result without turning refusal into truth.
    #[must_use]
    pub const fn negated(self) -> Self {
        match self {
            Self::Decided(value) => Self::Decided(!value),
            Self::Undetermined(_) => self,
        }
    }
}

/// The answer and its original, unnormalized operands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LiteralComparison<'a> {
    /// The original left operand.
    pub left: LiteralOperand<'a>,
    /// The original right operand.
    pub right: LiteralOperand<'a>,
    /// The operation the evidence describes.
    pub operation: LiteralComparisonOp,
    /// The decided or undetermined result.
    pub verdict: LiteralComparisonVerdict,
}

enum Prepared {
    Time(XsdValue),
    Digest(DigestAlgorithm, [u8; 32]),
}

fn time_datatype(datatype: &str) -> Option<XsdDatatype> {
    match XsdDatatype::from_iri(datatype) {
        Some(kind @ (XsdDatatype::DateTime | XsdDatatype::DateTimeStamp)) => Some(kind),
        _ => None,
    }
}

fn prepare<S: Admission + ?Sized>(
    operand: LiteralOperand<'_>,
    side: LiteralOperandSide,
    profile: &LiteralComparisonProfile,
    memory: &mut Memory<'_, S>,
) -> Result<Result<Prepared, LiteralComparisonRefusal>, StorageError> {
    let Some(datatype) = operand.datatype else {
        return Ok(Err(LiteralComparisonRefusal::Unsupported(side)));
    };
    let temporal = time_datatype(datatype);
    let digest = profile
        .digests
        .binary_search_by(|entry| entry.datatype.as_str().cmp(datatype))
        .ok()
        .map(|position| &profile.digests[position]);
    if temporal.is_none() && digest.is_none() {
        return Ok(Err(LiteralComparisonRefusal::Unsupported(side)));
    }
    if operand.language.is_some() || operand.direction.is_some() {
        return Ok(Err(LiteralComparisonRefusal::Invalid(side)));
    }
    if let Some(kind) = temporal {
        // Both time types have the fixed XML whiteSpace=collapse facet. Interior
        // whitespace still violates the grammar after collapse, so trim only.
        let lexical = purrdf_iri::terminals::trim_ws(operand.lexical_form);
        return match try_parse_with_memory(lexical, kind, false, memory)? {
            ParsedValue::Value(value) => Ok(Ok(Prepared::Time(value))),
            ParsedValue::Invalid(Some(_)) => Ok(Err(LiteralComparisonRefusal::Invalid(side))),
            ParsedValue::Invalid(None) => Ok(Err(LiteralComparisonRefusal::Unsupported(side))),
        };
    }
    let digest = digest.expect("the declared datatype is a time or digest");
    let mut bytes = [0; 32];
    if purrdf_hash::hex::decode_to_slice(operand.lexical_form, &mut bytes) == Ok(bytes.len()) {
        Ok(Ok(Prepared::Digest(digest.algorithm, bytes)))
    } else {
        Ok(Err(LiteralComparisonRefusal::Invalid(side)))
    }
}

fn settle<S: Admission + ?Sized>(
    left: &Prepared,
    right: &Prepared,
    operation: LiteralComparisonOp,
    memory: &mut Memory<'_, S>,
) -> Result<LiteralComparisonVerdict, StorageError> {
    use LiteralComparisonRefusal::Incomparable;
    use LiteralComparisonVerdict::{Decided, Undetermined};
    use LiteralIncomparability as Why;
    let order = match (left, right) {
        (
            Prepared::Time(left @ XsdValue::DateTime(a)),
            Prepared::Time(right @ XsdValue::DateTime(b)),
        ) => {
            if a.timezone_minutes().is_some() != b.timezone_minutes().is_some() {
                return Ok(Undetermined(Incomparable(Why::TimezoneStatus)));
            }
            // Two valid calendar values with the same timezone status are ordered.
            crate::annotation_range::compare(left, right, memory)?
                .expect("same-status dateTimes have a complete order")
        }
        (Prepared::Digest(a, left), Prepared::Digest(b, right)) => {
            if a != b {
                return Ok(Undetermined(Incomparable(Why::DigestAlgorithms)));
            }
            left.cmp(right)
        }
        _ => return Ok(Undetermined(Incomparable(Why::Families))),
    };
    Ok(Decided(match operation {
        LiteralComparisonOp::Less => order == Ordering::Less,
        LiteralComparisonOp::LessOrEqual => order != Ordering::Greater,
        LiteralComparisonOp::Equal => order == Ordering::Equal,
    }))
}

/// Compare two literal operands without changing either original.
///
/// A lexical refusal is a verdict, not a storage failure and not a false
/// answer. Operand preparation proceeds left to right, including on refusal.
///
/// # Errors
/// Returns the original resource refusal of exact parsing or comparison.
pub fn compare_literals<'a, S: Admission + ?Sized>(
    left: LiteralOperand<'a>,
    right: LiteralOperand<'a>,
    operation: LiteralComparisonOp,
    profile: &LiteralComparisonProfile,
    memory: &mut Memory<'_, S>,
) -> Result<LiteralComparison<'a>, StorageError> {
    let baseline = memory.admitted_bytes();
    let verdict = memory.scope(|memory| {
        let left = match prepare(left, LiteralOperandSide::Left, profile, memory)? {
            Ok(value) => value,
            Err(refusal) => return Ok(LiteralComparisonVerdict::Undetermined(refusal)),
        };
        let right = match prepare(right, LiteralOperandSide::Right, profile, memory)? {
            Ok(value) => value,
            Err(refusal) => return Ok(LiteralComparisonVerdict::Undetermined(refusal)),
        };
        settle(&left, &right, operation, memory)
    })?;
    // The closure owns both parsed values. At this boundary their allocations
    // have been destroyed on every result path, including refusal of the right.
    let charged = memory
        .admitted_bytes()
        .checked_sub(baseline)
        .ok_or(StorageError::SizeOverflow)?;
    memory.release_bytes(charged)?;
    Ok(LiteralComparison {
        left,
        right,
        operation,
        verdict,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_lex::allocation::Resident;

    const TIME: &str = XsdDatatype::DateTime.iri();
    const STAMP: &str = XsdDatatype::DateTimeStamp.iri();
    const BLAKE: &str = "urn:example:blake";
    const SHA: &str = "urn:example:sha";

    fn operand<'a>(lexical_form: &'a str, datatype: &'a str) -> LiteralOperand<'a> {
        LiteralOperand {
            lexical_form,
            datatype: Some(datatype),
            language: None,
            direction: None,
        }
    }

    #[test]
    fn strict_time_order_distinguishes_false_from_undetermined_in_both_polarities() {
        use LiteralComparisonOp::{Equal, Less, LessOrEqual};
        use LiteralComparisonRefusal::{Incomparable, Invalid};
        use LiteralComparisonVerdict::{Decided, Undetermined};
        let utc = operand("2026-04-04T09:12:00Z", STAMP);
        let equivalent = operand("2026-04-04T11:12:00+02:00", TIME);
        let local = operand("2026-01-01T00:00:00", TIME);
        let later_local = operand("2026-01-02T00:00:00", TIME);
        let invalid_stamp = operand("2026-04-04T09:12:00", STAMP);
        let cases = [
            (utc, equivalent, Equal, Decided(true), Decided(false)),
            (utc, equivalent, Less, Decided(false), Decided(true)),
            (utc, equivalent, LessOrEqual, Decided(true), Decided(false)),
            (local, later_local, Less, Decided(true), Decided(false)),
            (
                local,
                utc,
                Less,
                Undetermined(Incomparable(LiteralIncomparability::TimezoneStatus)),
                Undetermined(Incomparable(LiteralIncomparability::TimezoneStatus)),
            ),
            (
                utc,
                invalid_stamp,
                Equal,
                Undetermined(Invalid(LiteralOperandSide::Right)),
                Undetermined(Invalid(LiteralOperandSide::Right)),
            ),
        ];
        for (left, right, op, positive, negative) in cases {
            let result = compare_literals(
                left,
                right,
                op,
                &LiteralComparisonProfile::TIME_ONLY,
                &mut Memory::new(&mut Resident),
            )
            .unwrap();
            assert_eq!(
                (result.left, result.right, result.operation),
                (left, right, op)
            );
            assert_eq!(result.verdict, positive);
            assert_eq!(result.verdict.negated(), negative);
        }
    }

    #[test]
    fn declared_digests_compare_bytes_without_coercing_algorithm_or_datatype() {
        use LiteralComparisonOp::{Equal, Less, LessOrEqual};
        use LiteralComparisonRefusal::{Incomparable, Invalid, Unsupported};
        use LiteralComparisonVerdict::{Decided, Undetermined};
        let declared = vec![
            DigestDatatype {
                datatype: BLAKE.into(),
                algorithm: DigestAlgorithm::Blake3,
            },
            DigestDatatype {
                datatype: SHA.into(),
                algorithm: DigestAlgorithm::Sha256,
            },
        ];
        let profile = LiteralComparisonProfile::new(declared).unwrap();
        let low = "a1".repeat(32);
        let low_upper = low.to_uppercase();
        let high = "b0".repeat(32);
        let left = operand(&low, BLAKE);
        let same = operand(&low_upper, BLAKE);
        let greater = operand(&high, BLAKE);
        let other = operand(&low, SHA);
        let malformed = operand(&low[..62], BLAKE);
        let unknown = LiteralOperand {
            language: Some("en-US"),
            ..operand(&low, "urn:example:unregistered")
        };
        let time = operand("2026-04-04T09:12:00Z", STAMP);
        for (right, op, positive, negative) in [
            (same, Equal, Decided(true), Decided(false)),
            (greater, Less, Decided(true), Decided(false)),
            (same, LessOrEqual, Decided(true), Decided(false)),
            (
                other,
                Equal,
                Undetermined(Incomparable(LiteralIncomparability::DigestAlgorithms)),
                Undetermined(Incomparable(LiteralIncomparability::DigestAlgorithms)),
            ),
            (
                malformed,
                Equal,
                Undetermined(Invalid(LiteralOperandSide::Right)),
                Undetermined(Invalid(LiteralOperandSide::Right)),
            ),
            (
                unknown,
                Equal,
                Undetermined(Unsupported(LiteralOperandSide::Right)),
                Undetermined(Unsupported(LiteralOperandSide::Right)),
            ),
            (
                time,
                Equal,
                Undetermined(Incomparable(LiteralIncomparability::Families)),
                Undetermined(Incomparable(LiteralIncomparability::Families)),
            ),
        ] {
            let result =
                compare_literals(left, right, op, &profile, &mut Memory::new(&mut Resident))
                    .unwrap();
            assert_eq!(result.right, right);
            assert_eq!(result.verdict, positive);
            assert_eq!(result.verdict.negated(), negative);
        }
    }

    #[test]
    fn operand_refusal_releases_admitted_exact_time_scratch() {
        let wide = operand(
            "2026-04-04T09:12:12.345678901234567890123456789012345678901234567890123456789Z",
            TIME,
        );
        let refused = operand("2026-04-04T09:12:00", STAMP);
        let mut resident = Resident;
        let mut memory = Memory::new(&mut resident);
        let mut prior = Vec::<u8>::new();
        memory.reserve(&mut prior, 32).unwrap();
        let baseline = memory.admitted_bytes();
        for right in [wide, refused] {
            let answer = compare_literals(
                wide,
                right,
                LiteralComparisonOp::Equal,
                &LiteralComparisonProfile::TIME_ONLY,
                &mut memory,
            )
            .unwrap();
            assert_eq!(memory.admitted_bytes(), baseline);
            if right == refused {
                assert_eq!(
                    answer.verdict,
                    LiteralComparisonVerdict::Undetermined(LiteralComparisonRefusal::Invalid(
                        LiteralOperandSide::Right
                    ))
                );
            } else {
                assert_eq!(answer.verdict, LiteralComparisonVerdict::Decided(true));
            }
        }
        memory.release_vec(prior).unwrap();
    }

    #[test]
    fn secondary_refund_refusal_cannot_replace_operation_failure() {
        #[derive(Default)]
        struct Grant {
            live: usize,
            peak: usize,
            ceiling: Option<usize>,
            refused_growth: bool,
            attempted_refund: bool,
        }
        impl Admission for Grant {
            fn resize(&mut self, next: usize) -> Result<(), StorageError> {
                if self.ceiling.is_some_and(|ceiling| next > ceiling) {
                    self.refused_growth = true;
                    return Err(StorageError::SizeOverflow);
                }
                if self.refused_growth && next < self.live {
                    self.attempted_refund = true;
                    return Err(StorageError::AdmissionFailed);
                }
                self.live = next;
                self.peak = self.peak.max(next);
                Ok(())
            }
        }
        let time = operand(
            "2026-04-04T09:12:12.345678901234567890123456789012345678901234567890123456789Z",
            TIME,
        );
        let profile = &LiteralComparisonProfile::TIME_ONLY;
        let mut baseline = Grant::default();
        let mut scratch = Memory::new(&mut baseline);
        let left = prepare(time, LiteralOperandSide::Left, profile, &mut scratch).unwrap();
        assert!(left.is_ok() && scratch.admitted_bytes() > 0);
        drop(left);
        let mut grant = Grant {
            ceiling: Some(baseline.peak),
            ..Grant::default()
        };
        let error = compare_literals(
            time,
            time,
            LiteralComparisonOp::Equal,
            profile,
            &mut Memory::new(&mut grant),
        );
        assert_eq!(error, Err(StorageError::SizeOverflow));
        assert!(grant.refused_growth && grant.attempted_refund);
    }
}

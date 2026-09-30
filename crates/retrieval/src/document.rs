// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The JSON form of a plan and of every value it carries, over
//! [`purrdf_lex::json`].
//!
//! A plan is a value a caller stores, ships, edits and hands back, and JSON is
//! the plain document it travels as. Each type here has a `to_json` and a
//! `from_json`, and the document shape is one rule applied everywhere:
//!
//! * **A struct is an object** whose members are its fields, named as the
//!   fields are, in declaration order. [`Plan::origin`] is provenance rather
//!   than content and is not written; a decoded plan is
//!   [`PlanOrigin::Deserialized`](crate::PlanOrigin::Deserialized).
//! * **An enum is externally tagged.** A unit variant is its name as a string
//!   (`"Complete"`; `{"Complete": null}` reads the same), a newtype variant is a
//!   one-member object holding its value (`{"Bounded": 5}`), and a struct
//!   variant is a one-member object holding its fields (`{"Selected": {...}}`).
//! * **An absent optional is `null`.** A reader also takes a missing member as
//!   absent, except for the fixed-point optionals ([`Fixed`] is written as its
//!   raw `i128`), which must be present and may be `null`.
//! * **A map keyed by IRI is an object in ascending IRI order**, so equal plans
//!   write equal text whatever order a hash map iterates in. A reader keeps the
//!   last of two members naming one IRI.
//! * **Numbers are exact.** Integers are written as integers and read only from
//!   integer lexemes that fit the field; an embedding component is written as
//!   the shortest `f32` lexeme that reads back to it, and a component that
//!   denotes no finite `f32` is refused.
//!
//! A reader ignores a member no field names, and refuses a member it does read
//! if the member is repeated, since which occurrence was meant is unknowable. A
//! shape error is [`PlanError::InvalidJson`] with the JSON Pointer of the
//! offending value; a well-shaped document that breaks a value's own
//! construction law is refused with that law's variant (an IRI that does not
//! parse is [`PlanError::InvalidIri`], a statistics subject named twice is
//! [`PlanError::DuplicateStatisticsSubject`]), so JSON is never a second way to
//! build a value the constructors refuse.
//!
//! The JSON form is not the plan's identity: [`Plan::canonical_bytes`] is, and a
//! plan read from JSON has the same [`PlanId`](crate::PlanId) as the plan that
//! wrote it.

use std::collections::BTreeMap;

use purrdf_core::FastMap;
use purrdf_lex::json::{self, Object, Value};
use purrdf_lex::json_pointer::escape_token;
use purrdf_sparql_eval::RegistryId;
use purrdf_text::Fixed;

use crate::error::PlanError;
use crate::fuse::TopK;
use crate::iri::{Iri, Term};
use crate::plan::{
    DepthInputs, Plan, PlanOrigin, ProducerBinding, ProducerDecision, RejectionReason,
    StatisticsEntries, StatisticsEntry, StatisticsSnapshot, UnservedReason, UnservedTerm,
};
use crate::request::{Metric, ReadBound, RequestTerm, RetrievalRequest};

type Decoded<T> = Result<T, PlanError>;

/// A shape refusal at the value being read; [`within`] extends its pointer
/// outward as the error leaves each enclosing value.
fn invalid(reason: impl Into<String>) -> PlanError {
    PlanError::InvalidJson {
        pointer: String::new(),
        reason: reason.into(),
    }
}

/// Prefix `token` onto the pointer of a shape refusal raised inside the member
/// or item `token` names. Construction-law refusals pass through unchanged.
fn within<T>(token: &str, result: Decoded<T>) -> Decoded<T> {
    result.map_err(|error| match error {
        PlanError::InvalidJson { pointer, reason } => PlanError::InvalidJson {
            pointer: format!("/{}{pointer}", escape_token(token)),
            reason,
        },
        other => other,
    })
}

fn expected(what: &str, found: &Value) -> PlanError {
    invalid(format!("expected {what}, found {}", found.kind().name()))
}

fn string(value: &Value) -> Decoded<String> {
    value
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| expected("a string", value))
}

/// An integer lexeme that fits `T`. `-0` is `0`, and a fraction or exponent is
/// refused even where it denotes an integer.
fn integer<T: TryFrom<i128>>(value: &Value, what: &str) -> Decoded<T> {
    let number = value
        .as_number()
        .ok_or_else(|| expected(&format!("an integer that fits {what}"), value))?;
    number
        .as_i128()
        .and_then(|integer| T::try_from(integer).ok())
        .ok_or_else(|| {
            invalid(format!(
                "expected an integer that fits {what}, found {}",
                number.lexeme()
            ))
        })
}

fn u32_of(value: &Value) -> Decoded<u32> {
    integer(value, "u32")
}

fn u64_of(value: &Value) -> Decoded<u64> {
    integer(value, "u64")
}

/// The binary32 nearest a JSON number: correctly rounded to binary64 first, and
/// then to binary32, a double rounding that is exact for every decimal of
/// binary32's precision. A number beyond binary32's range is refused, since the
/// JSON form writes no infinity.
fn f32_of(value: &Value) -> Decoded<f32> {
    let number = value
        .as_number()
        .ok_or_else(|| expected("a number", value))?;
    #[allow(
        clippy::cast_possible_truncation,
        reason = "the binary64 to binary32 rounding is the conversion this reads"
    )]
    let component = number.as_f64() as f32;
    if component.is_finite() {
        Ok(component)
    } else {
        Err(invalid(format!(
            "{} denotes no finite f32",
            number.lexeme()
        )))
    }
}

fn iri_of(value: &Value) -> Decoded<Iri> {
    Iri::parse(&string(value)?)
}

fn array<T>(value: &Value, item: impl Fn(&Value) -> Decoded<T>) -> Decoded<Vec<T>> {
    let items = value
        .as_array()
        .ok_or_else(|| expected("an array", value))?;
    items
        .iter()
        .enumerate()
        .map(|(index, entry)| within(&index.to_string(), item(entry)))
        .collect()
}

fn nullable<T>(
    value: Option<&Value>,
    read: impl FnOnce(&Value) -> Decoded<T>,
) -> Decoded<Option<T>> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(present) => read(present).map(Some),
    }
}

fn fixed_to_json(value: Option<Fixed>) -> Value {
    value.map(Fixed::into_raw).into()
}

fn fixed_of(value: &Value) -> Decoded<Fixed> {
    integer::<i128>(value, "i128").map(Fixed::from_raw)
}

/// The members of a JSON object a struct is read from.
struct Fields<'a>(&'a Object);

impl<'a> Fields<'a> {
    fn of(value: &'a Value) -> Decoded<Self> {
        value
            .as_object()
            .map(Self)
            .ok_or_else(|| expected("an object", value))
    }

    /// The member `name`, or `None` when absent; a repeat is refused.
    fn member(&self, name: &str) -> Decoded<Option<&'a Value>> {
        if self.0.count(name) > 1 {
            return within(name, Err(invalid("the member is repeated")));
        }
        Ok(self.0.get(name))
    }

    /// Read the member `name`, which must be present.
    fn required<T>(&self, name: &str, read: impl FnOnce(&Value) -> Decoded<T>) -> Decoded<T> {
        let value = self
            .member(name)?
            .ok_or_else(|| invalid(format!("missing member `{name}`")))?;
        within(name, read(value))
    }

    /// Read the member `name`, absent or `null` being `None`.
    fn optional<T>(
        &self,
        name: &str,
        read: impl FnOnce(&Value) -> Decoded<T>,
    ) -> Decoded<Option<T>> {
        let value = self.member(name)?;
        within(name, nullable(value, read))
    }

    /// Read the member `name`, which must be present, `null` being `None`.
    fn present_nullable<T>(
        &self,
        name: &str,
        read: impl FnOnce(&Value) -> Decoded<T>,
    ) -> Decoded<Option<T>> {
        self.required(name, |value| nullable(Some(value), read))
    }
}

/// An externally tagged enum value: the variant name, and its content (`None`
/// for a variant written as a bare name).
fn variant(value: &Value) -> Decoded<(&str, Option<&Value>)> {
    match value {
        Value::String(name) => Ok((name, None)),
        Value::Object(object) if object.len() == 1 => {
            let (name, content) = &object.members()[0];
            Ok((name, Some(content)))
        }
        other => Err(expected(
            "a variant name or a one-member object naming a variant",
            other,
        )),
    }
}

/// A unit variant's content: none, or `null`.
fn unit(name: &str, content: Option<&Value>) -> Decoded<()> {
    match content {
        None | Some(Value::Null) => Ok(()),
        Some(other) => within(name, Err(expected("null", other))),
    }
}

/// A newtype or struct variant's content, which must be present.
fn content<'a>(name: &str, content: Option<&'a Value>) -> Decoded<&'a Value> {
    content.ok_or_else(|| invalid(format!("the variant `{name}` carries a value")))
}

fn unknown_variant(name: &str, known: &[&str]) -> PlanError {
    invalid(format!(
        "unknown variant `{name}`, expected one of {}",
        known.join(", ")
    ))
}

/// A unit-only enum's variant, looked up by name.
fn unit_variant<T: Copy>(value: &Value, variants: &[(&str, T)]) -> Decoded<T> {
    let (name, body) = variant(value)?;
    let found = variants
        .iter()
        .find(|(known, _)| *known == name)
        .map(|(_, variant)| *variant)
        .ok_or_else(|| {
            let names: Vec<&str> = variants.iter().map(|(known, _)| *known).collect();
            unknown_variant(name, &names)
        })?;
    unit(name, body)?;
    Ok(found)
}

/// The name a unit-only enum's variant is written as.
fn unit_name<T: Copy + PartialEq>(value: T, variants: &[(&'static str, T)]) -> &'static str {
    variants
        .iter()
        .find(|(_, variant)| *variant == value)
        .map(|(name, _)| *name)
        .expect("every variant is listed")
}

/// A struct variant: `{"Name": {fields}}`.
fn tagged(name: &str, fields: Object) -> Value {
    Object::new().with(name, fields).into()
}

impl Iri {
    /// The IRI as a JSON string.
    #[must_use]
    pub fn to_json(&self) -> Value {
        self.as_str().into()
    }

    /// Read an IRI from a JSON string, validating it as [`Iri::parse`] does.
    ///
    /// # Errors
    ///
    /// [`PlanError::InvalidJson`] when the value is not a string, and
    /// [`PlanError::InvalidIri`] when the string is not an IRI.
    pub fn from_json(value: &Value) -> Result<Self, PlanError> {
        iri_of(value)
    }
}

impl Term {
    /// The term's canonical text as a JSON string.
    #[must_use]
    pub fn to_json(&self) -> Value {
        self.as_str().into()
    }

    /// Read a term from a JSON string, carried verbatim.
    ///
    /// # Errors
    ///
    /// [`PlanError::InvalidJson`] when the value is not a string.
    pub fn from_json(value: &Value) -> Result<Self, PlanError> {
        string(value).map(Self::new)
    }
}

impl TopK {
    /// The bound as a JSON integer.
    #[must_use]
    pub fn to_json(self) -> Value {
        self.get().into()
    }

    /// Read a bound from a JSON integer.
    ///
    /// # Errors
    ///
    /// [`PlanError::InvalidJson`] unless the value is an integer that fits
    /// `usize`.
    pub fn from_json(value: &Value) -> Result<Self, PlanError> {
        integer(value, "usize").map(Self::new)
    }
}

const METRICS: [(&str, Metric); 3] = [
    ("Cosine", Metric::Cosine),
    ("Dot", Metric::Dot),
    ("Euclidean", Metric::Euclidean),
];

impl Metric {
    /// The metric's variant name as a JSON string.
    #[must_use]
    pub fn to_json(self) -> Value {
        unit_name(self, &METRICS).into()
    }

    /// Read a metric from its variant name.
    ///
    /// # Errors
    ///
    /// [`PlanError::InvalidJson`] for any other value.
    pub fn from_json(value: &Value) -> Result<Self, PlanError> {
        unit_variant(value, &METRICS)
    }
}

const REQUEST_TERMS: [&str; 6] = [
    "Lexical",
    "Vector",
    "Spatial",
    "Temporal",
    "NumericRange",
    "EntitySeed",
];

impl RequestTerm {
    /// The term as an externally tagged JSON object.
    #[must_use]
    pub fn to_json(&self) -> Value {
        match self {
            Self::Lexical {
                text,
                language,
                predicate,
            } => tagged(
                "Lexical",
                Object::new()
                    .with("text", text)
                    .with("language", language.as_deref())
                    .with("predicate", predicate.as_ref().map(Iri::to_json)),
            ),
            Self::Vector {
                embedding,
                metric,
                index_hint,
            } => tagged(
                "Vector",
                Object::new()
                    .with("embedding", embedding.as_slice())
                    .with("metric", metric.to_json())
                    .with("index_hint", index_hint.as_deref()),
            ),
            Self::Spatial {
                geometry,
                predicate,
                max_distance,
            } => tagged(
                "Spatial",
                Object::new()
                    .with("geometry", geometry)
                    .with("predicate", predicate.to_json())
                    .with("max_distance", fixed_to_json(*max_distance)),
            ),
            Self::Temporal {
                predicate,
                lower,
                upper,
            } => tagged(
                "Temporal",
                Object::new()
                    .with("predicate", predicate.to_json())
                    .with("lower", lower.as_deref())
                    .with("upper", upper.as_deref()),
            ),
            Self::NumericRange {
                predicate,
                lower,
                upper,
            } => tagged(
                "NumericRange",
                Object::new()
                    .with("predicate", predicate.to_json())
                    .with("lower", fixed_to_json(*lower))
                    .with("upper", fixed_to_json(*upper)),
            ),
            Self::EntitySeed { entity } => {
                tagged("EntitySeed", Object::new().with("entity", entity.to_json()))
            }
        }
    }

    /// Read a term from its externally tagged JSON object.
    ///
    /// # Errors
    ///
    /// [`PlanError::InvalidJson`] for a shape error or an unknown variant,
    /// [`PlanError::InvalidIri`] for a predicate that is not an IRI.
    pub fn from_json(value: &Value) -> Result<Self, PlanError> {
        let (name, body) = variant(value)?;
        if !REQUEST_TERMS.contains(&name) {
            return Err(unknown_variant(name, &REQUEST_TERMS));
        }
        let body = content(name, body)?;
        within(name, Self::from_fields(name, &Fields::of(body)?))
    }

    fn from_fields(name: &str, fields: &Fields<'_>) -> Decoded<Self> {
        Ok(match name {
            "Lexical" => Self::Lexical {
                text: fields.required("text", string)?,
                language: fields.optional("language", string)?,
                predicate: fields.optional("predicate", iri_of)?,
            },
            "Vector" => Self::Vector {
                embedding: fields.required("embedding", |value| array(value, f32_of))?,
                metric: fields.required("metric", Metric::from_json)?,
                index_hint: fields.optional("index_hint", string)?,
            },
            "Spatial" => Self::Spatial {
                geometry: fields.required("geometry", string)?,
                predicate: fields.required("predicate", iri_of)?,
                max_distance: fields.present_nullable("max_distance", fixed_of)?,
            },
            "Temporal" => Self::Temporal {
                predicate: fields.required("predicate", iri_of)?,
                lower: fields.optional("lower", string)?,
                upper: fields.optional("upper", string)?,
            },
            "NumericRange" => Self::NumericRange {
                predicate: fields.required("predicate", iri_of)?,
                lower: fields.present_nullable("lower", fixed_of)?,
                upper: fields.present_nullable("upper", fixed_of)?,
            },
            _ => Self::EntitySeed {
                entity: fields.required("entity", Term::from_json)?,
            },
        })
    }
}

impl ReadBound {
    /// The bound as an externally tagged JSON value: `"Complete"`, or
    /// `{"Bounded": rows}`.
    #[must_use]
    pub fn to_json(self) -> Value {
        match self {
            Self::Bounded(top_k) => Object::new().with("Bounded", top_k.to_json()).into(),
            Self::Complete => "Complete".into(),
        }
    }

    /// Read a bound from its externally tagged JSON value.
    ///
    /// # Errors
    ///
    /// [`PlanError::InvalidJson`] for a shape error or an unknown variant.
    pub fn from_json(value: &Value) -> Result<Self, PlanError> {
        match variant(value)? {
            ("Complete", body) => unit("Complete", body).map(|()| Self::Complete),
            ("Bounded", body) => within(
                "Bounded",
                content("Bounded", body).and_then(TopK::from_json),
            )
            .map(Self::Bounded),
            (name, _) => Err(unknown_variant(name, &["Bounded", "Complete"])),
        }
    }
}

impl RetrievalRequest {
    /// The request as a JSON object: `terms`, then `bound`.
    #[must_use]
    pub fn to_json(&self) -> Value {
        Object::new()
            .with(
                "terms",
                Value::array(self.terms.iter().map(RequestTerm::to_json)),
            )
            .with("bound", self.bound.to_json())
            .into()
    }

    /// Read a request from its JSON object.
    ///
    /// # Errors
    ///
    /// [`PlanError::InvalidJson`] for a shape error, and a term's own refusal
    /// otherwise.
    pub fn from_json(value: &Value) -> Result<Self, PlanError> {
        let fields = Fields::of(value)?;
        Ok(Self {
            terms: fields.required("terms", |value| array(value, RequestTerm::from_json))?,
            bound: fields.required("bound", ReadBound::from_json)?,
        })
    }
}

impl ProducerBinding {
    /// The binding as a JSON object: `producer`, `stratum`, `request_terms`.
    #[must_use]
    pub fn to_json(&self) -> Value {
        Object::new()
            .with("producer", &self.producer)
            .with("stratum", self.stratum.to_json())
            .with("request_terms", self.request_terms.as_slice())
            .into()
    }

    /// Read a binding from its JSON object.
    ///
    /// # Errors
    ///
    /// [`PlanError::InvalidJson`] for a shape error, [`PlanError::InvalidIri`]
    /// for a stratum that is not an IRI.
    pub fn from_json(value: &Value) -> Result<Self, PlanError> {
        let fields = Fields::of(value)?;
        Ok(Self {
            producer: fields.required("producer", string)?,
            stratum: fields.required("stratum", iri_of)?,
            request_terms: fields.required("request_terms", |value| array(value, u32_of))?,
        })
    }
}

const REJECTION_REASONS: [(&str, RejectionReason); 4] = [
    ("NotRanked", RejectionReason::NotRanked),
    ("NoAcceptedTerm", RejectionReason::NoAcceptedTerm),
    ("DepthExceeded", RejectionReason::DepthExceeded),
    (
        "UnsatisfiedConstraint",
        RejectionReason::UnsatisfiedConstraint,
    ),
];

impl RejectionReason {
    /// The reason's variant name as a JSON string.
    #[must_use]
    pub fn to_json(self) -> Value {
        unit_name(self, &REJECTION_REASONS).into()
    }

    /// Read a reason from its variant name.
    ///
    /// # Errors
    ///
    /// [`PlanError::InvalidJson`] for any other value.
    pub fn from_json(value: &Value) -> Result<Self, PlanError> {
        unit_variant(value, &REJECTION_REASONS)
    }
}

impl ProducerDecision {
    /// The decision as an externally tagged JSON object.
    #[must_use]
    pub fn to_json(&self) -> Value {
        match self {
            Self::Selected { producer, stratum } => tagged(
                "Selected",
                Object::new()
                    .with("producer", producer)
                    .with("stratum", stratum.to_json()),
            ),
            Self::Rejected { producer, reason } => tagged(
                "Rejected",
                Object::new()
                    .with("producer", producer)
                    .with("reason", reason.to_json()),
            ),
        }
    }

    /// Read a decision from its externally tagged JSON object.
    ///
    /// # Errors
    ///
    /// [`PlanError::InvalidJson`] for a shape error or an unknown variant,
    /// [`PlanError::InvalidIri`] for a stratum that is not an IRI.
    pub fn from_json(value: &Value) -> Result<Self, PlanError> {
        let (name, body) = variant(value)?;
        let read = |body: Option<&Value>| -> Decoded<Self> {
            let fields = Fields::of(content(name, body)?)?;
            let producer = fields.required("producer", string)?;
            Ok(if name == "Selected" {
                Self::Selected {
                    producer,
                    stratum: fields.required("stratum", iri_of)?,
                }
            } else {
                Self::Rejected {
                    producer,
                    reason: fields.required("reason", RejectionReason::from_json)?,
                }
            })
        };
        match name {
            "Selected" | "Rejected" => within(name, read(body)),
            _ => Err(unknown_variant(name, &["Selected", "Rejected"])),
        }
    }
}

const UNSERVED_REASONS: [(&str, UnservedReason); 4] = [
    ("NoProducerAccepts", UnservedReason::NoProducerAccepts),
    (
        "EveryAcceptingProducerRejected",
        UnservedReason::EveryAcceptingProducerRejected,
    ),
    (
        "AcceptedWithoutPlacement",
        UnservedReason::AcceptedWithoutPlacement,
    ),
    ("Unbound", UnservedReason::Unbound),
];

impl UnservedReason {
    /// The reason's variant name as a JSON string.
    #[must_use]
    pub fn to_json(self) -> Value {
        unit_name(self, &UNSERVED_REASONS).into()
    }

    /// Read a reason from its variant name.
    ///
    /// # Errors
    ///
    /// [`PlanError::InvalidJson`] for any other value.
    pub fn from_json(value: &Value) -> Result<Self, PlanError> {
        unit_variant(value, &UNSERVED_REASONS)
    }
}

impl UnservedTerm {
    /// The record as a JSON object: `request_term`, then `reason`.
    #[must_use]
    pub fn to_json(&self) -> Value {
        Object::new()
            .with("request_term", self.request_term)
            .with("reason", self.reason.to_json())
            .into()
    }

    /// Read a record from its JSON object.
    ///
    /// # Errors
    ///
    /// [`PlanError::InvalidJson`] for a shape error.
    pub fn from_json(value: &Value) -> Result<Self, PlanError> {
        let fields = Fields::of(value)?;
        Ok(Self {
            request_term: fields.required("request_term", u32_of)?,
            reason: fields.required("reason", UnservedReason::from_json)?,
        })
    }
}

impl DepthInputs {
    /// The inputs as a JSON object, members in field order.
    #[must_use]
    pub fn to_json(&self) -> Value {
        Object::new()
            .with("declared", self.declared)
            .with("cardinality", self.cardinality)
            .with("selectivity_ppm", self.selectivity_ppm)
            .with("selectivity_terms", self.selectivity_terms.as_slice())
            .with("licensed_prefix", self.licensed_prefix)
            .into()
    }

    /// Read the inputs from their JSON object.
    ///
    /// # Errors
    ///
    /// [`PlanError::InvalidJson`] for a shape error.
    pub fn from_json(value: &Value) -> Result<Self, PlanError> {
        let fields = Fields::of(value)?;
        Ok(Self {
            declared: fields.required("declared", u64_of)?,
            cardinality: fields.optional("cardinality", u64_of)?,
            selectivity_ppm: fields.optional("selectivity_ppm", u64_of)?,
            selectivity_terms: fields
                .required("selectivity_terms", |value| array(value, u32_of))?,
            licensed_prefix: fields.optional("licensed_prefix", u64_of)?,
        })
    }
}

impl StatisticsEntry {
    /// The entry as a JSON object, members in field order.
    #[must_use]
    pub fn to_json(&self) -> Value {
        Object::new()
            .with("subject", &self.subject)
            .with("cardinality", self.cardinality)
            .with("selectivity_ppm", self.selectivity_ppm)
            .with("selectivity_terms", self.selectivity_terms.as_slice())
            .into()
    }

    /// Read an entry from its JSON object.
    ///
    /// # Errors
    ///
    /// [`PlanError::InvalidJson`] for a shape error.
    pub fn from_json(value: &Value) -> Result<Self, PlanError> {
        let fields = Fields::of(value)?;
        Ok(Self {
            subject: fields.required("subject", string)?,
            cardinality: fields.optional("cardinality", u64_of)?,
            selectivity_ppm: fields.optional("selectivity_ppm", u64_of)?,
            selectivity_terms: fields
                .required("selectivity_terms", |value| array(value, u32_of))?,
        })
    }
}

impl StatisticsEntries {
    /// The entries as a JSON array, in subject order.
    #[must_use]
    pub fn to_json(&self) -> Value {
        Value::array(self.iter().map(StatisticsEntry::to_json))
    }

    /// Read entries from a JSON array in any order, held to
    /// [`StatisticsEntries::new`]'s law.
    ///
    /// # Errors
    ///
    /// [`PlanError::InvalidJson`] for a shape error, and
    /// [`PlanError::DuplicateStatisticsSubject`] when two entries name one
    /// subject.
    pub fn from_json(value: &Value) -> Result<Self, PlanError> {
        Self::new(array(value, StatisticsEntry::from_json)?)
    }
}

impl StatisticsSnapshot {
    /// The snapshot as a JSON object: `source`, `revision`, `entries`.
    #[must_use]
    pub fn to_json(&self) -> Value {
        Object::new()
            .with("source", &self.source)
            .with("revision", &self.revision)
            .with("entries", self.entries.to_json())
            .into()
    }

    /// Read a snapshot from its JSON object.
    ///
    /// # Errors
    ///
    /// [`PlanError::InvalidJson`] for a shape error, and
    /// [`PlanError::DuplicateStatisticsSubject`] when two entries name one
    /// subject.
    pub fn from_json(value: &Value) -> Result<Self, PlanError> {
        let fields = Fields::of(value)?;
        Ok(Self {
            source: fields.required("source", string)?,
            revision: fields.required("revision", string)?,
            entries: fields.required("entries", StatisticsEntries::from_json)?,
        })
    }
}

/// An object keyed by IRI, read into a map: each key is parsed as an IRI, and
/// the last member naming an IRI wins.
fn iri_keyed<T>(value: &Value, read: impl Fn(&Value) -> Decoded<T>) -> Decoded<Vec<(Iri, T)>> {
    let object = value
        .as_object()
        .ok_or_else(|| expected("an object", value))?;
    object
        .iter()
        .map(|(key, member)| Ok((Iri::parse(key)?, within(key, read(member))?)))
        .collect()
}

impl Plan {
    /// The plan as a JSON object: every field but [`Plan::origin`], in
    /// declaration order, with both stratum maps in ascending IRI order.
    ///
    /// ```rust
    /// # use purrdf_retrieval::Plan;
    /// fn round_trip(plan: &Plan) -> Plan {
    ///     Plan::from_json(&plan.to_json()).expect("a plan reads its own JSON")
    /// }
    /// ```
    #[must_use]
    pub fn to_json(&self) -> Value {
        let mut depths: Vec<(&Iri, u32)> = self
            .stratum_depths
            .iter()
            .map(|(stratum, depth)| (stratum, *depth))
            .collect();
        depths.sort_unstable_by(|left, right| left.0.as_str().cmp(right.0.as_str()));
        let depths: Object = depths
            .into_iter()
            .map(|(stratum, depth)| (stratum.as_str(), depth))
            .collect();
        let derivations: Object = self
            .stratum_derivations
            .iter()
            .map(|(stratum, inputs)| (stratum.as_str(), inputs.to_json()))
            .collect();
        Object::new()
            .with("version", self.version)
            .with(
                "request_terms",
                Value::array(self.request_terms.iter().map(RequestTerm::to_json)),
            )
            .with("read_bound", self.read_bound.to_json())
            .with(
                "producer_bindings",
                Value::array(self.producer_bindings.iter().map(ProducerBinding::to_json)),
            )
            .with(
                "producer_decisions",
                Value::array(
                    self.producer_decisions
                        .iter()
                        .map(ProducerDecision::to_json),
                ),
            )
            .with(
                "unserved_terms",
                Value::array(self.unserved_terms.iter().map(UnservedTerm::to_json)),
            )
            .with("stratum_depths", depths)
            .with("stratum_derivations", derivations)
            .with("statistics_snapshot", self.statistics_snapshot.to_json())
            .with("registry_instance_id", self.registry_instance_id.as_u64())
            .with(
                "registry_content_fingerprint",
                &self.registry_content_fingerprint,
            )
            .into()
    }

    /// Read a plan from its JSON object. The plan's origin is
    /// [`PlanOrigin::Deserialized`]: its registry instance id was minted
    /// elsewhere.
    ///
    /// Reading checks the document's shape and each value's construction law,
    /// never the plan's coherence; admission does that, for a plan from any
    /// source.
    ///
    /// # Errors
    ///
    /// [`PlanError::InvalidJson`] for a shape error, [`PlanError::InvalidIri`]
    /// for an IRI that does not parse, and
    /// [`PlanError::DuplicateStatisticsSubject`] for a statistics subject named
    /// twice.
    pub fn from_json(value: &Value) -> Result<Self, PlanError> {
        let fields = Fields::of(value)?;
        Ok(Self {
            version: fields.required("version", |value| integer(value, "u16"))?,
            request_terms: fields.required("request_terms", |value| {
                array(value, RequestTerm::from_json)
            })?,
            read_bound: fields.required("read_bound", ReadBound::from_json)?,
            producer_bindings: fields.required("producer_bindings", |value| {
                array(value, ProducerBinding::from_json)
            })?,
            producer_decisions: fields.required("producer_decisions", |value| {
                array(value, ProducerDecision::from_json)
            })?,
            unserved_terms: fields.required("unserved_terms", |value| {
                array(value, UnservedTerm::from_json)
            })?,
            stratum_depths: fields.required("stratum_depths", |value| {
                iri_keyed(value, u32_of)
                    .map(|entries| entries.into_iter().collect::<FastMap<_, _>>())
            })?,
            stratum_derivations: fields.required("stratum_derivations", |value| {
                iri_keyed(value, DepthInputs::from_json)
                    .map(|entries| entries.into_iter().collect::<BTreeMap<_, _>>())
            })?,
            statistics_snapshot: fields
                .required("statistics_snapshot", StatisticsSnapshot::from_json)?,
            registry_instance_id: fields.required("registry_instance_id", |value| {
                u64_of(value).map(RegistryId::from_raw)
            })?,
            registry_content_fingerprint: fields
                .required("registry_content_fingerprint", string)?,
            origin: PlanOrigin::Deserialized,
        })
    }

    /// [`Plan::to_json`], written as compact JSON.
    #[must_use]
    pub fn to_json_string(&self) -> String {
        json::write_compact(&self.to_json())
    }

    /// Read a plan from JSON text ([`purrdf_lex::json::read`]'s default
    /// limits), as [`Plan::from_json`] reads the value.
    ///
    /// # Errors
    ///
    /// [`PlanError::InvalidJson`] when the text is not JSON, and
    /// [`Plan::from_json`]'s refusals otherwise.
    pub fn from_json_str(text: &str) -> Result<Self, PlanError> {
        let value = json::read(text).map_err(|error| invalid(format!("not JSON: {error}")))?;
        Self::from_json(&value)
    }
}

#[cfg(test)]
mod tests {
    use purrdf_lex::json::{self, Value};

    use super::{f32_of, integer};
    use crate::error::PlanError;
    use crate::{Iri, Metric, ReadBound, RequestTerm, StatisticsSnapshot};

    fn read(text: &str) -> Value {
        json::read(text).expect("test JSON")
    }

    fn pointer_of(error: &PlanError) -> &str {
        match error {
            PlanError::InvalidJson { pointer, .. } => pointer,
            other => panic!("expected a JSON shape refusal, got {other:?}"),
        }
    }

    #[test]
    fn an_integer_field_reads_integer_lexemes_that_fit_and_refuses_the_rest() {
        assert_eq!(
            integer::<u32>(&read("4294967295"), "u32").ok(),
            Some(u32::MAX)
        );
        assert_eq!(integer::<u32>(&read("-0"), "u32").ok(), Some(0));
        assert!(integer::<u32>(&read("4294967296"), "u32").is_err());
        assert!(integer::<u32>(&read("-1"), "u32").is_err());
        assert!(integer::<u32>(&read("1.0"), "u32").is_err());
        assert!(integer::<u32>(&read("1e2"), "u32").is_err());
        assert!(integer::<u32>(&read("\"1\""), "u32").is_err());
        assert_eq!(
            integer::<i128>(&read("-170141183460469231731687303715884105728"), "i128").ok(),
            Some(i128::MIN)
        );
    }

    #[test]
    fn an_embedding_component_beyond_binary32_is_refused_and_its_neighbour_reads() {
        assert!(f32_of(&read("1e39")).is_err());
        assert!(f32_of(&read("-1e39")).is_err());
        assert_eq!(f32_of(&read("3.4028235e38")).ok(), Some(f32::MAX));
        assert_eq!(f32_of(&read("2")).ok(), Some(2.0));
        assert_eq!(f32_of(&read("0.1")).ok(), Some(0.1_f32));
    }

    #[test]
    fn a_unit_variant_reads_as_a_name_or_a_null_bodied_object() {
        assert_eq!(Metric::from_json(&read("\"Dot\"")).ok(), Some(Metric::Dot));
        assert_eq!(
            Metric::from_json(&read(r#"{"Dot":null}"#)).ok(),
            Some(Metric::Dot)
        );
        assert!(Metric::from_json(&read(r#"{"Dot":1}"#)).is_err());
        assert!(Metric::from_json(&read("\"Manhattan\"")).is_err());
        assert_eq!(
            ReadBound::from_json(&read(r#"{"Bounded":5}"#)).ok(),
            Some(ReadBound::Bounded(crate::TopK::new(5)))
        );
        assert!(ReadBound::from_json(&read("\"Bounded\"")).is_err());
        assert!(ReadBound::from_json(&read(r#"{"Bounded":5,"Complete":null}"#)).is_err());
    }

    #[test]
    fn a_repeated_member_is_refused_where_a_single_one_reads() {
        let single = r#"{"source":"s","revision":"r","entries":[]}"#;
        assert!(StatisticsSnapshot::from_json(&read(single)).is_ok());
        let repeated = r#"{"source":"s","revision":"r","revision":"q","entries":[]}"#;
        let error = StatisticsSnapshot::from_json(&read(repeated)).expect_err("repeated");
        assert_eq!(pointer_of(&error), "/revision");
        let unknown = r#"{"source":"s","revision":"r","entries":[],"note":1,"note":2}"#;
        assert!(
            StatisticsSnapshot::from_json(&read(unknown)).is_ok(),
            "a member no field names is ignored, repeated or not"
        );
    }

    #[test]
    fn a_shape_refusal_names_the_pointer_of_the_offending_value() {
        let term = read(r#"{"Vector":{"embedding":[1,"x"],"metric":"Dot","index_hint":null}}"#);
        let error = RequestTerm::from_json(&term).expect_err("a string component");
        assert_eq!(pointer_of(&error), "/Vector/embedding/1");

        let missing =
            read(r#"{"Spatial":{"geometry":"POINT(0 0)","predicate":"http://example.org/p"}}"#);
        let error = RequestTerm::from_json(&missing).expect_err("max_distance is required");
        assert_eq!(pointer_of(&error), "/Spatial");
        let present = read(
            r#"{"Spatial":{"geometry":"POINT(0 0)","predicate":"http://example.org/p","max_distance":null}}"#,
        );
        assert!(RequestTerm::from_json(&present).is_ok());

        let lexical = read(r#"{"Lexical":{"text":"a"}}"#);
        assert_eq!(
            RequestTerm::from_json(&lexical).ok(),
            Some(RequestTerm::Lexical {
                text: "a".to_owned(),
                language: None,
                predicate: None,
            }),
            "an absent plain optional is None"
        );
    }

    /// The document shape, pinned byte for byte: struct members in field order,
    /// externally tagged variants, `null` for an absent optional, a fixed-point
    /// value as its raw integer and an embedding component as its shortest
    /// binary32 lexeme.
    #[test]
    fn every_request_term_writes_its_pinned_document_and_reads_it_back() {
        let predicate = Iri::parse("http://example.org/p").expect("IRI");
        let cases = [
            (
                RequestTerm::Lexical {
                    text: "cat \"x\"".to_owned(),
                    language: Some("en".to_owned()),
                    predicate: None,
                },
                r#"{"Lexical":{"text":"cat \"x\"","language":"en","predicate":null}}"#,
            ),
            (
                RequestTerm::Vector {
                    embedding: vec![0.1, 1.0, -2.5e-8, 3.0e20],
                    metric: Metric::Cosine,
                    index_hint: None,
                },
                r#"{"Vector":{"embedding":[0.1,1.0,-2.5e-8,3e+20],"metric":"Cosine","index_hint":null}}"#,
            ),
            (
                RequestTerm::Spatial {
                    geometry: "POINT(0 0)".to_owned(),
                    predicate: predicate.clone(),
                    max_distance: Some(crate::Fixed::from_raw(-12_500)),
                },
                r#"{"Spatial":{"geometry":"POINT(0 0)","predicate":"http://example.org/p","max_distance":-12500}}"#,
            ),
            (
                RequestTerm::Temporal {
                    predicate: predicate.clone(),
                    lower: None,
                    upper: Some("2026".to_owned()),
                },
                r#"{"Temporal":{"predicate":"http://example.org/p","lower":null,"upper":"2026"}}"#,
            ),
            (
                RequestTerm::NumericRange {
                    predicate,
                    lower: Some(crate::Fixed::from_raw(i128::MAX)),
                    upper: None,
                },
                r#"{"NumericRange":{"predicate":"http://example.org/p","lower":170141183460469231731687303715884105727,"upper":null}}"#,
            ),
            (
                RequestTerm::EntitySeed {
                    entity: crate::Term::new("<http://example.org/a>"),
                },
                r#"{"EntitySeed":{"entity":"<http://example.org/a>"}}"#,
            ),
        ];
        for (term, text) in cases {
            assert_eq!(json::write_compact(&term.to_json()), text);
            assert_eq!(RequestTerm::from_json(&read(text)).ok(), Some(term));
        }
        assert_eq!(
            json::write_compact(&ReadBound::Complete.to_json()),
            r#""Complete""#
        );
        assert_eq!(
            json::write_compact(&ReadBound::Bounded(crate::TopK::new(5)).to_json()),
            r#"{"Bounded":5}"#
        );
    }

    #[test]
    fn a_non_finite_embedding_component_writes_null_which_does_not_read_back() {
        let term = RequestTerm::Vector {
            embedding: vec![f32::NAN],
            metric: Metric::Dot,
            index_hint: None,
        };
        let value = term.to_json();
        assert_eq!(
            json::write_compact(&value),
            r#"{"Vector":{"embedding":[null],"metric":"Dot","index_hint":null}}"#
        );
        let error = RequestTerm::from_json(&value).expect_err("null is not a component");
        assert_eq!(pointer_of(&error), "/Vector/embedding/0");
    }

    #[test]
    fn an_iri_that_does_not_parse_keeps_its_own_refusal() {
        assert!(matches!(
            Iri::from_json(&read("\"not an iri\"")),
            Err(PlanError::InvalidIri { .. })
        ));
        assert!(Iri::from_json(&read("\"http://example.org/a\"")).is_ok());
    }
}

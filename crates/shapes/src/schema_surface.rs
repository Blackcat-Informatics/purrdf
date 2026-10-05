// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Deterministic OWL/RDFS property-surface derivation for developer schemas.
//!
//! This is deliberately a bounded schema theory, not an instance reasoner. It
//! computes one sparse class/property relation with source-axiom provenance;
//! every schema emitter and both public coverage manifests project from that
//! relation.
//!
//! Anonymous OWL class expressions are part of the theory. A restriction
//! (`owl:someValuesFrom`, `owl:allValuesFrom`, `owl:hasValue`, `owl:hasSelf`
//! and the six cardinality forms) or a boolean form (`owl:unionOf`,
//! `owl:intersectionOf`, `owl:complementOf`, `owl:oneOf`) asserted of a named
//! class through `rdfs:subClassOf` or `owl:equivalentClass` is recorded with
//! its source axiom, inherited by every subclass, projected onto the class's
//! developer schema where a JSON Schema keyword carries it, and reported in the
//! class-expression manifest with its outcome either way. Only structurally
//! malformed input — a restriction without `owl:onProperty`, a conflicting or
//! ill-typed cardinality, an ill-formed or cyclic RDF list, an expression that
//! contains itself — fails, with a typed error.
//!
//! Every walk over an expression is bounded: an expression nests at most
//! `MAX_OWL_EXPRESSION_DEPTH` levels and one request expands at most
//! `MAX_SCHEMA_RELATIONS` expression nodes, and RDF lists are read by the
//! strict iterative walker, so neither recursion depth nor work grows with the
//! input beyond those ceilings.

use std::collections::{BTreeMap, BTreeSet};

use ::purrdf_rdf::RdfDataset;
use purrdf_xsd::XsdDatatype;

use crate::data::{GraphFilter, native_quads, objects_of};
use crate::json_schema::{
    MAX_OWL_EXPRESSION_DEPTH, MAX_SCHEMA_CLASSES, MAX_SCHEMA_PROPERTIES, MAX_SCHEMA_RELATIONS,
    SchemaClassExpressionAxiom, SchemaClassExpressionCoverage, SchemaClassExpressionReport,
    SchemaClassPropertyCoverage, SchemaCompileError, SchemaCompileRequest, SchemaCoveragePrecision,
    SchemaCoverageProvenance, SchemaCoverageReport, SchemaCoverageStatus,
    SchemaExpressionComponent, SchemaExpressionOutcome, SchemaPropertyCoverage, SchemaSurfaceMode,
};
use crate::model::{rdf, rdfs};
use crate::shapes::{ClosedMode, Constraint, Path, Shape, Target};
use crate::term::{NamedNode, Term};

use purrdf_iri::vocab::owl::{
    ALL_VALUES_FROM as OWL_ALL_VALUES_FROM, ANNOTATION_PROPERTY as OWL_ANNOTATION_PROPERTY,
    CARDINALITY as OWL_CARDINALITY, CLASS as OWL_CLASS, COMPLEMENT_OF as OWL_COMPLEMENT_OF,
    DATA_RANGE as OWL_DATA_RANGE, DATATYPE_COMPLEMENT_OF as OWL_DATATYPE_COMPLEMENT_OF,
    DATATYPE_PROPERTY as OWL_DATATYPE_PROPERTY, EQUIVALENT_CLASS as OWL_EQUIVALENT_CLASS,
    EQUIVALENT_PROPERTY as OWL_EQUIVALENT_PROPERTY, FUNCTIONAL_PROPERTY as OWL_FUNCTIONAL_PROPERTY,
    HAS_SELF as OWL_HAS_SELF, HAS_VALUE as OWL_HAS_VALUE, INTERSECTION_OF as OWL_INTERSECTION_OF,
    INVERSE_FUNCTIONAL_PROPERTY as OWL_INVERSE_FUNCTIONAL_PROPERTY, INVERSE_OF as OWL_INVERSE_OF,
    MAX_CARDINALITY as OWL_MAX_CARDINALITY,
    MAX_QUALIFIED_CARDINALITY as OWL_MAX_QUALIFIED_CARDINALITY,
    MIN_CARDINALITY as OWL_MIN_CARDINALITY,
    MIN_QUALIFIED_CARDINALITY as OWL_MIN_QUALIFIED_CARDINALITY,
    OBJECT_PROPERTY as OWL_OBJECT_PROPERTY, ON_CLASS as OWL_ON_CLASS,
    ON_DATA_RANGE as OWL_ON_DATA_RANGE, ON_DATATYPE as OWL_ON_DATATYPE,
    ON_PROPERTIES as OWL_ON_PROPERTIES, ON_PROPERTY as OWL_ON_PROPERTY, ONE_OF as OWL_ONE_OF,
    QUALIFIED_CARDINALITY as OWL_QUALIFIED_CARDINALITY, SOME_VALUES_FROM as OWL_SOME_VALUES_FROM,
    UNION_OF as OWL_UNION_OF, WITH_RESTRICTIONS as OWL_WITH_RESTRICTIONS,
};
use purrdf_iri::vocab::rdf::{LANG_STRING as RDF_LANG_STRING, PROPERTY as RDF_PROPERTY};
use purrdf_iri::vocab::rdfs::{
    DATATYPE as RDFS_DATATYPE, DOMAIN as RDFS_DOMAIN, LITERAL as RDFS_LITERAL,
    SUB_PROPERTY_OF as RDFS_SUB_PROPERTY_OF,
};
use purrdf_xsd::datatype::{
    XSD_BOOLEAN, XSD_LENGTH, XSD_MAX_EXCLUSIVE, XSD_MAX_INCLUSIVE, XSD_MAX_LENGTH,
    XSD_MIN_EXCLUSIVE, XSD_MIN_INCLUSIVE, XSD_MIN_LENGTH, XSD_PATTERN,
};

// ── Expression model ────────────────────────────────────────────────────────

/// The canonical rendering of an anonymous individual inside an expression.
/// A blank-node label is local to one parse, so it never enters a canonical
/// rendering: the expression would otherwise change with triple order.
const ANONYMOUS_INDIVIDUAL: &str = "[]";

/// An RDF term inside an expression — an `owl:oneOf` member, an `owl:hasValue`
/// filler or a facet value — ordered by its canonical rendering.
#[derive(Debug, Clone)]
pub(crate) struct ExpressionTerm {
    key: String,
    pub(crate) term: Term,
}

impl ExpressionTerm {
    fn new(term: Term) -> Self {
        let key = match &term {
            Term::BlankNode(_) => ANONYMOUS_INDIVIDUAL.to_owned(),
            _ => term.to_string(),
        };
        Self { key, term }
    }

    /// Whether the term is an anonymous individual (a blank node), which has
    /// no `@id` stable beyond one document.
    pub(crate) const fn is_anonymous(&self) -> bool {
        matches!(self.term, Term::BlankNode(_))
    }

    /// Whether the term is a named individual.
    pub(crate) const fn is_named(&self) -> bool {
        matches!(self.term, Term::NamedNode(_))
    }

    /// Whether the term is a literal.
    pub(crate) const fn is_literal(&self) -> bool {
        matches!(self.term, Term::Literal(_))
    }
}

impl PartialEq for ExpressionTerm {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
    }
}

impl Eq for ExpressionTerm {}

impl PartialOrd for ExpressionTerm {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ExpressionTerm {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.key.cmp(&other.key)
    }
}

/// An OWL object or data property expression: a named property, or the
/// inverse of one (`[ owl:inverseOf p ]`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum PropertyExpression {
    Named(String),
    Inverse(String),
}

impl PropertyExpression {
    fn iri(&self) -> &str {
        match self {
            Self::Named(iri) | Self::Inverse(iri) => iri,
        }
    }

    const fn is_inverse(&self) -> bool {
        matches!(self, Self::Inverse(_))
    }

    fn write_canonical(&self, out: &mut String) {
        match self {
            Self::Named(iri) => write_iri(out, iri),
            Self::Inverse(iri) => {
                out.push_str("inverse(");
                write_iri(out, iri);
                out.push(')');
            }
        }
    }

    fn canonical(&self) -> String {
        let mut out = String::new();
        self.write_canonical(&mut out);
        out
    }

    /// The provenance subject for an axiom whose subject is this expression:
    /// the bare IRI of a named property, the canonical rendering otherwise.
    fn provenance_subject(&self) -> String {
        match self {
            Self::Named(iri) => iri.clone(),
            Self::Inverse(_) => self.canonical(),
        }
    }
}

/// The property a restriction constrains: one property expression
/// (`owl:onProperty`), or several data properties (`owl:onProperties`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum RestrictedProperty {
    One(PropertyExpression),
    Many(Vec<String>),
}

impl RestrictedProperty {
    fn write_canonical(&self, out: &mut String) {
        match self {
            Self::One(property) => property.write_canonical(out),
            Self::Many(properties) => {
                out.push_str("properties(");
                for (index, iri) in properties.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    write_iri(out, iri);
                }
                out.push(')');
            }
        }
    }

    fn canonical(&self) -> String {
        let mut out = String::new();
        self.write_canonical(&mut out);
        out
    }

    fn iris(&self) -> Vec<&str> {
        match self {
            Self::One(property) => vec![property.iri()],
            Self::Many(properties) => properties.iter().map(String::as_str).collect(),
        }
    }

    /// The named property this restriction constrains directly, when it is
    /// one named property (not an inverse, not several).
    pub(crate) fn named(&self) -> Option<&str> {
        match self {
            Self::One(PropertyExpression::Named(iri)) => Some(iri),
            _ => None,
        }
    }
}

/// The constraint of one OWL property restriction.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Restriction {
    /// `owl:someValuesFrom`: some value is in the filler.
    SomeValues(OntologyExpression),
    /// `owl:allValuesFrom`: every value is in the filler.
    AllValues(OntologyExpression),
    /// `owl:hasValue`: the term is a value.
    HasValue(ExpressionTerm),
    /// `owl:hasSelf true`: the focus node is its own value.
    HasSelf,
    /// `owl:minCardinality` / `owl:minQualifiedCardinality`.
    Min(u64, Option<OntologyExpression>),
    /// `owl:maxCardinality` / `owl:maxQualifiedCardinality`.
    Max(u64, Option<OntologyExpression>),
    /// `owl:cardinality` / `owl:qualifiedCardinality`.
    Exact(u64, Option<OntologyExpression>),
}

impl Restriction {
    /// Whether every instance of a class carrying the restriction has at least
    /// one value of the property.
    pub(crate) const fn is_existential(&self) -> bool {
        match self {
            Self::SomeValues(_) | Self::HasValue(_) | Self::HasSelf => true,
            Self::Min(count, _) | Self::Exact(count, _) => *count > 0,
            Self::AllValues(_) | Self::Max(..) => false,
        }
    }

    fn fillers(&self) -> impl Iterator<Item = &OntologyExpression> {
        match self {
            Self::SomeValues(filler) | Self::AllValues(filler) => Some(filler),
            Self::Min(_, qualifier) | Self::Max(_, qualifier) | Self::Exact(_, qualifier) => {
                qualifier.as_ref()
            }
            Self::HasValue(_) | Self::HasSelf => None,
        }
        .into_iter()
    }

    fn write_canonical(&self, on: &RestrictedProperty, out: &mut String) {
        let (name, count, filler) = match self {
            Self::SomeValues(filler) => ("some", None, Some(filler)),
            Self::AllValues(filler) => ("all", None, Some(filler)),
            Self::HasValue(term) => {
                out.push_str("has_value(");
                on.write_canonical(out);
                out.push(',');
                out.push_str(&term.key);
                out.push(')');
                return;
            }
            Self::HasSelf => ("has_self", None, None),
            Self::Min(count, qualifier) => ("min", Some(*count), qualifier.as_ref()),
            Self::Max(count, qualifier) => ("max", Some(*count), qualifier.as_ref()),
            Self::Exact(count, qualifier) => ("exact", Some(*count), qualifier.as_ref()),
        };
        out.push_str(name);
        out.push('(');
        if let Some(count) = count {
            out.push_str(&count.to_string());
            out.push(',');
        }
        on.write_canonical(out);
        if let Some(filler) = filler {
            out.push(',');
            filler.write_canonical(out);
        }
        out.push(')');
    }
}

/// An OWL 2 class expression or data range, as the bounded schema theory reads
/// it (OWL 2 Mapping to RDF Graphs, Tables 12 and 13).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum OntologyExpression {
    Named(String),
    Union(Vec<Self>),
    Intersection(Vec<Self>),
    /// `owl:complementOf`.
    Complement(Box<Self>),
    /// `owl:oneOf`: named or anonymous individuals, or literals.
    OneOf(Vec<ExpressionTerm>),
    /// An `owl:Restriction`.
    Restriction(RestrictedProperty, Box<Restriction>),
    /// `owl:onDatatype` with `owl:withRestrictions` facet/value pairs.
    DatatypeRestriction(String, Vec<(String, ExpressionTerm)>),
    /// `owl:datatypeComplementOf`.
    DatatypeComplement(Box<Self>),
}

impl OntologyExpression {
    pub(crate) fn canonical(&self) -> String {
        let mut out = String::new();
        self.write_canonical(&mut out);
        out
    }

    fn write_canonical(&self, out: &mut String) {
        match self {
            Self::Named(iri) => write_iri(out, iri),
            Self::Union(members) => write_members(out, "union", members),
            Self::Intersection(members) => write_members(out, "intersection", members),
            Self::Complement(inner) => {
                out.push_str("complement(");
                inner.write_canonical(out);
                out.push(')');
            }
            Self::OneOf(members) => {
                out.push_str("one_of(");
                for (index, member) in members.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    out.push_str(&member.key);
                }
                out.push(')');
            }
            Self::Restriction(on, restriction) => restriction.write_canonical(on, out),
            Self::DatatypeRestriction(base, facets) => {
                out.push_str("datatype_restriction(");
                write_iri(out, base);
                for (facet, value) in facets {
                    out.push(',');
                    write_iri(out, facet);
                    out.push('=');
                    out.push_str(&value.key);
                }
                out.push(')');
            }
            Self::DatatypeComplement(inner) => {
                out.push_str("datatype_complement(");
                inner.write_canonical(out);
                out.push(')');
            }
        }
    }

    /// The provenance subject for an axiom whose subject is this expression:
    /// the bare IRI of a named class, the canonical rendering otherwise.
    fn provenance_subject(&self) -> String {
        match self {
            Self::Named(iri) => iri.clone(),
            _ => self.canonical(),
        }
    }

    /// The named classes and datatypes of the expression's boolean skeleton:
    /// its named members through unions, intersections and complements, and a
    /// datatype restriction's base. Restriction fillers and enumerated
    /// individuals are not members.
    fn named_members(&self, out: &mut BTreeSet<String>) {
        match self {
            Self::Named(iri) | Self::DatatypeRestriction(iri, _) => {
                out.insert(iri.clone());
            }
            Self::Union(members) | Self::Intersection(members) => {
                for member in members {
                    member.named_members(out);
                }
            }
            Self::Complement(inner) | Self::DatatypeComplement(inner) => inner.named_members(out),
            Self::OneOf(_) | Self::Restriction(..) => {}
        }
    }

    /// Every IRI the expression names in a position a class may hold,
    /// restriction fillers and qualifiers included. A datatype restriction's
    /// base and a datatype complement's operand are datatypes by construction,
    /// so neither is named here.
    fn mentioned_names(&self, out: &mut BTreeSet<String>) {
        match self {
            Self::Named(iri) => {
                out.insert(iri.clone());
            }
            Self::Union(members) | Self::Intersection(members) => {
                for member in members {
                    member.mentioned_names(out);
                }
            }
            Self::Complement(inner) => inner.mentioned_names(out),
            Self::Restriction(_, restriction) => {
                for filler in restriction.fillers() {
                    filler.mentioned_names(out);
                }
            }
            Self::OneOf(_) | Self::DatatypeRestriction(..) | Self::DatatypeComplement(_) => {}
        }
    }

    /// Visit every restriction the expression contains, at any depth.
    fn visit_restrictions<'s, E>(
        &'s self,
        visit: &mut impl FnMut(&'s RestrictedProperty, &'s Restriction) -> Result<(), E>,
    ) -> Result<(), E> {
        match self {
            Self::Named(_) | Self::OneOf(_) | Self::DatatypeRestriction(..) => Ok(()),
            Self::Union(members) | Self::Intersection(members) => {
                for member in members {
                    member.visit_restrictions(visit)?;
                }
                Ok(())
            }
            Self::Complement(inner) | Self::DatatypeComplement(inner) => {
                inner.visit_restrictions(visit)
            }
            Self::Restriction(on, restriction) => {
                visit(on, restriction)?;
                for filler in restriction.fillers() {
                    filler.visit_restrictions(visit)?;
                }
                Ok(())
            }
        }
    }

    /// Whether the boolean skeleton holds a construct only a class expression
    /// has: a restriction, a complement, or an enumeration of individuals.
    fn has_class_only_construct(&self) -> bool {
        match self {
            Self::Restriction(..) | Self::Complement(_) => true,
            Self::OneOf(members) => members.iter().any(|member| !member.is_literal()),
            Self::Union(members) | Self::Intersection(members) => {
                members.iter().any(Self::has_class_only_construct)
            }
            Self::Named(_) | Self::DatatypeRestriction(..) | Self::DatatypeComplement(_) => false,
        }
    }

    /// Whether the boolean skeleton holds a construct only a data range has: a
    /// datatype restriction, a datatype complement, or an enumeration of
    /// literals.
    fn has_data_only_construct(&self) -> bool {
        match self {
            Self::DatatypeRestriction(..) | Self::DatatypeComplement(_) => true,
            Self::OneOf(members) => members.iter().any(ExpressionTerm::is_literal),
            Self::Union(members) | Self::Intersection(members) => {
                members.iter().any(Self::has_data_only_construct)
            }
            Self::Complement(inner) => inner.has_data_only_construct(),
            Self::Named(_) | Self::Restriction(..) => false,
        }
    }

    /// Whether the expression is only named classes under unions and
    /// intersections — the fragment whose class membership the named
    /// hierarchy decides.
    fn is_named_skeleton(&self) -> bool {
        match self {
            Self::Named(_) => true,
            Self::Union(members) | Self::Intersection(members) => {
                members.iter().all(Self::is_named_skeleton)
            }
            _ => false,
        }
    }

    fn matches_class(&self, supertypes: &BTreeSet<String>, anonymous: &AnonymousSupers) -> bool {
        match self {
            Self::Named(iri) => supertypes.contains(iri),
            Self::Union(members) => {
                members
                    .iter()
                    .any(|member| member.matches_class(supertypes, anonymous))
                    || anonymous.entails_union(members)
            }
            Self::Intersection(members) => members
                .iter()
                .all(|member| member.matches_class(supertypes, anonymous)),
            _ => anonymous.entails(self),
        }
    }

    fn all_named_members_match(&self, predicate: &impl Fn(&str) -> bool) -> bool {
        match self {
            Self::Named(iri) | Self::DatatypeRestriction(iri, _) => predicate(iri),
            Self::Union(members) | Self::Intersection(members) => members
                .iter()
                .all(|member| member.all_named_members_match(predicate)),
            Self::Complement(inner) | Self::DatatypeComplement(inner) => {
                inner.all_named_members_match(predicate)
            }
            Self::OneOf(_) | Self::Restriction(..) => true,
        }
    }

    /// The conjuncts of the expression: its members through nested
    /// intersections, the expression itself otherwise.
    fn conjuncts(&self) -> Vec<&Self> {
        let mut out = Vec::new();
        let mut stack = vec![self];
        while let Some(expression) = stack.pop() {
            if let Self::Intersection(members) = expression {
                stack.extend(members.iter().rev());
            } else {
                out.push(expression);
            }
        }
        out
    }
}

fn write_iri(out: &mut String, iri: &str) {
    out.push('<');
    out.push_str(iri);
    out.push('>');
}

fn write_members(out: &mut String, name: &str, members: &[OntologyExpression]) {
    out.push_str(name);
    out.push('(');
    for (index, member) in members.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        member.write_canonical(out);
    }
    out.push(')');
}

/// How exactly a developer schema states membership of one value in an
/// expression used as a filler or range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ValuePrecision {
    /// The value schema admits exactly the expression's values.
    Exact,
    /// A named class: the value is an open node reference, as an
    /// `rdfs:range` class projects.
    ClassLike,
    /// The value schema admits more than the expression's values.
    Approximate,
}

/// The precision of the value schema an expression projects as.
pub(crate) fn value_precision(
    expression: &OntologyExpression,
    datatypes: &BTreeSet<String>,
) -> ValuePrecision {
    match expression {
        OntologyExpression::Named(iri) => {
            if is_datatype(iri, datatypes) {
                ValuePrecision::Exact
            } else {
                ValuePrecision::ClassLike
            }
        }
        OntologyExpression::Union(members) | OntologyExpression::Intersection(members) => members
            .iter()
            .map(|member| value_precision(member, datatypes))
            .max()
            .unwrap_or(ValuePrecision::Exact),
        OntologyExpression::OneOf(members) => {
            if members.iter().all(ExpressionTerm::is_literal) {
                ValuePrecision::Exact
            } else {
                ValuePrecision::Approximate
            }
        }
        OntologyExpression::DatatypeRestriction(base, facets) => {
            if is_datatype(base, datatypes)
                && facets
                    .iter()
                    .all(|(facet, value)| facet_supported(base, facet, &value.term))
            {
                ValuePrecision::Exact
            } else {
                ValuePrecision::Approximate
            }
        }
        OntologyExpression::DatatypeComplement(inner) => {
            if value_precision(inner, datatypes) == ValuePrecision::Exact {
                ValuePrecision::Exact
            } else {
                ValuePrecision::Approximate
            }
        }
        OntologyExpression::Complement(_) | OntologyExpression::Restriction(..) => {
            ValuePrecision::Approximate
        }
    }
}

/// Whether a filler's value schema admits exactly its values, so that it can
/// be counted (a qualified maximum) or negated soundly.
pub(crate) fn projects_exactly(
    expression: &OntologyExpression,
    datatypes: &BTreeSet<String>,
) -> bool {
    value_precision(expression, datatypes) == ValuePrecision::Exact
}

pub(crate) fn is_datatype(iri: &str, datatypes: &BTreeSet<String>) -> bool {
    is_builtin_datatype(iri) || datatypes.contains(iri)
}

/// Whether one `owl:withRestrictions` facet on `base` projects exactly through
/// the shared SHACL value-constraint compiler: a numeric bound on an integer or
/// decimal datatype, a temporal bound on its own temporal datatype, a length on
/// `xsd:string`, or a pattern on a datatype whose values project as strings.
pub(crate) fn facet_supported(base: &str, facet: &str, value: &Term) -> bool {
    let Term::Literal(literal) = value else {
        return false;
    };
    let base_type = XsdDatatype::from_iri(base);
    let value_type = XsdDatatype::from_iri(literal.datatype_str());
    match facet {
        XSD_MIN_INCLUSIVE | XSD_MAX_INCLUSIVE | XSD_MIN_EXCLUSIVE | XSD_MAX_EXCLUSIVE => {
            let exact_number = |datatype: Option<XsdDatatype>| {
                datatype.is_some_and(|datatype| {
                    datatype.is_integer_family() || datatype == XsdDatatype::Decimal
                })
            };
            let temporal = matches!(
                base_type,
                Some(XsdDatatype::Date | XsdDatatype::Time | XsdDatatype::DateTime)
            );
            (exact_number(base_type) && exact_number(value_type))
                || (temporal && value_type == base_type)
        }
        XSD_LENGTH | XSD_MIN_LENGTH | XSD_MAX_LENGTH => {
            base_type == Some(XsdDatatype::String)
                && value_type.is_some_and(XsdDatatype::is_integer_family)
                && non_negative_integer(literal.value()).is_some()
        }
        XSD_PATTERN => {
            base_type.is_none_or(|datatype| {
                !datatype.is_integer_family() && datatype != XsdDatatype::Boolean
            }) && purrdf_core::xsd_regex::to_ecma_262(&xsd_pattern_as_xpath(literal.value()), "")
                .is_ok()
        }
        _ => false,
    }
}

/// An XSD `pattern` facet (XSD 1.1 Part 2 §F, implicitly anchored, with `^`
/// and `$` ordinary characters) as the equivalent XPath regular expression
/// that SHACL's `sh:pattern` searches with: the two characters escaped outside
/// character classes, and the whole anchored.
pub(crate) fn xsd_pattern_as_xpath(pattern: &str) -> String {
    let mut out = String::with_capacity(pattern.len() + 6);
    out.push_str("^(");
    let mut depth = 0_usize;
    let mut characters = pattern.chars();
    while let Some(character) = characters.next() {
        match character {
            '\\' => {
                out.push(character);
                if let Some(escaped) = characters.next() {
                    out.push(escaped);
                }
            }
            '[' => {
                depth += 1;
                out.push(character);
            }
            ']' if depth > 0 => {
                depth -= 1;
                out.push(character);
            }
            '^' | '$' if depth == 0 => {
                out.push('\\');
                out.push(character);
            }
            _ => out.push(character),
        }
    }
    out.push_str(")$");
    out
}

/// A non-negative integer lexical form (`[+]?[0-9]+`, or a signed zero), read
/// into a `u64`.
pub(crate) fn non_negative_integer(lexical: &str) -> Option<u64> {
    let (negative, digits) = match lexical.as_bytes().first() {
        Some(b'+') => (false, &lexical[1..]),
        Some(b'-') => (true, &lexical[1..]),
        _ => (false, lexical),
    };
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    if negative {
        return digits.bytes().all(|byte| byte == b'0').then_some(0);
    }
    digits.bytes().try_fold(0_u64, |value, byte| {
        value
            .checked_mul(10)
            .and_then(|value| value.checked_add(u64::from(byte - b'0')))
    })
}

/// The anonymous class expressions a class is known to be a subclass of, read
/// structurally when a domain is not a named class.
#[derive(Debug, Default)]
struct AnonymousSupers {
    canonical: BTreeSet<String>,
    unions: Vec<BTreeSet<String>>,
}

impl AnonymousSupers {
    fn entails(&self, expression: &OntologyExpression) -> bool {
        !self.canonical.is_empty() && self.canonical.contains(&expression.canonical())
    }

    /// Whether the class is a subclass of a union whose members are among
    /// `members` — and so of the union of `members`.
    fn entails_union(&self, members: &[OntologyExpression]) -> bool {
        if self.unions.is_empty() {
            return false;
        }
        let members: BTreeSet<String> = members.iter().map(OntologyExpression::canonical).collect();
        self.unions.iter().any(|union| union.is_subset(&members))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OntologyPropertyKind {
    Generic,
    Object,
    Datatype,
    Annotation,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SourcedExpression {
    expression: OntologyExpression,
    provenance: SchemaCoverageProvenance,
}

#[derive(Debug, Clone, Default)]
struct PropertyFacts {
    declarations: BTreeSet<String>,
    provenance: BTreeSet<SchemaCoverageProvenance>,
    object_property: bool,
    datatype_property: bool,
    annotation_property: bool,
    domains: BTreeSet<SourcedExpression>,
    ranges: BTreeSet<SourcedExpression>,
    functional: BTreeSet<SchemaCoverageProvenance>,
}

impl PropertyFacts {
    fn kind(&self, property_iri: &str) -> Result<OntologyPropertyKind, SchemaCompileError> {
        let specialized = usize::from(self.object_property)
            + usize::from(self.datatype_property)
            + usize::from(self.annotation_property);
        if specialized > 1 {
            return Err(SchemaCompileError::InvalidOntology {
                subject: property_iri.to_owned(),
                reason: "property has incompatible object, datatype, or annotation declarations"
                    .to_owned(),
            });
        }
        Ok(if self.object_property {
            OntologyPropertyKind::Object
        } else if self.datatype_property {
            OntologyPropertyKind::Datatype
        } else if self.annotation_property {
            OntologyPropertyKind::Annotation
        } else {
            OntologyPropertyKind::Generic
        })
    }
}

/// One unshaped property admitted into a class definition.
#[derive(Debug, Clone)]
pub(crate) struct SurfaceProperty {
    pub(crate) iri: String,
    pub(crate) kind: OntologyPropertyKind,
    pub(crate) ranges: Vec<OntologyExpression>,
    pub(crate) datatype_iris: BTreeSet<String>,
    pub(crate) functional: bool,
    pub(crate) provenance: Vec<SchemaCoverageProvenance>,
    /// The OWL restrictions on this property that the class inherits from its
    /// anonymous superclass expressions, canonically ordered. `owl:hasSelf` is
    /// never here: no schema keyword at the value location states it.
    pub(crate) restrictions: Vec<Restriction>,
}

/// One existing named class represented by a schema `$def`.
#[derive(Debug, Clone, Default)]
pub(crate) struct SurfaceClass {
    pub(crate) synthesized_open: bool,
    pub(crate) properties: BTreeMap<String, SurfaceProperty>,
    /// Class-level anonymous superclass expressions projected on the focus
    /// node itself: an enumeration of named individuals, the complement of a
    /// named class, or a disjunction of property restrictions.
    pub(crate) focus: Vec<OntologyExpression>,
    /// Each anonymous superclass expression component the class's schema
    /// cannot represent, rendered `expression: reason`, sorted.
    pub(crate) unrepresented: Vec<String>,
}

/// Single source of truth for ontology-aware schema definitions and coverage.
#[derive(Debug, Clone)]
pub(crate) struct SchemaSurface {
    pub(crate) classes: BTreeMap<String, SurfaceClass>,
    pub(crate) report: SchemaCoverageReport,
    pub(crate) class_expressions: SchemaClassExpressionReport,
    /// Every IRI declared `rdfs:Datatype` or `owl:DataRange`, or defined as a
    /// datatype by an equivalence with a data range.
    pub(crate) datatypes: BTreeSet<String>,
    /// Each defined datatype's defining data range.
    pub(crate) datatype_definitions: BTreeMap<String, OntologyExpression>,
}

impl SchemaSurface {
    fn assert_conservation(&self) {
        let report_properties: BTreeSet<&str> = self
            .report
            .properties
            .iter()
            .map(|property| property.property_iri.as_str())
            .collect();
        debug_assert_eq!(report_properties.len(), self.report.properties.len());
        let mut emitted = 0_usize;
        let mut range_expressions = 0_usize;
        let mut provenance_records = 0_usize;
        for class in self.classes.values() {
            for (property_iri, property) in &class.properties {
                debug_assert_eq!(property_iri, &property.iri);
                debug_assert!(report_properties.contains(property_iri.as_str()));
                match property.kind {
                    OntologyPropertyKind::Generic
                    | OntologyPropertyKind::Object
                    | OntologyPropertyKind::Datatype
                    | OntologyPropertyKind::Annotation => {}
                }
                if property.functional {
                    debug_assert_ne!(property.provenance, [] as [_; 0]);
                }
                debug_assert!(
                    !property
                        .restrictions
                        .iter()
                        .any(|restriction| matches!(restriction, Restriction::HasSelf))
                );
                emitted += 1;
                range_expressions += property.ranges.len();
                provenance_records += property.provenance.len();
            }
        }
        debug_assert!(emitted <= MAX_SCHEMA_RELATIONS);
        debug_assert!(range_expressions <= MAX_SCHEMA_RELATIONS);
        debug_assert!(provenance_records <= MAX_SCHEMA_RELATIONS * 8);
        // Never silently dropped: every anonymous class axiom carries at least
        // one component, on a class or on the axiom itself.
        for axiom in &self.class_expressions.axioms {
            debug_assert!(
                !axiom.components.is_empty()
                    || axiom
                        .classes
                        .iter()
                        .any(|class| !class.components.is_empty())
            );
        }
    }
}

#[derive(Debug, Clone, Default)]
struct ShapeClassInfo {
    direct_properties: BTreeSet<String>,
    closed_surfaces: Vec<ClosedSurface>,
}

#[derive(Debug, Clone, Default)]
struct ClosedSurface {
    direct_properties: BTreeSet<String>,
    ignored_properties: BTreeSet<String>,
}

impl ShapeClassInfo {
    fn closed_allows(&self, property: &str) -> bool {
        self.closed_surfaces.iter().all(|closed| {
            closed.direct_properties.contains(property)
                || closed.ignored_properties.contains(property)
        })
    }
}

#[derive(Debug, Clone)]
struct TripleRow {
    subject: Term,
    subject_key: String,
    predicate: String,
    object: Term,
    object_key: String,
}

#[derive(Debug, Clone, Copy)]
enum PropertyRelationKind {
    SubProperty,
    Equivalent,
}

/// One property axiom between two property expressions. `owl:inverseOf` is
/// stored as an equivalence with its right side inverted.
#[derive(Debug, Clone)]
struct PropertyRelation {
    left: PropertyExpression,
    right: PropertyExpression,
    kind: PropertyRelationKind,
}

// ── Anonymous class axioms ──────────────────────────────────────────────────

const HIERARCHY_REASON: &str =
    "the named superclass joins the class hierarchy, which drives domain membership";
const SHAPED_ONLY_REASON: &str =
    "shaped-only mode projects no OWL class axioms; only active SHACL target classes are emitted";
const GENERAL_INCLUSION_REASON: &str = "a general class inclusion whose subclass expression is \
     anonymous classifies instances that satisfy the expression; no named class carries it, so no \
     developer schema constraint is projected";
const SUFFICIENT_REASON: &str = "the sufficient-condition direction of owl:equivalentClass \
     classifies instances that satisfy the expression; it constrains no named class, so no \
     developer schema constraint is projected";
const DATATYPE_DEFINITION_REASON: &str = "a datatype definition: a value of the defined datatype \
     is accepted when it is typed with the datatype by name, or when it meets the defining data \
     range";
const UNCARRIED_REASON: &str =
    "no caller-owned class carries this axiom, so no developer schema represents it";

/// One `rdfs:subClassOf` or `owl:equivalentClass` axiom with an anonymous side.
#[derive(Debug, Clone)]
struct ClassAxiom {
    provenance: SchemaCoverageProvenance,
    /// The named classes the axiom constrains, each with the conjuncts it
    /// asserts of them (a named conjunct is a hierarchy edge).
    carriers: Vec<(String, Vec<OntologyExpression>)>,
    /// Components of the axiom that no named class carries.
    uncarried: Vec<SchemaExpressionComponent>,
}

impl ClassAxiom {
    fn classify(
        subject: &OntologyExpression,
        object: &OntologyExpression,
        equivalent: bool,
        provenance: SchemaCoverageProvenance,
    ) -> Self {
        let mut axiom = Self {
            provenance,
            carriers: Vec::new(),
            uncarried: Vec::new(),
        };
        if equivalent {
            axiom.include(subject, object, SUFFICIENT_REASON);
            axiom.include(object, subject, SUFFICIENT_REASON);
        } else {
            axiom.include(subject, object, GENERAL_INCLUSION_REASON);
        }
        axiom.carriers.sort();
        axiom.carriers.dedup();
        axiom.uncarried.sort();
        axiom.uncarried.dedup();
        axiom
    }

    /// Record `sub ⊑ sup`. A named class carries the conjuncts of `sup`; so
    /// does each named member of a union `sub` (each member is a subclass of
    /// the union). Any other subclass expression classifies rather than
    /// constrains, and is reported as such.
    fn include(
        &mut self,
        sub: &OntologyExpression,
        sup: &OntologyExpression,
        uncarried_reason: &'static str,
    ) {
        let conjuncts: Vec<OntologyExpression> = sup.conjuncts().into_iter().cloned().collect();
        let mut stack = vec![sub];
        while let Some(expression) = stack.pop() {
            match expression {
                OntologyExpression::Named(iri) => {
                    self.carriers.push((iri.clone(), conjuncts.clone()));
                }
                OntologyExpression::Union(members) => stack.extend(members.iter().rev()),
                other => self.uncarried.push(SchemaExpressionComponent {
                    expression: other.canonical(),
                    property_iri: None,
                    outcome: SchemaExpressionOutcome::Unrepresented,
                    reason: uncarried_reason.to_owned(),
                }),
            }
        }
    }

    /// The named subclass edges `(child, parent)` this axiom asserts.
    fn edges(&self) -> impl Iterator<Item = (&str, &str)> {
        self.carriers.iter().flat_map(|(carrier, conjuncts)| {
            conjuncts.iter().filter_map(move |conjunct| match conjunct {
                OntologyExpression::Named(parent) if parent != carrier => {
                    Some((carrier.as_str(), parent.as_str()))
                }
                _ => None,
            })
        })
    }
}

// ── Expression reader ───────────────────────────────────────────────────────

/// The restriction facets of OWL 2 Mapping to RDF Graphs Table 13.
#[derive(Debug, Clone, Copy)]
enum FacetKind {
    Some,
    All,
    HasValue,
    HasSelf,
    Min,
    Max,
    Exact,
    QualifiedMin,
    QualifiedMax,
    QualifiedExact,
}

const RESTRICTION_FACETS: [(&str, FacetKind); 10] = [
    (OWL_SOME_VALUES_FROM, FacetKind::Some),
    (OWL_ALL_VALUES_FROM, FacetKind::All),
    (OWL_HAS_VALUE, FacetKind::HasValue),
    (OWL_HAS_SELF, FacetKind::HasSelf),
    (OWL_MIN_CARDINALITY, FacetKind::Min),
    (OWL_MAX_CARDINALITY, FacetKind::Max),
    (OWL_CARDINALITY, FacetKind::Exact),
    (OWL_MIN_QUALIFIED_CARDINALITY, FacetKind::QualifiedMin),
    (OWL_MAX_QUALIFIED_CARDINALITY, FacetKind::QualifiedMax),
    (OWL_QUALIFIED_CARDINALITY, FacetKind::QualifiedExact),
];

/// Reads class expressions, data ranges and property expressions out of the
/// ontology ∪ shapes dataset, under one expansion budget per request.
struct ExpressionReader<'a> {
    dataset: &'a RdfDataset,
    /// Expression nodes expanded so far. A blank node shared by several
    /// expressions is expanded once per occurrence, so the budget bounds the
    /// expanded size, not only the graph size.
    nodes: usize,
    /// The most expression nodes one request may expand.
    budget: usize,
    /// The blank nodes on the path from the expression root being read: one
    /// seen again on its own path is an expression containing itself.
    path: Vec<String>,
}

impl<'a> ExpressionReader<'a> {
    const fn new(dataset: &'a RdfDataset) -> Self {
        Self::with_budget(dataset, MAX_SCHEMA_RELATIONS)
    }

    const fn with_budget(dataset: &'a RdfDataset, budget: usize) -> Self {
        Self {
            dataset,
            nodes: 0,
            budget,
            path: Vec::new(),
        }
    }

    fn expression(
        &mut self,
        term: &Term,
        depth: usize,
    ) -> Result<OntologyExpression, SchemaCompileError> {
        if depth > MAX_OWL_EXPRESSION_DEPTH {
            return Err(SchemaCompileError::LimitExceeded {
                resource: "OWL expression depth",
                limit: MAX_OWL_EXPRESSION_DEPTH,
                observed: depth,
            });
        }
        self.count_nodes(1)?;
        match term {
            Term::NamedNode(node) => Ok(OntologyExpression::Named(node.as_str().to_owned())),
            Term::BlankNode(_) => {
                let key = term.to_string();
                if self.path.contains(&key) {
                    return Err(SchemaCompileError::InvalidOntology {
                        subject: key,
                        reason: "anonymous OWL expression contains itself; an expression must be \
                                 a finite tree"
                            .to_owned(),
                    });
                }
                self.path.push(key.clone());
                let parsed = self.anonymous(term, &key, depth);
                self.path.pop();
                parsed
            }
            _ => Err(SchemaCompileError::InvalidOntology {
                subject: term.to_string(),
                reason: "an OWL class expression or data range must be an IRI or a blank node"
                    .to_owned(),
            }),
        }
    }

    fn count_nodes(&mut self, count: usize) -> Result<(), SchemaCompileError> {
        self.nodes = self.nodes.saturating_add(count);
        enforce_limit("OWL expression nodes", self.nodes, self.budget)
    }

    #[allow(
        clippy::too_many_lines,
        reason = "one dispatch over the disjoint RDF encodings of OWL 2 Mapping Tables 12 and 13"
    )]
    fn anonymous(
        &mut self,
        term: &Term,
        key: &str,
        depth: usize,
    ) -> Result<OntologyExpression, SchemaCompileError> {
        let mut fields: BTreeMap<String, Vec<Term>> = BTreeMap::new();
        for (_, predicate, object) in
            native_quads(self.dataset, Some(term), None, None, GraphFilter::AnyGraph)
        {
            fields
                .entry(predicate.into_string())
                .or_default()
                .push(object);
        }
        let take = |name: &str| fields.get(name).map_or(&[][..], Vec::as_slice);
        let malformed = |reason: String| SchemaCompileError::InvalidOntology {
            subject: key.to_owned(),
            reason,
        };
        let restriction = [
            OWL_ON_PROPERTY,
            OWL_ON_PROPERTIES,
            OWL_ON_CLASS,
            OWL_ON_DATA_RANGE,
        ]
        .into_iter()
        .chain(RESTRICTION_FACETS.iter().map(|(facet, _)| *facet))
        .any(|predicate| !take(predicate).is_empty());
        let forms = [
            (!take(OWL_UNION_OF).is_empty(), "owl:unionOf"),
            (!take(OWL_INTERSECTION_OF).is_empty(), "owl:intersectionOf"),
            (!take(OWL_COMPLEMENT_OF).is_empty(), "owl:complementOf"),
            (!take(OWL_ONE_OF).is_empty(), "owl:oneOf"),
            (restriction, "an owl:Restriction"),
            (
                !take(OWL_ON_DATATYPE).is_empty() || !take(OWL_WITH_RESTRICTIONS).is_empty(),
                "a datatype restriction",
            ),
            (
                !take(OWL_DATATYPE_COMPLEMENT_OF).is_empty(),
                "owl:datatypeComplementOf",
            ),
        ];
        let declared: Vec<&str> = forms
            .iter()
            .filter(|(present, _)| *present)
            .map(|(_, name)| *name)
            .collect();
        match declared.as_slice() {
            [] => {
                return Err(malformed(
                    "anonymous OWL expression declares no class-expression or data-range \
                     construct (owl:unionOf, owl:intersectionOf, owl:complementOf, owl:oneOf, an \
                     owl:Restriction, owl:onDatatype or owl:datatypeComplementOf)"
                        .to_owned(),
                ));
            }
            [_] => {}
            several => {
                return Err(malformed(format!(
                    "anonymous OWL expression mixes {}; one blank node encodes exactly one \
                     construct",
                    several.join(" and ")
                )));
            }
        }

        if !take(OWL_UNION_OF).is_empty() {
            let head = single(take(OWL_UNION_OF), OWL_UNION_OF, key)?;
            return Ok(OntologyExpression::Union(self.boolean_members(
                head,
                depth + 1,
                key,
            )?));
        }
        if !take(OWL_INTERSECTION_OF).is_empty() {
            let head = single(take(OWL_INTERSECTION_OF), OWL_INTERSECTION_OF, key)?;
            return Ok(OntologyExpression::Intersection(self.boolean_members(
                head,
                depth + 1,
                key,
            )?));
        }
        if !take(OWL_COMPLEMENT_OF).is_empty() {
            let inner = single(take(OWL_COMPLEMENT_OF), OWL_COMPLEMENT_OF, key)?;
            return Ok(OntologyExpression::Complement(Box::new(
                self.expression(inner, depth + 1)?,
            )));
        }
        if !take(OWL_DATATYPE_COMPLEMENT_OF).is_empty() {
            let inner = single(
                take(OWL_DATATYPE_COMPLEMENT_OF),
                OWL_DATATYPE_COMPLEMENT_OF,
                key,
            )?;
            return Ok(OntologyExpression::DatatypeComplement(Box::new(
                self.expression(inner, depth + 1)?,
            )));
        }
        if !take(OWL_ONE_OF).is_empty() {
            let head = single(take(OWL_ONE_OF), OWL_ONE_OF, key)?;
            return self.one_of(head, key);
        }
        if !restriction {
            let base = match single(take(OWL_ON_DATATYPE), OWL_ON_DATATYPE, key)? {
                Term::NamedNode(node) => node.as_str().to_owned(),
                other => {
                    return Err(malformed(format!(
                        "owl:onDatatype must name a datatype IRI; found {other}"
                    )));
                }
            };
            let head = single(take(OWL_WITH_RESTRICTIONS), OWL_WITH_RESTRICTIONS, key)?;
            return self.datatype_restriction(base, head, key);
        }

        let property = match (take(OWL_ON_PROPERTY), take(OWL_ON_PROPERTIES)) {
            ([one], []) => RestrictedProperty::One(self.property_expression(one)?),
            ([], [head]) => {
                let mut properties = Vec::new();
                for item in self.list_items(head, key)? {
                    match item {
                        Term::NamedNode(node) => properties.push(node.into_string()),
                        other => {
                            return Err(malformed(format!(
                                "owl:onProperties members must be data property IRIs; found \
                                 {other}"
                            )));
                        }
                    }
                }
                if properties.is_empty() {
                    return Err(malformed(
                        "owl:onProperties requires at least one property".to_owned(),
                    ));
                }
                properties.sort();
                properties.dedup();
                RestrictedProperty::Many(properties)
            }
            ([], []) => {
                return Err(malformed(
                    "owl:Restriction declares no owl:onProperty; a restriction must name the \
                     property it restricts"
                        .to_owned(),
                ));
            }
            _ => {
                return Err(malformed(
                    "owl:Restriction must declare exactly one owl:onProperty or owl:onProperties \
                     value"
                        .to_owned(),
                ));
            }
        };
        let qualifier = match (take(OWL_ON_CLASS), take(OWL_ON_DATA_RANGE)) {
            ([], []) => None,
            ([one], []) | ([], [one]) => Some(self.expression(one, depth + 1)?),
            _ => {
                return Err(malformed(
                    "owl:Restriction must declare at most one owl:onClass or owl:onDataRange \
                     qualifier"
                        .to_owned(),
                ));
            }
        };
        let mut restrictions = Vec::new();
        let mut qualified = false;
        for (facet, kind) in RESTRICTION_FACETS {
            let value = match take(facet) {
                [] => continue,
                [one] => one,
                _ => {
                    return Err(malformed(format!(
                        "owl:Restriction declares conflicting values for <{facet}>"
                    )));
                }
            };
            let mut qualify = || {
                qualified = true;
                qualifier.clone().ok_or_else(|| {
                    malformed(format!(
                        "<{facet}> requires an owl:onClass or owl:onDataRange qualifier"
                    ))
                })
            };
            restrictions.push(match kind {
                FacetKind::Some => Restriction::SomeValues(self.expression(value, depth + 1)?),
                FacetKind::All => Restriction::AllValues(self.expression(value, depth + 1)?),
                FacetKind::HasValue => Restriction::HasValue(ExpressionTerm::new(value.clone())),
                FacetKind::HasSelf => {
                    has_self_value(value, key)?;
                    Restriction::HasSelf
                }
                FacetKind::Min => Restriction::Min(cardinality(value, facet, key)?, None),
                FacetKind::Max => Restriction::Max(cardinality(value, facet, key)?, None),
                FacetKind::Exact => Restriction::Exact(cardinality(value, facet, key)?, None),
                FacetKind::QualifiedMin => {
                    Restriction::Min(cardinality(value, facet, key)?, Some(qualify()?))
                }
                FacetKind::QualifiedMax => {
                    Restriction::Max(cardinality(value, facet, key)?, Some(qualify()?))
                }
                FacetKind::QualifiedExact => {
                    Restriction::Exact(cardinality(value, facet, key)?, Some(qualify()?))
                }
            });
        }
        if qualifier.is_some() && !qualified {
            return Err(malformed(
                "owl:onClass/owl:onDataRange qualifies no qualified cardinality".to_owned(),
            ));
        }
        if restrictions.is_empty() {
            return Err(malformed(format!(
                "owl:Restriction on {} declares no constraint (owl:someValuesFrom, \
                 owl:allValuesFrom, owl:hasValue, owl:hasSelf or a cardinality)",
                property.canonical()
            )));
        }
        if matches!(property, RestrictedProperty::Many(_))
            && restrictions.iter().any(|restriction| {
                !matches!(
                    restriction,
                    Restriction::SomeValues(_) | Restriction::AllValues(_)
                )
            })
        {
            return Err(malformed(
                "owl:onProperties is defined only with owl:someValuesFrom or owl:allValuesFrom"
                    .to_owned(),
            ));
        }
        let mut members: Vec<OntologyExpression> = restrictions
            .into_iter()
            .map(|restriction| {
                OntologyExpression::Restriction(property.clone(), Box::new(restriction))
            })
            .collect();
        if members.len() == 1 {
            Ok(members.pop().expect("one restriction"))
        } else {
            // An OWL 1 restriction node carrying several facets on one property
            // (for example both owl:minCardinality and owl:maxCardinality) is
            // the conjunction of one restriction per facet.
            members.sort();
            Ok(OntologyExpression::Intersection(members))
        }
    }

    /// Read a property expression: an IRI, or a blank node that is
    /// `owl:inverseOf` exactly one named property.
    fn property_expression(&self, term: &Term) -> Result<PropertyExpression, SchemaCompileError> {
        match term {
            Term::NamedNode(node) => Ok(PropertyExpression::Named(node.as_str().to_owned())),
            Term::BlankNode(_) => match objects_of(self.dataset, term, OWL_INVERSE_OF).as_slice() {
                [Term::NamedNode(node)] => {
                    Ok(PropertyExpression::Inverse(node.as_str().to_owned()))
                }
                _ => Err(SchemaCompileError::InvalidOntology {
                    subject: term.to_string(),
                    reason: "an anonymous property expression must be owl:inverseOf exactly one \
                             named property"
                        .to_owned(),
                }),
            },
            _ => Err(SchemaCompileError::InvalidOntology {
                subject: term.to_string(),
                reason: "a property expression must be an IRI or an owl:inverseOf blank node"
                    .to_owned(),
            }),
        }
    }

    /// The items of an RDF list, read by the strict walker
    /// ([`DatasetView::rdf_list_strict`](::purrdf_rdf::DatasetView::rdf_list_strict)):
    /// OWL 2 Mapping to RDF Graphs §3.1 reads a sequence only from a
    /// well-formed collection, so a malformed or cyclic one is refused rather
    /// than read short.
    fn list_items(&mut self, head: &Term, owner: &str) -> Result<Vec<Term>, SchemaCompileError> {
        let malformed = |reason: String| SchemaCompileError::InvalidOntology {
            subject: owner.to_owned(),
            reason,
        };
        let items = match crate::data::resolve_id(self.dataset, head) {
            Some(head_id) => ::purrdf_rdf::DatasetView::rdf_list_strict(
                self.dataset,
                head_id,
                ::purrdf_rdf::GraphMatch::Any,
            )
            .map_err(|error| malformed(format!("OWL expression list at {head}: {error}")))?,
            None => {
                return Err(malformed(format!(
                    "OWL expression list must be an RDF list; found {head}"
                )));
            }
        };
        enforce_limit(
            "OWL expression list members",
            items.len(),
            MAX_SCHEMA_CLASSES,
        )?;
        self.count_nodes(items.len())?;
        Ok(items
            .into_iter()
            .map(|item| crate::term::term_id_to_native(self.dataset, item))
            .collect())
    }

    /// The members of an `owl:unionOf`/`owl:intersectionOf` list, each read as
    /// an expression.
    fn boolean_members(
        &mut self,
        head: &Term,
        depth: usize,
        owner: &str,
    ) -> Result<Vec<OntologyExpression>, SchemaCompileError> {
        let items = self.list_items(head, owner)?;
        let mut members = Vec::with_capacity(items.len());
        for item in &items {
            members.push(self.expression(item, depth)?);
        }
        if members.len() < 2 {
            return Err(SchemaCompileError::InvalidOntology {
                subject: owner.to_owned(),
                reason: "owl:unionOf/owl:intersectionOf requires at least two members".to_owned(),
            });
        }
        members.sort();
        members.dedup();
        if members.len() < 2 {
            return Err(SchemaCompileError::InvalidOntology {
                subject: owner.to_owned(),
                reason: "owl:unionOf/owl:intersectionOf must contain two distinct members"
                    .to_owned(),
            });
        }
        Ok(members)
    }

    fn one_of(
        &mut self,
        head: &Term,
        owner: &str,
    ) -> Result<OntologyExpression, SchemaCompileError> {
        let malformed = |reason: String| SchemaCompileError::InvalidOntology {
            subject: owner.to_owned(),
            reason,
        };
        let items = self.list_items(head, owner)?;
        if items.is_empty() {
            return Err(malformed(
                "owl:oneOf requires at least one member".to_owned(),
            ));
        }
        let mut members = Vec::with_capacity(items.len());
        for item in items {
            if matches!(item, Term::Triple(_)) {
                return Err(malformed(format!(
                    "owl:oneOf member {item} is a triple term, neither an individual nor a literal"
                )));
            }
            members.push(ExpressionTerm::new(item));
        }
        if members.iter().any(ExpressionTerm::is_literal)
            && !members.iter().all(ExpressionTerm::is_literal)
        {
            return Err(malformed(
                "owl:oneOf mixes individuals and literals; an enumeration is of one kind"
                    .to_owned(),
            ));
        }
        members.sort();
        members.dedup();
        Ok(OntologyExpression::OneOf(members))
    }

    fn datatype_restriction(
        &mut self,
        base: String,
        head: &Term,
        owner: &str,
    ) -> Result<OntologyExpression, SchemaCompileError> {
        let malformed = |reason: String| SchemaCompileError::InvalidOntology {
            subject: owner.to_owned(),
            reason,
        };
        let items = self.list_items(head, owner)?;
        if items.is_empty() {
            return Err(malformed(
                "owl:withRestrictions requires at least one facet restriction".to_owned(),
            ));
        }
        let mut facets = Vec::with_capacity(items.len());
        for item in items {
            if !matches!(item, Term::BlankNode(_)) {
                return Err(malformed(format!(
                    "facet restriction {item} must be a blank node"
                )));
            }
            let quads = native_quads(self.dataset, Some(&item), None, None, GraphFilter::AnyGraph);
            match quads.as_slice() {
                [(_, facet, value @ Term::Literal(literal))] => {
                    // A pattern outside the XSD regular-expression language is
                    // a broken facet, refused as `sh:pattern` is; one that is
                    // valid but has no ECMA-262 translation is reported.
                    if facet.as_str() == XSD_PATTERN {
                        use purrdf_core::xsd_regex::Ecma262Error;
                        if let Err(error @ (Ecma262Error::Source(_) | Ecma262Error::Syntax(_))) =
                            purrdf_core::xsd_regex::to_ecma_262(
                                &xsd_pattern_as_xpath(literal.value()),
                                "",
                            )
                        {
                            return Err(malformed(format!(
                                "xsd:pattern facet {value} is not an XSD regular expression \
                                 ({error})"
                            )));
                        }
                    }
                    facets.push((
                        facet.as_str().to_owned(),
                        ExpressionTerm::new(value.clone()),
                    ));
                }
                _ => {
                    return Err(malformed(format!(
                        "facet restriction {item} must carry exactly one facet with a literal \
                         value"
                    )));
                }
            }
        }
        facets.sort();
        facets.dedup();
        Ok(OntologyExpression::DatatypeRestriction(base, facets))
    }
}

/// Refuse a data range standing where a class expression is required.
fn refuse_data_range_as_class(
    expression: &OntologyExpression,
    term: &Term,
) -> Result<(), SchemaCompileError> {
    if expression.has_data_only_construct() {
        return Err(SchemaCompileError::InvalidOntology {
            subject: term.to_string(),
            reason: format!(
                "the data range {} stands where a class expression is required",
                expression.canonical()
            ),
        });
    }
    Ok(())
}

/// An `owl:equivalentClass` or `rdfs:subClassOf` axiom with an anonymous
/// side, read but not yet classified: whether it defines a datatype depends on
/// every datatype declaration and definition in the request.
struct PendingAxiom {
    subject_term: Term,
    object_term: Term,
    subject: OntologyExpression,
    object: OntologyExpression,
    equivalent: bool,
    provenance: SchemaCoverageProvenance,
}

impl PendingAxiom {
    /// The datatype this axiom defines and its defining data range, when it is
    /// `DT owl:equivalentClass DR` (OWL 2 Structural Specification §9.4).
    fn datatype_definition(
        &self,
        datatypes: &BTreeSet<String>,
    ) -> Option<(&str, &OntologyExpression)> {
        if !self.equivalent {
            return None;
        }
        match (&self.subject, &self.object) {
            (OntologyExpression::Named(name), range) | (range, OntologyExpression::Named(name))
                if is_data_range(range, datatypes) =>
            {
                Some((name, range))
            }
            _ => None,
        }
    }
}

/// Whether an expression is a data range: no class-only construct, and a
/// data-only construct or only datatypes as named members.
fn is_data_range(expression: &OntologyExpression, datatypes: &BTreeSet<String>) -> bool {
    !expression.has_class_only_construct()
        && (expression.has_data_only_construct()
            || expression.all_named_members_match(&|iri| is_datatype(iri, datatypes)))
}

fn single<'t>(
    values: &'t [Term],
    predicate: &str,
    owner: &str,
) -> Result<&'t Term, SchemaCompileError> {
    match values {
        [one] => Ok(one),
        _ => Err(SchemaCompileError::InvalidOntology {
            subject: owner.to_owned(),
            reason: format!(
                "<{predicate}> has {} values on one expression; exactly one is required",
                values.len()
            ),
        }),
    }
}

/// A cardinality: a literal of `xsd:integer` or a datatype derived from it,
/// whose value is a non-negative integer.
fn cardinality(value: &Term, facet: &str, owner: &str) -> Result<u64, SchemaCompileError> {
    let malformed = |reason: String| SchemaCompileError::InvalidOntology {
        subject: owner.to_owned(),
        reason,
    };
    let Term::Literal(literal) = value else {
        return Err(malformed(format!(
            "<{facet}> requires a non-negative integer literal; found {value}"
        )));
    };
    if !XsdDatatype::from_iri(literal.datatype_str()).is_some_and(XsdDatatype::is_integer_family) {
        return Err(malformed(format!(
            "<{facet}> requires a non-negative integer literal; found {value}"
        )));
    }
    non_negative_integer(literal.value()).ok_or_else(|| {
        malformed(format!(
            "<{facet}> requires a non-negative integer no larger than {}; found {value}",
            u64::MAX
        ))
    })
}

/// `owl:hasSelf` takes exactly the literal `true` (OWL 2 Mapping Table 13).
fn has_self_value(value: &Term, owner: &str) -> Result<(), SchemaCompileError> {
    match value {
        Term::Literal(literal)
            if literal.datatype_str() == XSD_BOOLEAN && matches!(literal.value(), "true" | "1") =>
        {
            Ok(())
        }
        _ => Err(SchemaCompileError::InvalidOntology {
            subject: owner.to_owned(),
            reason: format!("owl:hasSelf requires the literal true; found {value}"),
        }),
    }
}

/// Catalog every property a restriction in `expression` names, with the
/// axiom that names it as provenance.
fn catalog_restrictions(
    expression: &OntologyExpression,
    provenance: &SchemaCoverageProvenance,
    properties: &mut BTreeMap<String, PropertyFacts>,
) -> Result<(), SchemaCompileError> {
    expression.visit_restrictions(&mut |on, _restriction| {
        for iri in on.iris() {
            let facts = property_entry(properties, iri)?;
            facts.declarations.insert(OWL_ON_PROPERTY.to_owned());
            facts.provenance.insert(provenance.clone());
        }
        Ok(())
    })
}

// ── Surface derivation ──────────────────────────────────────────────────────

/// Build the deterministic class/property relation for one request.
#[allow(
    clippy::too_many_lines,
    reason = "one pass over the sorted schema rows, dispatching each axiom kind"
)]
pub(crate) fn build(
    request: &SchemaCompileRequest<'_>,
) -> Result<SchemaSurface, SchemaCompileError> {
    let union = RdfDataset::union(&[request.ontology(), request.shapes().shapes_dataset.as_ref()]);
    let mut rows = dataset_rows(&union);
    rows.sort_by(|left, right| {
        left.subject_key
            .cmp(&right.subject_key)
            .then_with(|| left.predicate.cmp(&right.predicate))
            .then_with(|| left.object_key.cmp(&right.object_key))
    });
    rows.dedup_by(|left, right| {
        left.subject == right.subject
            && left.predicate == right.predicate
            && left.object == right.object
    });

    let shape_classes = shape_class_info(request.shapes());
    let mut properties: BTreeMap<String, PropertyFacts> = BTreeMap::new();
    let mut explicit_classes = BTreeSet::new();
    let mut datatypes = BTreeSet::new();
    let mut subclass_relations = Vec::new();
    let mut equivalent_class_relations = Vec::new();
    let mut property_relations = Vec::new();
    let mut named_equivalences: Vec<(String, String)> = Vec::new();
    let mut pending_axioms: Vec<PendingAxiom> = Vec::new();
    let mut reader = ExpressionReader::new(&union);

    for (class, info) in &shape_classes {
        explicit_classes.insert(class.clone());
        for property in &info.direct_properties {
            let facts = property_entry(&mut properties, property)?;
            facts.declarations.insert("sh:path".to_owned());
            facts.provenance.insert(SchemaCoverageProvenance {
                subject: class.clone(),
                predicate: crate::model::sh::PATH.to_owned(),
                object: format!("<{property}>"),
            });
        }
    }

    for row in &rows {
        match row.predicate.as_str() {
            rdfs::SUB_CLASS_OF | OWL_EQUIVALENT_CLASS => {
                let equivalent = row.predicate == OWL_EQUIVALENT_CLASS;
                if let (Some(child), Some(parent)) =
                    (named_iri(&row.subject), named_iri(&row.object))
                {
                    if equivalent {
                        // Two classes, or a datatype and its definition; which
                        // is decided once every datatype is known.
                        named_equivalences.push((child.to_owned(), parent.to_owned()));
                    } else {
                        explicit_classes.insert(child.to_owned());
                        explicit_classes.insert(parent.to_owned());
                        subclass_relations.push((child.to_owned(), parent.to_owned()));
                    }
                    continue;
                }
                let subject = reader.expression(&row.subject, 0)?;
                let object = reader.expression(&row.object, 0)?;
                if !equivalent {
                    refuse_data_range_as_class(&subject, &row.subject)?;
                    refuse_data_range_as_class(&object, &row.object)?;
                }
                let provenance = SchemaCoverageProvenance {
                    subject: subject.provenance_subject(),
                    predicate: row.predicate.clone(),
                    object: object.canonical(),
                };
                catalog_restrictions(&subject, &provenance, &mut properties)?;
                catalog_restrictions(&object, &provenance, &mut properties)?;
                pending_axioms.push(PendingAxiom {
                    subject_term: row.subject.clone(),
                    object_term: row.object.clone(),
                    subject,
                    object,
                    equivalent,
                    provenance,
                });
            }
            RDFS_SUB_PROPERTY_OF | OWL_EQUIVALENT_PROPERTY | OWL_INVERSE_OF => {
                // `_:x owl:inverseOf p` with a blank subject defines an inverse
                // property expression; it is read where that expression is used.
                if row.predicate == OWL_INVERSE_OF && matches!(row.subject, Term::BlankNode(_)) {
                    continue;
                }
                let left = reader.property_expression(&row.subject)?;
                let mut right = reader.property_expression(&row.object)?;
                let provenance = SchemaCoverageProvenance {
                    subject: left.provenance_subject(),
                    predicate: row.predicate.clone(),
                    object: right.canonical(),
                };
                let left_facts = property_entry(&mut properties, left.iri())?;
                left_facts
                    .declarations
                    .insert(format!("{}:subject", row.predicate));
                left_facts.provenance.insert(provenance.clone());
                let right_facts = property_entry(&mut properties, right.iri())?;
                right_facts
                    .declarations
                    .insert(format!("{}:object", row.predicate));
                right_facts.provenance.insert(provenance);
                let kind = match row.predicate.as_str() {
                    RDFS_SUB_PROPERTY_OF => PropertyRelationKind::SubProperty,
                    OWL_EQUIVALENT_PROPERTY => PropertyRelationKind::Equivalent,
                    _ => {
                        // `p owl:inverseOf q` is `p` equivalent to the inverse of `q`.
                        right = match right {
                            PropertyExpression::Named(iri) => PropertyExpression::Inverse(iri),
                            PropertyExpression::Inverse(iri) => PropertyExpression::Named(iri),
                        };
                        PropertyRelationKind::Equivalent
                    }
                };
                property_relations.push(PropertyRelation { left, right, kind });
            }
            predicate => {
                let Some(subject_iri) = named_iri(&row.subject) else {
                    continue;
                };
                match predicate {
                    rdf::TYPE => {
                        let Some(type_iri) = named_iri(&row.object) else {
                            continue;
                        };
                        match type_iri {
                            RDF_PROPERTY
                            | OWL_OBJECT_PROPERTY
                            | OWL_DATATYPE_PROPERTY
                            | OWL_ANNOTATION_PROPERTY
                            | OWL_FUNCTIONAL_PROPERTY
                            | OWL_INVERSE_FUNCTIONAL_PROPERTY => {
                                let facts = property_entry(&mut properties, subject_iri)?;
                                facts.declarations.insert(type_iri.to_owned());
                                let provenance = axiom_provenance(subject_iri, rdf::TYPE, type_iri);
                                facts.provenance.insert(provenance.clone());
                                match type_iri {
                                    OWL_OBJECT_PROPERTY => facts.object_property = true,
                                    OWL_DATATYPE_PROPERTY => facts.datatype_property = true,
                                    OWL_ANNOTATION_PROPERTY => facts.annotation_property = true,
                                    OWL_FUNCTIONAL_PROPERTY => {
                                        facts.functional.insert(provenance);
                                    }
                                    _ => {}
                                }
                            }
                            rdfs::CLASS | OWL_CLASS => {
                                explicit_classes.insert(subject_iri.to_owned());
                            }
                            RDFS_DATATYPE | OWL_DATA_RANGE => {
                                datatypes.insert(subject_iri.to_owned());
                            }
                            _ => {}
                        }
                    }
                    RDFS_DOMAIN | rdfs::RANGE => {
                        let expression = reader.expression(&row.object, 0)?;
                        let provenance = SchemaCoverageProvenance {
                            subject: subject_iri.to_owned(),
                            predicate: predicate.to_owned(),
                            object: expression.canonical(),
                        };
                        catalog_restrictions(&expression, &provenance, &mut properties)?;
                        let sourced = SourcedExpression {
                            expression,
                            provenance: provenance.clone(),
                        };
                        let facts = property_entry(&mut properties, subject_iri)?;
                        facts.declarations.insert(predicate.to_owned());
                        facts.provenance.insert(provenance);
                        if predicate == RDFS_DOMAIN {
                            facts.domains.insert(sourced);
                        } else {
                            facts.ranges.insert(sourced);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    // Datatype definitions (OWL 2 Structural Specification §9.4): an
    // equivalence between a datatype and a data range defines the datatype.
    // A definition can make another equivalence a definition, so this runs to
    // a fixpoint, bounded by the number of equivalences.
    let mut datatype_definitions: BTreeMap<String, OntologyExpression> = BTreeMap::new();
    let mut datatype_axioms: Vec<(SchemaCoverageProvenance, String)> = Vec::new();
    loop {
        let known = datatypes.len();
        named_equivalences.retain(|(left, right)| {
            match (
                is_datatype(left, &datatypes),
                is_datatype(right, &datatypes),
            ) {
                (false, false) => true,
                (true, true) => false,
                (left_is_datatype, _) => {
                    let (defined, by) = if left_is_datatype {
                        (right, left)
                    } else {
                        (left, right)
                    };
                    datatypes.insert(defined.clone());
                    datatype_definitions
                        .entry(defined.clone())
                        .or_insert_with(|| OntologyExpression::Named(by.clone()));
                    false
                }
            }
        });
        let mut index = 0;
        while index < pending_axioms.len() {
            if let Some((name, range)) = pending_axioms[index].datatype_definition(&datatypes) {
                let (name, range) = (name.to_owned(), range.clone());
                let axiom = pending_axioms.swap_remove(index);
                datatypes.insert(name.clone());
                datatype_axioms.push((axiom.provenance, range.canonical()));
                datatype_definitions.entry(name).or_insert(range);
            } else {
                index += 1;
            }
        }
        if datatypes.len() == known {
            break;
        }
    }
    for (left, right) in named_equivalences {
        explicit_classes.insert(left.clone());
        explicit_classes.insert(right.clone());
        equivalent_class_relations.push((left, right));
    }
    pending_axioms.sort_by(|left, right| left.provenance.cmp(&right.provenance));
    let mut class_axioms: Vec<ClassAxiom> = Vec::with_capacity(pending_axioms.len());
    for axiom in pending_axioms {
        refuse_data_range_as_class(&axiom.subject, &axiom.subject_term)?;
        refuse_data_range_as_class(&axiom.object, &axiom.object_term)?;
        class_axioms.push(ClassAxiom::classify(
            &axiom.subject,
            &axiom.object,
            axiom.equivalent,
            axiom.provenance,
        ));
    }
    datatype_axioms.sort();

    enforce_limit("properties", properties.len(), MAX_SCHEMA_PROPERTIES)?;
    propagate_property_facts(&mut properties, &property_relations)?;
    validate_property_ranges(&properties, &datatypes)?;
    validate_restriction_fillers(&properties, &class_axioms, &datatypes)?;

    let class_names = |expression: &OntologyExpression, out: &mut BTreeSet<String>| {
        let mut mentioned = BTreeSet::new();
        expression.mentioned_names(&mut mentioned);
        out.extend(
            mentioned
                .into_iter()
                .filter(|iri| !is_datatype(iri, &datatypes)),
        );
    };
    for facts in properties.values() {
        for domain in &facts.domains {
            domain.expression.named_members(&mut explicit_classes);
            class_names(&domain.expression, &mut explicit_classes);
        }
        let kind = facts.kind("range class discovery")?;
        if matches!(
            kind,
            OntologyPropertyKind::Object | OntologyPropertyKind::Generic
        ) {
            for range in &facts.ranges {
                class_names(&range.expression, &mut explicit_classes);
            }
        }
    }
    for axiom in &class_axioms {
        for (carrier, conjuncts) in &axiom.carriers {
            explicit_classes.insert(carrier.clone());
            for conjunct in conjuncts {
                class_names(conjunct, &mut explicit_classes);
            }
        }
        for (child, parent) in axiom.edges() {
            subclass_relations.push((child.to_owned(), parent.to_owned()));
        }
    }
    explicit_classes.extend(shape_classes.keys().cloned());
    enforce_limit("classes", explicit_classes.len(), MAX_SCHEMA_CLASSES)?;
    let supertypes = class_supertypes(
        &explicit_classes,
        &subclass_relations,
        &equivalent_class_relations,
    )?;

    assemble_surface(
        request,
        properties,
        &shape_classes,
        explicit_classes,
        DatatypeFacts {
            declared: datatypes,
            definitions: datatype_definitions,
            axioms: datatype_axioms,
        },
        &supertypes,
        &class_axioms,
    )
}

/// The request's datatypes: every declared or defined one, each definition's
/// data range, and the anonymous definition axioms for the manifest.
struct DatatypeFacts {
    declared: BTreeSet<String>,
    definitions: BTreeMap<String, OntologyExpression>,
    axioms: Vec<(SchemaCoverageProvenance, String)>,
}

fn dataset_rows(dataset: &RdfDataset) -> Vec<TripleRow> {
    native_quads(dataset, None, None, None, GraphFilter::AnyGraph)
        .into_iter()
        .map(|(subject, predicate, object)| {
            let subject_key = subject.to_string();
            let object_key = object.to_string();
            TripleRow {
                subject,
                subject_key,
                predicate: predicate.into_string(),
                object,
                object_key,
            }
        })
        .collect()
}

fn named_iri(term: &Term) -> Option<&str> {
    match term {
        Term::NamedNode(node) => Some(node.as_str()),
        _ => None,
    }
}

fn property_entry<'a>(
    properties: &'a mut BTreeMap<String, PropertyFacts>,
    iri: &str,
) -> Result<&'a mut PropertyFacts, SchemaCompileError> {
    if !properties.contains_key(iri) && properties.len() == MAX_SCHEMA_PROPERTIES {
        return Err(SchemaCompileError::LimitExceeded {
            resource: "properties",
            limit: MAX_SCHEMA_PROPERTIES,
            observed: MAX_SCHEMA_PROPERTIES + 1,
        });
    }
    Ok(properties.entry(iri.to_owned()).or_default())
}

fn axiom_provenance(subject: &str, predicate: &str, object_iri: &str) -> SchemaCoverageProvenance {
    SchemaCoverageProvenance {
        subject: subject.to_owned(),
        predicate: predicate.to_owned(),
        object: format!("<{object_iri}>"),
    }
}

fn enforce_limit(
    resource: &'static str,
    observed: usize,
    limit: usize,
) -> Result<(), SchemaCompileError> {
    if observed > limit {
        Err(SchemaCompileError::LimitExceeded {
            resource,
            limit,
            observed,
        })
    } else {
        Ok(())
    }
}

fn coverage_cell_count(
    property_count: usize,
    eligible_class_count: usize,
    external_shaped_cells: usize,
    limit: usize,
) -> Result<usize, SchemaCompileError> {
    let observed = property_count
        .checked_mul(eligible_class_count)
        .and_then(|cells| cells.checked_add(external_shaped_cells))
        .unwrap_or(usize::MAX);
    enforce_limit("class/property coverage cells", observed, limit)?;
    Ok(observed)
}

fn shape_class_info(shapes: &crate::shapes::Shapes) -> BTreeMap<String, ShapeClassInfo> {
    let mut classes = BTreeMap::new();
    for shape in &shapes.node_shapes {
        if shape.deactivated {
            continue;
        }
        for target in &shape.targets {
            let class = match target {
                Target::Class(node) => Some(node.as_str()),
                Target::ImplicitClass(Term::NamedNode(node)) => Some(node.as_str()),
                _ => None,
            };
            if let Some(class) = class {
                merge_shape_info(classes.entry(class.to_owned()).or_default(), shape, class);
            }
        }
    }
    classes
}

fn merge_shape_info(info: &mut ShapeClassInfo, shape: &Shape, class: &str) {
    let direct = direct_shape_properties(shape);
    info.direct_properties.extend(direct.iter().cloned());
    for constraint in &shape.constraints {
        if let Constraint::Closed { ignored, mode } = constraint {
            // What the closed shape permits an instance of `class`: under
            // `sh:closed true` the shape's own property paths; under
            // `sh:closed sh:ByTypes` (SHACL 1.2 Core §7.9.1) what the instance's
            // type `class` collects, plus `rdf:type`.
            let permitted = match mode {
                ClosedMode::Declared => direct.clone(),
                ClosedMode::ByTypes(index) => index
                    .properties(&Term::NamedNode(NamedNode::new_unchecked(class)))
                    .iter()
                    .map(|property| property.as_str().to_owned())
                    .chain(std::iter::once(rdf::TYPE.to_owned()))
                    .collect(),
            };
            info.closed_surfaces.push(ClosedSurface {
                direct_properties: permitted,
                ignored_properties: ignored
                    .iter()
                    .map(|node| node.as_str().to_owned())
                    .collect(),
            });
        }
    }
}

fn direct_shape_properties(shape: &Shape) -> BTreeSet<String> {
    shape
        .property_shapes
        .iter()
        .filter(|property| !property.deactivated)
        .filter_map(|property| match &property.path {
            Path::Predicate(node) => Some(node.as_str().to_owned()),
            _ => None,
        })
        .collect()
}

/// The domain and range nodes of a property expression in the propagation
/// graph: node `2i` holds property `i`'s domains and `2i + 1` its ranges, and
/// an inverse exchanges the two.
const fn expression_nodes(index: usize, inverse: bool) -> (usize, usize) {
    if inverse {
        (index * 2 + 1, index * 2)
    } else {
        (index * 2, index * 2 + 1)
    }
}

fn propagate_property_facts(
    properties: &mut BTreeMap<String, PropertyFacts>,
    relations: &[PropertyRelation],
) -> Result<(), SchemaCompileError> {
    let names: Vec<String> = properties.keys().cloned().collect();
    let indices: BTreeMap<&str, usize> = names
        .iter()
        .enumerate()
        .map(|(index, name)| (name.as_str(), index))
        .collect();
    let mut expression_graph = vec![BTreeSet::new(); names.len() * 2];
    let mut functional_graph = vec![BTreeSet::new(); names.len()];

    for relation in relations {
        let left = indices[relation.left.iri()];
        let right = indices[relation.right.iri()];
        let (left_domain, left_range) = expression_nodes(left, relation.left.is_inverse());
        let (right_domain, right_range) = expression_nodes(right, relation.right.is_inverse());
        // Forward functionality follows a relation only between expressions of
        // the same direction: the inverse of a functional property is
        // inverse-functional, which selects no scalar representation.
        let same_direction = relation.left.is_inverse() == relation.right.is_inverse();
        match relation.kind {
            PropertyRelationKind::SubProperty => {
                add_edge(&mut expression_graph, right_domain, left_domain);
                add_edge(&mut expression_graph, right_range, left_range);
                if same_direction {
                    add_edge(&mut functional_graph, right, left);
                }
            }
            PropertyRelationKind::Equivalent => {
                add_bidirectional_edge(&mut expression_graph, left_domain, right_domain);
                add_bidirectional_edge(&mut expression_graph, left_range, right_range);
                if same_direction {
                    add_bidirectional_edge(&mut functional_graph, left, right);
                }
            }
        }
    }
    let edge_count = expression_graph.iter().map(BTreeSet::len).sum::<usize>()
        + functional_graph.iter().map(BTreeSet::len).sum::<usize>();
    enforce_limit("ontology relation edges", edge_count, MAX_SCHEMA_RELATIONS)?;

    let mut expression_seeds = vec![BTreeSet::new(); names.len() * 2];
    let mut functional_seeds = vec![BTreeSet::new(); names.len()];
    for (index, name) in names.iter().enumerate() {
        let facts = &properties[name];
        expression_seeds[index * 2].clone_from(&facts.domains);
        expression_seeds[index * 2 + 1].clone_from(&facts.ranges);
        functional_seeds[index].clone_from(&facts.functional);
    }
    let effective_expressions = propagate_sets(
        &expression_graph,
        expression_seeds,
        "propagated ontology expressions",
    )?;
    let effective_functionality = propagate_sets(
        &functional_graph,
        functional_seeds,
        "propagated functionality facts",
    )?;
    let mut effective_expressions = effective_expressions.into_iter();
    let mut effective_functionality = effective_functionality.into_iter();
    for name in &names {
        let facts = properties
            .get_mut(name)
            .expect("property index was built from this map");
        facts.domains = effective_expressions
            .next()
            .expect("each property has one propagated domain set");
        facts.ranges = effective_expressions
            .next()
            .expect("each property has one propagated range set");
        facts.functional = effective_functionality
            .next()
            .expect("each property has one propagated functionality set");
    }
    Ok(())
}

fn add_edge(graph: &mut [BTreeSet<usize>], source: usize, destination: usize) {
    graph[source].insert(destination);
}

fn add_bidirectional_edge(graph: &mut [BTreeSet<usize>], left: usize, right: usize) {
    add_edge(graph, left, right);
    add_edge(graph, right, left);
}

fn validate_property_ranges(
    properties: &BTreeMap<String, PropertyFacts>,
    datatypes: &BTreeSet<String>,
) -> Result<(), SchemaCompileError> {
    for (property, facts) in properties {
        let kind = facts.kind(property)?;
        for range in &facts.ranges {
            match kind {
                OntologyPropertyKind::Datatype
                    if range.expression.has_class_only_construct()
                        || !range.expression.all_named_members_match(&|iri| {
                            is_builtin_datatype(iri) || datatypes.contains(iri)
                        }) =>
                {
                    return Err(SchemaCompileError::InvalidOntology {
                        subject: property.clone(),
                        reason: format!(
                            "owl:DatatypeProperty has non-datatype range {}",
                            range.expression.canonical()
                        ),
                    });
                }
                OntologyPropertyKind::Object
                    if range.expression.has_data_only_construct()
                        || !range.expression.all_named_members_match(&|iri| {
                            !is_builtin_datatype(iri) && !datatypes.contains(iri)
                        }) =>
                {
                    return Err(SchemaCompileError::InvalidOntology {
                        subject: property.clone(),
                        reason: format!(
                            "owl:ObjectProperty has datatype range {}",
                            range.expression.canonical()
                        ),
                    });
                }
                _ => {}
            }
        }
    }
    Ok(())
}

/// Refuse a restriction whose filler, qualifier or value contradicts the kind
/// of the property it restricts: a class filler on an `owl:DatatypeProperty`,
/// a data range or literal on an `owl:ObjectProperty`, or `owl:hasSelf` on a
/// datatype property (OWL 2 Structural Specification §8.2 and §8.4).
fn validate_restriction_fillers(
    properties: &BTreeMap<String, PropertyFacts>,
    axioms: &[ClassAxiom],
    datatypes: &BTreeSet<String>,
) -> Result<(), SchemaCompileError> {
    let is_data = |iri: &str| is_datatype(iri, datatypes);
    for axiom in axioms {
        for (_, conjuncts) in &axiom.carriers {
            for conjunct in conjuncts {
                conjunct.visit_restrictions(&mut |on, restriction| {
                    let Some(iri) = on.named() else {
                        return Ok(());
                    };
                    let kind = properties
                        .get(iri)
                        .map_or(Ok(OntologyPropertyKind::Generic), |facts| facts.kind(iri))?;
                    let ill_typed = |reason: String| SchemaCompileError::InvalidOntology {
                        subject: iri.to_owned(),
                        reason: format!(
                            "{reason} in the axiom {}",
                            render_axiom(&axiom.provenance)
                        ),
                    };
                    match kind {
                        OntologyPropertyKind::Datatype => {
                            if matches!(restriction, Restriction::HasSelf) {
                                return Err(ill_typed(
                                    "owl:hasSelf restricts an owl:DatatypeProperty".to_owned(),
                                ));
                            }
                            if let Restriction::HasValue(value) = restriction
                                && !value.is_literal()
                            {
                                return Err(ill_typed(format!(
                                    "owl:DatatypeProperty has the individual {} as owl:hasValue",
                                    value.key
                                )));
                            }
                            for filler in restriction.fillers() {
                                if filler.has_class_only_construct()
                                    || !filler.all_named_members_match(&is_data)
                                {
                                    return Err(ill_typed(format!(
                                        "owl:DatatypeProperty is restricted to the class \
                                         expression {}",
                                        filler.canonical()
                                    )));
                                }
                            }
                        }
                        OntologyPropertyKind::Object => {
                            if let Restriction::HasValue(value) = restriction
                                && value.is_literal()
                            {
                                return Err(ill_typed(format!(
                                    "owl:ObjectProperty has the literal {} as owl:hasValue",
                                    value.key
                                )));
                            }
                            for filler in restriction.fillers() {
                                if filler.has_data_only_construct()
                                    || !filler.all_named_members_match(&|iri| !is_data(iri))
                                {
                                    return Err(ill_typed(format!(
                                        "owl:ObjectProperty is restricted to the data range {}",
                                        filler.canonical()
                                    )));
                                }
                            }
                        }
                        OntologyPropertyKind::Generic | OntologyPropertyKind::Annotation => {}
                    }
                    Ok(())
                })?;
            }
        }
    }
    Ok(())
}

fn render_axiom(provenance: &SchemaCoverageProvenance) -> String {
    format!(
        "{} <{}> {}",
        provenance.subject, provenance.predicate, provenance.object
    )
}

fn is_builtin_datatype(iri: &str) -> bool {
    iri.starts_with(crate::model::xsd::BASE) || iri == RDFS_LITERAL || iri == RDF_LANG_STRING
}

fn class_supertypes(
    classes: &BTreeSet<String>,
    subclasses: &[(String, String)],
    equivalents: &[(String, String)],
) -> Result<BTreeMap<String, BTreeSet<String>>, SchemaCompileError> {
    let names: Vec<String> = classes.iter().cloned().collect();
    let indices: BTreeMap<&str, usize> = names
        .iter()
        .enumerate()
        .map(|(index, name)| (name.as_str(), index))
        .collect();
    let mut graph = vec![BTreeSet::new(); names.len()];
    for (child, parent) in subclasses {
        if let (Some(&child), Some(&parent)) =
            (indices.get(child.as_str()), indices.get(parent.as_str()))
        {
            add_edge(&mut graph, parent, child);
        }
    }
    for (left, right) in equivalents {
        if let (Some(&left), Some(&right)) =
            (indices.get(left.as_str()), indices.get(right.as_str()))
        {
            add_bidirectional_edge(&mut graph, left, right);
        }
    }
    enforce_limit(
        "class hierarchy edges",
        graph.iter().map(BTreeSet::len).sum(),
        MAX_SCHEMA_RELATIONS,
    )?;
    let seeds: Vec<BTreeSet<String>> = names
        .iter()
        .map(|name| BTreeSet::from([name.clone()]))
        .collect();
    let effective = propagate_sets(&graph, seeds, "propagated class memberships")?;
    Ok(names.into_iter().zip(effective).collect())
}

/// Propagate ordered fact sets through a directed graph after condensing every
/// legal cycle into one strongly connected component.
fn propagate_sets<T: Clone + Ord>(
    graph: &[BTreeSet<usize>],
    seeds: Vec<BTreeSet<T>>,
    resource: &'static str,
) -> Result<Vec<BTreeSet<T>>, SchemaCompileError> {
    propagate_sets_with_limit(graph, seeds, resource, MAX_SCHEMA_RELATIONS)
}

fn propagate_sets_with_limit<T: Clone + Ord>(
    graph: &[BTreeSet<usize>],
    seeds: Vec<BTreeSet<T>>,
    resource: &'static str,
    limit: usize,
) -> Result<Vec<BTreeSet<T>>, SchemaCompileError> {
    debug_assert_eq!(graph.len(), seeds.len());
    if graph.is_empty() {
        return Ok(Vec::new());
    }
    let components = purrdf_core::graph::scc_component_index(graph);
    let component_count = components.iter().copied().max().map_or(0, |max| max + 1);
    let mut values = vec![BTreeSet::new(); component_count];
    let mut materialized = 0_usize;
    for (node, facts) in seeds.into_iter().enumerate() {
        for fact in facts {
            insert_propagated_fact(
                &mut values[components[node]],
                fact,
                &mut materialized,
                resource,
                limit,
            )?;
        }
    }

    let mut dag = vec![BTreeSet::new(); component_count];
    let mut indegree = vec![0_usize; component_count];
    for (source, destinations) in graph.iter().enumerate() {
        let source_component = components[source];
        for &destination in destinations {
            let destination_component = components[destination];
            if source_component != destination_component
                && dag[source_component].insert(destination_component)
            {
                indegree[destination_component] += 1;
            }
        }
    }

    let mut ready: BTreeSet<usize> = indegree
        .iter()
        .enumerate()
        .filter_map(|(component, &degree)| (degree == 0).then_some(component))
        .collect();
    let mut visited = 0_usize;
    while let Some(component) = ready.pop_first() {
        visited += 1;
        let inherited: Vec<T> = values[component].iter().cloned().collect();
        for &destination in &dag[component] {
            for fact in &inherited {
                insert_propagated_fact(
                    &mut values[destination],
                    fact.clone(),
                    &mut materialized,
                    resource,
                    limit,
                )?;
            }
            indegree[destination] -= 1;
            if indegree[destination] == 0 {
                ready.insert(destination);
            }
        }
    }
    debug_assert_eq!(visited, component_count, "component graph is acyclic");
    let output_entries = components.iter().fold(0_usize, |total, &component| {
        total.saturating_add(values[component].len())
    });
    enforce_limit(resource, output_entries, limit)?;
    Ok(components
        .into_iter()
        .map(|component| values[component].clone())
        .collect())
}

fn insert_propagated_fact<T: Ord>(
    destination: &mut BTreeSet<T>,
    fact: T,
    materialized: &mut usize,
    resource: &'static str,
    limit: usize,
) -> Result<(), SchemaCompileError> {
    if destination.get(&fact).is_none() {
        let observed = materialized.saturating_add(1);
        enforce_limit(resource, observed, limit)?;
        destination.insert(fact);
        *materialized = observed;
    }
    Ok(())
}

/// What one eligible class inherits from the anonymous class axioms on it and
/// its superclasses.
#[derive(Debug, Default)]
struct ClassExpressionFacts<'a> {
    /// `(axiom index, conjunct)`, canonically ordered and de-duplicated.
    entries: Vec<(usize, &'a OntologyExpression)>,
    anonymous: AnonymousSupers,
    /// Restrictions on one named property, asserted as top-level conjuncts.
    restrictions: BTreeMap<&'a str, BTreeSet<&'a Restriction>>,
    /// Properties some top-level conjunct requires a value of. By OWL
    /// semantics such a class is within the property's `rdfs:domain`.
    existential: BTreeSet<&'a str>,
    /// Every named property any conjunct restricts, at any depth.
    mentioned: BTreeSet<&'a str>,
}

impl<'a> ClassExpressionFacts<'a> {
    fn new(entries: Vec<(usize, &'a OntologyExpression)>) -> Self {
        let mut facts = Self {
            entries,
            ..Self::default()
        };
        for &(_, conjunct) in &facts.entries {
            match conjunct {
                OntologyExpression::Named(_) => continue,
                OntologyExpression::Restriction(on, restriction) => {
                    if let Some(iri) = on.named() {
                        if restriction.is_existential() {
                            facts.existential.insert(iri);
                        }
                        if !matches!(**restriction, Restriction::HasSelf) {
                            facts
                                .restrictions
                                .entry(iri)
                                .or_default()
                                .insert(restriction);
                        }
                    }
                }
                OntologyExpression::Union(members) => {
                    facts
                        .anonymous
                        .unions
                        .push(members.iter().map(OntologyExpression::canonical).collect());
                }
                _ => {}
            }
            facts.anonymous.canonical.insert(conjunct.canonical());
            let mentioned = &mut facts.mentioned;
            let _: Result<(), ()> = conjunct.visit_restrictions(&mut |on, _| {
                if let Some(iri) = on.named() {
                    mentioned.insert(iri);
                }
                Ok(())
            });
        }
        facts
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "one pass over the property × class cells, each decision kept beside its row"
)]
fn assemble_surface(
    request: &SchemaCompileRequest<'_>,
    properties: BTreeMap<String, PropertyFacts>,
    shape_classes: &BTreeMap<String, ShapeClassInfo>,
    explicit_classes: BTreeSet<String>,
    datatype_facts: DatatypeFacts,
    supertypes: &BTreeMap<String, BTreeSet<String>>,
    class_axioms: &[ClassAxiom],
) -> Result<SchemaSurface, SchemaCompileError> {
    let DatatypeFacts {
        declared: datatypes,
        definitions: datatype_definitions,
        axioms: datatype_axioms,
    } = datatype_facts;
    let eligible_classes: Vec<String> = explicit_classes
        .into_iter()
        .filter(|class| request.namespaces().is_caller_owned(class))
        .collect();
    let shaped_classes: BTreeSet<String> = shape_classes.keys().cloned().collect();
    let eligible_class_set: BTreeSet<&str> = eligible_classes.iter().map(String::as_str).collect();
    let external_shaped_classes: Vec<&str> = shaped_classes
        .iter()
        .map(String::as_str)
        .filter(|class| !eligible_class_set.contains(class))
        .collect();
    let represented_classes: BTreeSet<String> = match request.mode() {
        SchemaSurfaceMode::ShapedOnly => shaped_classes.clone(),
        SchemaSurfaceMode::OntologyComplete => eligible_classes
            .iter()
            .cloned()
            .chain(shaped_classes.iter().cloned())
            .collect(),
    };
    enforce_limit("classes", represented_classes.len(), MAX_SCHEMA_CLASSES)?;

    let mut classes: BTreeMap<String, SurfaceClass> = represented_classes
        .iter()
        .map(|class| {
            (
                class.clone(),
                SurfaceClass {
                    synthesized_open: !shaped_classes.contains(class),
                    ..SurfaceClass::default()
                },
            )
        })
        .collect();
    let mut report_properties = Vec::with_capacity(properties.len());
    let external_shaped_cells = external_shaped_classes
        .iter()
        .map(|class| shape_classes[*class].direct_properties.len())
        .fold(0_usize, usize::saturating_add);
    coverage_cell_count(
        properties.len(),
        eligible_classes.len(),
        external_shaped_cells,
        MAX_SCHEMA_RELATIONS,
    )?;

    let class_facts = class_expression_facts(class_axioms, &eligible_classes, supertypes)?;
    let no_anonymous = AnonymousSupers::default();
    let mut statuses: BTreeMap<(String, String), SchemaCoverageStatus> = BTreeMap::new();

    for (property_iri, facts) in properties {
        let kind = facts.kind(&property_iri)?;
        let mut datatype_iris = BTreeSet::new();
        for range in &facts.ranges {
            range.expression.named_members(&mut datatype_iris);
        }
        datatype_iris.retain(|iri| datatypes.contains(iri));
        let mut class_rows = Vec::new();
        let mut outcomes = BTreeSet::new();
        let base_provenance: Vec<SchemaCoverageProvenance> = facts
            .provenance
            .iter()
            .chain(facts.domains.iter().map(|domain| &domain.provenance))
            .chain(facts.ranges.iter().map(|range| &range.provenance))
            .chain(facts.functional.iter())
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        // A range a value schema cannot state exactly (a restriction, a
        // complement, an enumeration of individuals) is projected as an open
        // node reference, like a named class range, but weaker than stated.
        let approximate_range = facts.ranges.iter().any(|range| {
            !range.expression.is_named_skeleton()
                && value_precision(&range.expression, &datatypes) == ValuePrecision::Approximate
        });
        // Membership in a domain beyond the named hierarchy is read
        // structurally, so an exclusion against one is not a proof.
        let undecided_domain = facts
            .domains
            .iter()
            .any(|domain| !domain.expression.is_named_skeleton());

        for class_iri in &eligible_classes {
            let shape_info = shape_classes.get(class_iri);
            let class_expressions = class_facts.get(class_iri.as_str());
            let has_shape = shape_info
                .is_some_and(|info| info.direct_properties.contains(property_iri.as_str()));
            let existential = class_expressions
                .is_some_and(|expressions| expressions.existential.contains(property_iri.as_str()));
            let (status, precision) = if has_shape {
                (
                    SchemaCoverageStatus::HasShape,
                    SchemaCoveragePrecision::Exact,
                )
            } else if request.mode() == SchemaSurfaceMode::ShapedOnly {
                (
                    SchemaCoverageStatus::ExcludedShapedOnly,
                    SchemaCoveragePrecision::Exact,
                )
            } else if !request.namespaces().is_caller_owned(&property_iri) {
                (
                    SchemaCoverageStatus::ExcludedNamespace,
                    SchemaCoveragePrecision::Exact,
                )
            } else if !existential
                && !facts.domains.iter().all(|domain| {
                    supertypes.get(class_iri).is_some_and(|types| {
                        domain.expression.matches_class(
                            types,
                            class_expressions
                                .map_or(&no_anonymous, |expressions| &expressions.anonymous),
                        )
                    })
                })
            {
                (
                    SchemaCoverageStatus::ExcludedDomain,
                    if undecided_domain {
                        SchemaCoveragePrecision::RepresentationApproximation
                    } else {
                        SchemaCoveragePrecision::Exact
                    },
                )
            } else if shape_info.is_some_and(|info| !info.closed_allows(&property_iri)) {
                (
                    SchemaCoverageStatus::ExcludedClosedShape,
                    SchemaCoveragePrecision::Exact,
                )
            } else {
                let restrictions: Vec<Restriction> = class_expressions
                    .and_then(|expressions| expressions.restrictions.get(property_iri.as_str()))
                    .map(|set| set.iter().map(|&restriction| restriction.clone()).collect())
                    .unwrap_or_default();
                let restricted_approximately = restrictions.iter().any(|restriction| {
                    restriction_outcomes(restriction, &datatypes)
                        .iter()
                        .any(|(outcome, _)| *outcome != SchemaExpressionOutcome::Projected)
                });
                let precision = if facts.functional.is_empty()
                    && !restricted_approximately
                    && !approximate_range
                {
                    SchemaCoveragePrecision::Exact
                } else {
                    SchemaCoveragePrecision::RepresentationApproximation
                };
                let class = classes
                    .get_mut(class_iri)
                    .expect("eligible complete-mode class has a surface entry");
                class.properties.insert(
                    property_iri.clone(),
                    SurfaceProperty {
                        iri: property_iri.clone(),
                        kind,
                        ranges: facts
                            .ranges
                            .iter()
                            .map(|range| range.expression.clone())
                            .collect(),
                        datatype_iris: datatype_iris.clone(),
                        functional: !facts.functional.is_empty(),
                        provenance: base_provenance.clone(),
                        restrictions,
                    },
                );
                (SchemaCoverageStatus::IncludedUnshaped, precision)
            };
            if class_expressions
                .is_some_and(|expressions| expressions.mentioned.contains(property_iri.as_str()))
            {
                statuses.insert((property_iri.clone(), class_iri.clone()), status);
            }
            outcomes.insert(status);
            class_rows.push(SchemaClassPropertyCoverage {
                class_iri: class_iri.clone(),
                synthesized_open_class: classes
                    .get(class_iri)
                    .is_some_and(|class| class.synthesized_open),
                status,
                precision,
                provenance: base_provenance.clone(),
            });
        }

        // A shaped class may intentionally live outside the caller-owned
        // ontology boundary; retain its direct-shape audit row because legacy
        // compilation still emits it.
        for &class_iri in &external_shaped_classes {
            if shape_classes[class_iri]
                .direct_properties
                .contains(&property_iri)
            {
                outcomes.insert(SchemaCoverageStatus::HasShape);
                class_rows.push(SchemaClassPropertyCoverage {
                    class_iri: class_iri.to_owned(),
                    synthesized_open_class: false,
                    status: SchemaCoverageStatus::HasShape,
                    precision: SchemaCoveragePrecision::Exact,
                    provenance: base_provenance.clone(),
                });
            }
        }
        if outcomes.is_empty() {
            outcomes.insert(if !request.namespaces().is_caller_owned(&property_iri) {
                SchemaCoverageStatus::ExcludedNamespace
            } else if request.mode() == SchemaSurfaceMode::ShapedOnly {
                SchemaCoverageStatus::ExcludedShapedOnly
            } else {
                SchemaCoverageStatus::ExcludedDomain
            });
        }
        class_rows.sort_by(|left, right| left.class_iri.cmp(&right.class_iri));
        report_properties.push(SchemaPropertyCoverage {
            property_iri,
            declarations: facts.declarations.into_iter().collect(),
            outcomes: outcomes.into_iter().collect(),
            classes: class_rows,
        });
    }

    let class_expressions = class_expression_report(
        request.mode(),
        class_axioms,
        &datatype_axioms,
        &class_facts,
        &statuses,
        supertypes,
        &mut classes,
        &datatypes,
    )?;

    let surface = SchemaSurface {
        classes,
        report: SchemaCoverageReport {
            mode: request.mode(),
            properties: report_properties,
        },
        class_expressions,
        datatypes,
        datatype_definitions,
    };
    surface.assert_conservation();
    Ok(surface)
}

/// The anonymous class axiom conjuncts each eligible class inherits: those
/// carried by the class itself and by every one of its superclasses.
fn class_expression_facts<'a>(
    class_axioms: &'a [ClassAxiom],
    eligible_classes: &'a [String],
    supertypes: &BTreeMap<String, BTreeSet<String>>,
) -> Result<BTreeMap<&'a str, ClassExpressionFacts<'a>>, SchemaCompileError> {
    let mut carried: BTreeMap<&str, Vec<(usize, &OntologyExpression)>> = BTreeMap::new();
    for (index, axiom) in class_axioms.iter().enumerate() {
        for (carrier, conjuncts) in &axiom.carriers {
            carried
                .entry(carrier.as_str())
                .or_default()
                .extend(conjuncts.iter().map(|conjunct| (index, conjunct)));
        }
    }
    let mut facts = BTreeMap::new();
    if carried.is_empty() {
        return Ok(facts);
    }
    let mut assertions = 0_usize;
    for class_iri in eligible_classes {
        let Some(types) = supertypes.get(class_iri) else {
            continue;
        };
        let mut entries: BTreeSet<(usize, &OntologyExpression)> = BTreeSet::new();
        for supertype in types {
            if let Some(list) = carried.get(supertype.as_str()) {
                entries.extend(list.iter().copied());
            }
        }
        if entries.is_empty() {
            continue;
        }
        assertions = assertions.saturating_add(entries.len());
        enforce_limit(
            "inherited class-expression assertions",
            assertions,
            MAX_SCHEMA_RELATIONS,
        )?;
        facts.insert(
            class_iri.as_str(),
            ClassExpressionFacts::new(entries.into_iter().collect()),
        );
    }
    Ok(facts)
}

// ── Class-expression coverage ───────────────────────────────────────────────

const HAS_SHAPE_REASON: &str =
    "a direct SHACL property shape on this class is authoritative for the restricted property";
const NAMESPACE_REASON: &str =
    "the restricted property is outside the caller-declared vocabulary boundary";
const UNSATISFIED_SCOPE_REASON: &str = "the class does not satisfy the restricted property's rdfs:domain, \
     so the property is not emitted on it";
const CLOSED_REASON: &str =
    "an authoritative sh:closed shape on this class excludes the restricted property";
const INVERSE_REASON: &str = "a restriction on an inverse property constrains the nodes that \
     refer to the focus node, which a schema at the focus node cannot see";
const NARY_REASON: &str = "an n-ary data restriction relates the values of several properties, \
     which JSON Schema keywords cannot compare";
const HAS_SELF_REASON: &str = "owl:hasSelf requires the focus node to be its own value, an \
     identity between two instance locations that no JSON Schema keyword states";
const ONE_OF_REASON: &str = "the focus node's @id is restricted to the enumerated individuals; \
     without the unique-name assumption OWL also admits an IRI owl:sameAs one of them";
const ONE_OF_ANONYMOUS_REASON: &str =
    "an enumerated anonymous individual has no @id stable beyond one document";
const COMPLEMENT_REASON: &str = "@type must not include the complemented class; membership in it \
     entailed through other classes is not visible to the schema";
const COMPLEMENT_EXPRESSION_REASON: &str = "the complement of a class expression whose projection \
     is approximate would reject conforming data, so no negation is projected";
const UNION_REASON: &str = "projected as anyOf over the members' property constraints, read \
     closed-world";
const UNION_ENTAILED_REASON: &str = "the class is a subclass of a member of the disjunction, so \
     the class hierarchy already entails it";
const UNION_NAMED_REASON: &str = "a disjunction with a named class member cannot be decided from \
     the focus node's own properties";
const UNION_PROPERTY_REASON: &str = "a disjunct restricts a property this class does not emit as \
     an ontology field, so the disjunction cannot be stated over the class's properties";
const UNION_MEMBER_REASON: &str =
    "a disjunct has no projection on the focus node, so the disjunction cannot be stated";
const DATA_RANGE_REASON: &str = "a data range in class position has no class projection";
const SOME_REASON: &str = "required, with one value in the filler: the projection reads OWL's \
     open-world existential closed-world";
const ALL_REASON: &str = "every value is held to the filler's value schema, as an rdfs:range is";
const ALL_APPROXIMATE_REASON: &str = "every value is held to the filler's value schema, which \
     admits more than the filler: a referenced node's class membership is not visible at the value";
const HAS_VALUE_REASON: &str = "required, with the value among the property's values: the \
     projection reads OWL's open-world existential closed-world";
const HAS_VALUE_ANONYMOUS_REASON: &str = "required; the anonymous individual has no stable @id, \
     so the value itself is not pinned";
const TRIVIAL_MIN_REASON: &str = "a minimum of zero constrains nothing";
const MIN_REASON: &str = "required, with at least the minimum number of values: the projection \
     reads OWL's open-world minimum closed-world";
const QUALIFIED_MIN_REASON: &str = "required, with at least the minimum number of values in the \
     qualifier: the projection reads OWL's open-world minimum closed-world";
const MAX_REASON: &str = "at most the maximum number of values: the projection counts distinct \
     terms, a unique-name reading of OWL's maximum";
const QUALIFIED_MAX_REASON: &str = "at most the maximum number of values in the qualifier: the \
     projection counts distinct terms, a unique-name reading of OWL's maximum";
const QUALIFIED_MAX_CLASS_REASON: &str = "counting the values in a class qualifier requires the \
     class membership of referenced nodes, which is not visible at the value, so the maximum is \
     not projected";
const EXACT_REASON: &str = "exactly the stated number of values: required, and counted over \
     distinct terms, a closed-world, unique-name reading of OWL's cardinality";
const QUALIFIED_EXACT_LOWER_REASON: &str = "the lower bound of the qualified cardinality is \
     projected as a required minimum, read closed-world";

/// The outcome of each part of one property restriction projected on a class
/// that emits the property.
pub(crate) fn restriction_outcomes(
    restriction: &Restriction,
    datatypes: &BTreeSet<String>,
) -> Vec<(SchemaExpressionOutcome, &'static str)> {
    use SchemaExpressionOutcome::{Approximated, Projected, Unrepresented};
    match restriction {
        Restriction::SomeValues(_) => vec![(Approximated, SOME_REASON)],
        Restriction::AllValues(filler) => match value_precision(filler, datatypes) {
            ValuePrecision::Exact | ValuePrecision::ClassLike => vec![(Projected, ALL_REASON)],
            ValuePrecision::Approximate => vec![(Approximated, ALL_APPROXIMATE_REASON)],
        },
        Restriction::HasValue(value) => {
            if value.is_anonymous() {
                vec![(Approximated, HAS_VALUE_ANONYMOUS_REASON)]
            } else {
                vec![(Approximated, HAS_VALUE_REASON)]
            }
        }
        Restriction::HasSelf => vec![(Unrepresented, HAS_SELF_REASON)],
        Restriction::Min(0, _) => vec![(Projected, TRIVIAL_MIN_REASON)],
        Restriction::Min(_, None) => vec![(Approximated, MIN_REASON)],
        Restriction::Min(_, Some(_)) => vec![(Approximated, QUALIFIED_MIN_REASON)],
        Restriction::Max(_, None) => vec![(Approximated, MAX_REASON)],
        Restriction::Max(_, Some(qualifier)) => {
            if projects_exactly(qualifier, datatypes) {
                vec![(Approximated, QUALIFIED_MAX_REASON)]
            } else {
                vec![(Unrepresented, QUALIFIED_MAX_CLASS_REASON)]
            }
        }
        Restriction::Exact(_, None) => vec![(Approximated, EXACT_REASON)],
        Restriction::Exact(count, Some(qualifier)) => {
            if projects_exactly(qualifier, datatypes) {
                vec![(Approximated, EXACT_REASON)]
            } else if *count == 0 {
                vec![(Unrepresented, QUALIFIED_MAX_CLASS_REASON)]
            } else {
                vec![
                    (Approximated, QUALIFIED_EXACT_LOWER_REASON),
                    (Unrepresented, QUALIFIED_MAX_CLASS_REASON),
                ]
            }
        }
    }
}

/// What one class's coverage decisions are, for classifying its inherited
/// conjuncts.
struct ConjunctContext<'c> {
    mode: SchemaSurfaceMode,
    class_iri: &'c str,
    /// The class's named supertypes, itself included.
    supertypes: &'c BTreeSet<String>,
    statuses: &'c BTreeMap<(String, String), SchemaCoverageStatus>,
    datatypes: &'c BTreeSet<String>,
}

impl ConjunctContext<'_> {
    fn status(&self, property_iri: &str) -> Option<SchemaCoverageStatus> {
        self.statuses
            .get(&(property_iri.to_owned(), self.class_iri.to_owned()))
            .copied()
    }

    /// Whether a disjunct is stated over the focus node's own properties, or
    /// the reason it is not.
    fn focus_projectable(&self, expression: &OntologyExpression) -> Result<(), &'static str> {
        match expression {
            OntologyExpression::Restriction(on, restriction) => {
                let Some(iri) = on.named() else {
                    return Err(UNION_MEMBER_REASON);
                };
                if self.status(iri) != Some(SchemaCoverageStatus::IncludedUnshaped) {
                    return Err(UNION_PROPERTY_REASON);
                }
                if restriction_outcomes(restriction, self.datatypes)
                    .iter()
                    .any(|(outcome, _)| *outcome == SchemaExpressionOutcome::Unrepresented)
                {
                    return Err(UNION_MEMBER_REASON);
                }
                Ok(())
            }
            OntologyExpression::Union(members) | OntologyExpression::Intersection(members) => {
                members
                    .iter()
                    .try_for_each(|member| self.focus_projectable(member))
            }
            OntologyExpression::OneOf(members) if members.iter().all(ExpressionTerm::is_named) => {
                Ok(())
            }
            OntologyExpression::Complement(inner)
                if matches!(**inner, OntologyExpression::Named(_)) =>
            {
                Ok(())
            }
            OntologyExpression::Named(_) => Err(UNION_NAMED_REASON),
            _ => Err(UNION_MEMBER_REASON),
        }
    }

    /// The components one inherited conjunct reports, each flagged when the
    /// conjunct is projected on the focus node (as a class-level schema
    /// constraint rather than through a property).
    fn classify(&self, conjunct: &OntologyExpression) -> Vec<(SchemaExpressionComponent, bool)> {
        use SchemaExpressionOutcome::{Approximated, Excluded, Projected, Unrepresented};
        let expression = conjunct.canonical();
        let restricted = match conjunct {
            OntologyExpression::Restriction(on, _) => on.named().map(str::to_owned),
            _ => None,
        };
        let component = |outcome, reason: &str| SchemaExpressionComponent {
            expression: expression.clone(),
            property_iri: restricted.clone(),
            outcome,
            reason: reason.to_owned(),
        };
        if self.mode == SchemaSurfaceMode::ShapedOnly {
            return vec![(component(Excluded, SHAPED_ONLY_REASON), false)];
        }
        match conjunct {
            OntologyExpression::Named(_) => vec![(component(Projected, HIERARCHY_REASON), false)],
            OntologyExpression::Restriction(on, restriction) => {
                let Some(iri) = on.named() else {
                    let reason = if matches!(on, RestrictedProperty::Many(_)) {
                        NARY_REASON
                    } else {
                        INVERSE_REASON
                    };
                    return vec![(component(Unrepresented, reason), false)];
                };
                let excluded = |reason| vec![(component(Excluded, reason), false)];
                match self.status(iri) {
                    Some(SchemaCoverageStatus::IncludedUnshaped) => {
                        restriction_outcomes(restriction, self.datatypes)
                            .into_iter()
                            .map(|(outcome, reason)| (component(outcome, reason), false))
                            .collect()
                    }
                    Some(SchemaCoverageStatus::HasShape) => excluded(HAS_SHAPE_REASON),
                    Some(SchemaCoverageStatus::ExcludedNamespace) => excluded(NAMESPACE_REASON),
                    Some(SchemaCoverageStatus::ExcludedDomain) => {
                        excluded(UNSATISFIED_SCOPE_REASON)
                    }
                    Some(SchemaCoverageStatus::ExcludedClosedShape) => excluded(CLOSED_REASON),
                    Some(SchemaCoverageStatus::ExcludedShapedOnly) | None => {
                        excluded(SHAPED_ONLY_REASON)
                    }
                }
            }
            OntologyExpression::OneOf(members) => {
                if members.iter().all(ExpressionTerm::is_named) {
                    vec![(component(Approximated, ONE_OF_REASON), true)]
                } else {
                    vec![(component(Unrepresented, ONE_OF_ANONYMOUS_REASON), false)]
                }
            }
            OntologyExpression::Complement(inner) => {
                if matches!(**inner, OntologyExpression::Named(_)) {
                    vec![(component(Approximated, COMPLEMENT_REASON), true)]
                } else {
                    vec![(
                        component(Unrepresented, COMPLEMENT_EXPRESSION_REASON),
                        false,
                    )]
                }
            }
            OntologyExpression::Union(members) => {
                let no_anonymous = AnonymousSupers::default();
                if members.iter().any(|member| {
                    member.is_named_skeleton()
                        && member.matches_class(self.supertypes, &no_anonymous)
                }) {
                    return vec![(component(Projected, UNION_ENTAILED_REASON), false)];
                }
                match members
                    .iter()
                    .try_for_each(|member| self.focus_projectable(member))
                {
                    Ok(()) => vec![(component(Approximated, UNION_REASON), true)],
                    Err(reason) => vec![(component(Unrepresented, reason), false)],
                }
            }
            OntologyExpression::Intersection(members) => members
                .iter()
                .flat_map(|member| self.classify(member))
                .collect(),
            OntologyExpression::DatatypeRestriction(..)
            | OntologyExpression::DatatypeComplement(_) => {
                vec![(component(Unrepresented, DATA_RANGE_REASON), false)]
            }
        }
    }
}

/// Classify every inherited conjunct of every eligible class, route the
/// class-level projections onto the surface, and assemble the manifest.
fn class_expression_report(
    mode: SchemaSurfaceMode,
    class_axioms: &[ClassAxiom],
    datatype_axioms: &[(SchemaCoverageProvenance, String)],
    class_facts: &BTreeMap<&str, ClassExpressionFacts<'_>>,
    statuses: &BTreeMap<(String, String), SchemaCoverageStatus>,
    supertypes: &BTreeMap<String, BTreeSet<String>>,
    classes: &mut BTreeMap<String, SurfaceClass>,
    datatypes: &BTreeSet<String>,
) -> Result<SchemaClassExpressionReport, SchemaCompileError> {
    let no_supertypes = BTreeSet::new();
    let mut per_axiom: Vec<BTreeMap<&str, BTreeSet<SchemaExpressionComponent>>> =
        vec![BTreeMap::new(); class_axioms.len()];
    let mut cells = 0_usize;
    for (&class_iri, facts) in class_facts {
        let context = ConjunctContext {
            mode,
            class_iri,
            supertypes: supertypes.get(class_iri).unwrap_or(&no_supertypes),
            statuses,
            datatypes,
        };
        let mut focus: BTreeSet<OntologyExpression> = BTreeSet::new();
        let mut unrepresented: BTreeSet<String> = BTreeSet::new();
        for &(axiom, conjunct) in &facts.entries {
            for (component, on_focus) in context.classify(conjunct) {
                if on_focus {
                    focus.insert(conjunct.clone());
                }
                if component.outcome == SchemaExpressionOutcome::Unrepresented {
                    unrepresented.insert(format!("{}: {}", component.expression, component.reason));
                }
                cells = cells.saturating_add(1);
                enforce_limit(
                    "class-expression coverage cells",
                    cells,
                    MAX_SCHEMA_RELATIONS,
                )?;
                per_axiom[axiom]
                    .entry(class_iri)
                    .or_default()
                    .insert(component);
            }
        }
        if let Some(class) = classes.get_mut(class_iri) {
            class.focus = focus.into_iter().collect();
            class.unrepresented = unrepresented.into_iter().collect();
        }
    }

    let mut axioms: BTreeMap<SchemaCoverageProvenance, SchemaClassExpressionAxiom> =
        BTreeMap::new();
    for (axiom, rows) in class_axioms.iter().zip(per_axiom) {
        let entry =
            axioms
                .entry(axiom.provenance.clone())
                .or_insert_with(|| SchemaClassExpressionAxiom {
                    provenance: axiom.provenance.clone(),
                    components: Vec::new(),
                    classes: Vec::new(),
                });
        entry.components.extend(axiom.uncarried.iter().cloned());
        for (class_iri, components) in rows {
            entry.classes.push(SchemaClassExpressionCoverage {
                class_iri: class_iri.to_owned(),
                components: components.into_iter().collect(),
            });
        }
    }
    for (provenance, range) in datatype_axioms {
        let (outcome, reason) = if mode == SchemaSurfaceMode::ShapedOnly {
            (SchemaExpressionOutcome::Excluded, SHAPED_ONLY_REASON)
        } else {
            (
                SchemaExpressionOutcome::Approximated,
                DATATYPE_DEFINITION_REASON,
            )
        };
        axioms
            .entry(provenance.clone())
            .or_insert_with(|| SchemaClassExpressionAxiom {
                provenance: provenance.clone(),
                components: Vec::new(),
                classes: Vec::new(),
            })
            .components
            .push(SchemaExpressionComponent {
                expression: range.clone(),
                property_iri: None,
                outcome,
                reason: reason.to_owned(),
            });
    }
    let mut axioms: Vec<SchemaClassExpressionAxiom> = axioms.into_values().collect();
    for axiom in &mut axioms {
        axiom.components.sort();
        axiom.components.dedup();
        axiom
            .classes
            .sort_by(|left, right| left.class_iri.cmp(&right.class_iri));
        let mut merged: Vec<SchemaClassExpressionCoverage> =
            Vec::with_capacity(axiom.classes.len());
        for class in std::mem::take(&mut axiom.classes) {
            match merged.last_mut() {
                Some(last) if last.class_iri == class.class_iri => {
                    last.components.extend(class.components);
                    last.components.sort();
                    last.components.dedup();
                }
                _ => merged.push(class),
            }
        }
        axiom.classes = merged;
        if axiom.classes.is_empty() && axiom.components.is_empty() {
            axiom.components.push(SchemaExpressionComponent {
                expression: axiom.provenance.object.clone(),
                property_iri: None,
                outcome: SchemaExpressionOutcome::Excluded,
                reason: UNCARRIED_REASON.to_owned(),
            });
        }
    }
    Ok(SchemaClassExpressionReport { mode, axioms })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json_schema::Namespaces;
    use crate::shapes::from_dataset;

    const PREFIXES: &str = r"
        @prefix ex: <https://example.org/schema/> .
        @prefix ext: <https://external.example/vocab/> .
        @prefix sh: <http://www.w3.org/ns/shacl#> .
        @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> .
        @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
        @prefix owl: <http://www.w3.org/2002/07/owl#> .
        @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
    ";

    fn namespaces() -> Namespaces {
        Namespaces::new(
            "ex",
            &[("ex".to_owned(), "https://example.org/schema/".to_owned())],
        )
        .expect("test namespace")
    }

    fn surface(
        shapes_body: &str,
        ontology_body: &str,
        mode: SchemaSurfaceMode,
    ) -> Result<SchemaSurface, SchemaCompileError> {
        let shape_dataset =
            crate::text_ingest::parse_turtle_to_dataset(&format!("{PREFIXES}{shapes_body}"), None)
                .expect("shape Turtle");
        let shapes = from_dataset(&shape_dataset).expect("shape graph");
        let ontology = crate::text_ingest::parse_turtle_to_dataset(
            &format!("{PREFIXES}{ontology_body}"),
            None,
        )
        .expect("ontology Turtle");
        build(&SchemaCompileRequest::new(
            &shapes,
            &namespaces(),
            ontology.as_ref(),
            mode,
        ))
    }

    fn property<'a>(surface: &'a SchemaSurface, iri: &str) -> &'a SchemaPropertyCoverage {
        surface
            .report
            .properties
            .iter()
            .find(|property| property.property_iri == iri)
            .expect("catalogued property")
    }

    fn class_status(
        surface: &SchemaSurface,
        property_iri: &str,
        class_iri: &str,
    ) -> SchemaCoverageStatus {
        property(surface, property_iri)
            .classes
            .iter()
            .find(|row| row.class_iri == class_iri)
            .expect("class coverage row")
            .status
    }

    #[test]
    fn full_surface_catalogs_only_schema_properties_and_applies_domains() {
        let surface = surface(
            r"
                ex:PersonShape a sh:NodeShape ;
                    sh:targetClass ex:Person ;
                    sh:property [ sh:path ex:name ; sh:datatype xsd:string ] .
            ",
            r#"
                ex:Agent a owl:Class .
                ex:Person a owl:Class ; rdfs:subClassOf ex:Agent .
                ex:Message a owl:Class .

                ex:resentDate a owl:DatatypeProperty ;
                    rdfs:domain ex:Message ; rdfs:range xsd:dateTime .
                ex:identifier a owl:DatatypeProperty, owl:FunctionalProperty ;
                    rdfs:domain ex:Agent ; rdfs:range rdfs:Literal .
                ex:tag a rdf:Property .
                ext:externalOnly a owl:DatatypeProperty ; rdfs:range xsd:string .

                ex:instance ex:incidental "not a declaration" .
            "#,
            SchemaSurfaceMode::OntologyComplete,
        )
        .expect("complete surface");

        let catalog: BTreeSet<&str> = surface
            .report
            .properties
            .iter()
            .map(|property| property.property_iri.as_str())
            .collect();
        assert!(catalog.contains("https://example.org/schema/resentDate"));
        assert!(catalog.contains("https://example.org/schema/identifier"));
        assert!(catalog.contains("https://example.org/schema/tag"));
        assert!(catalog.contains("https://example.org/schema/name"));
        assert!(catalog.contains("https://external.example/vocab/externalOnly"));
        assert!(!catalog.contains("https://example.org/schema/incidental"));

        assert_eq!(
            class_status(
                &surface,
                "https://example.org/schema/resentDate",
                "https://example.org/schema/Message"
            ),
            SchemaCoverageStatus::IncludedUnshaped
        );
        assert_eq!(
            class_status(
                &surface,
                "https://example.org/schema/resentDate",
                "https://example.org/schema/Person"
            ),
            SchemaCoverageStatus::ExcludedDomain
        );
        assert_eq!(
            class_status(
                &surface,
                "https://example.org/schema/identifier",
                "https://example.org/schema/Person"
            ),
            SchemaCoverageStatus::IncludedUnshaped,
            "subclasses satisfy inherited domain membership"
        );
        assert_eq!(
            class_status(
                &surface,
                "https://example.org/schema/name",
                "https://example.org/schema/Person"
            ),
            SchemaCoverageStatus::HasShape
        );
        assert_eq!(
            property(&surface, "https://external.example/vocab/externalOnly").outcomes,
            vec![SchemaCoverageStatus::ExcludedNamespace]
        );

        assert!(!surface.classes["https://example.org/schema/Person"].synthesized_open);
        assert!(surface.classes["https://example.org/schema/Agent"].synthesized_open);
        assert!(surface.classes["https://example.org/schema/Message"].synthesized_open);
        let identifier = &surface.classes["https://example.org/schema/Person"].properties["https://example.org/schema/identifier"];
        assert!(identifier.functional);
        assert_eq!(identifier.kind, OntologyPropertyKind::Datatype);
    }

    #[test]
    fn shaped_only_reports_exclusion_and_creates_no_carrier_classes() {
        let surface = surface(
            r"
                ex:PersonShape a sh:NodeShape ; sh:targetClass ex:Person ;
                    sh:property [ sh:path ex:name ] .
            ",
            r"
                ex:Person a owl:Class .
                ex:Message a owl:Class .
                ex:resentMessageId a owl:DatatypeProperty ;
                    rdfs:domain ex:Message ; rdfs:range rdfs:Literal .
            ",
            SchemaSurfaceMode::ShapedOnly,
        )
        .expect("shaped surface");
        assert_eq!(
            surface.classes.keys().cloned().collect::<Vec<_>>(),
            vec!["https://example.org/schema/Person"]
        );
        assert_eq!(
            property(&surface, "https://example.org/schema/resentMessageId").outcomes,
            vec![SchemaCoverageStatus::ExcludedShapedOnly]
        );
    }

    #[test]
    fn closed_shape_excludes_unshaped_property_but_ignored_property_is_admitted() {
        let surface = surface(
            r"
                ex:PersonShape a sh:NodeShape ; sh:targetClass ex:Person ;
                    sh:closed true ; sh:ignoredProperties ( ex:allowed ) ;
                    sh:property [ sh:path ex:name ] .
            ",
            r"
                ex:Person a owl:Class .
                ex:blocked a owl:DatatypeProperty ; rdfs:domain ex:Person .
                ex:allowed a owl:DatatypeProperty ; rdfs:domain ex:Person .
            ",
            SchemaSurfaceMode::OntologyComplete,
        )
        .expect("closed surface");
        assert_eq!(
            class_status(
                &surface,
                "https://example.org/schema/blocked",
                "https://example.org/schema/Person"
            ),
            SchemaCoverageStatus::ExcludedClosedShape
        );
        assert_eq!(
            class_status(
                &surface,
                "https://example.org/schema/allowed",
                "https://example.org/schema/Person"
            ),
            SchemaCoverageStatus::IncludedUnshaped
        );
    }

    #[test]
    fn subproperty_equivalence_and_inverse_propagate_in_defined_directions() {
        let surface = surface(
            "",
            r"
                ex:Agent a owl:Class .
                ex:Document a owl:Class .
                ex:parent a owl:ObjectProperty, owl:FunctionalProperty ;
                    rdfs:domain ex:Agent ; rdfs:range ex:Document .
                ex:child a owl:ObjectProperty ; rdfs:subPropertyOf ex:parent .
                ex:alias a owl:ObjectProperty ; owl:equivalentProperty ex:child .
                ex:inverse a owl:ObjectProperty ; owl:inverseOf ex:parent .
                ex:reverseUnique a owl:InverseFunctionalProperty ;
                    rdfs:domain ex:Agent ; rdfs:range ex:Document .
            ",
            SchemaSurfaceMode::OntologyComplete,
        )
        .expect("relation surface");
        for property_iri in [
            "https://example.org/schema/parent",
            "https://example.org/schema/child",
            "https://example.org/schema/alias",
        ] {
            assert!(
                surface.classes["https://example.org/schema/Agent"].properties[property_iri]
                    .functional
            );
        }
        assert!(
            surface.classes["https://example.org/schema/Document"]
                .properties
                .contains_key("https://example.org/schema/inverse"),
            "inverse swaps the parent's range into its domain"
        );
        assert!(
            !surface.classes["https://example.org/schema/Agent"].properties
                ["https://example.org/schema/reverseUnique"]
                .functional,
            "inverse-functional does not select a scalar value representation"
        );
    }

    #[test]
    fn multiple_domains_are_conjunctive_and_union_domain_is_disjunctive() {
        let surface = surface(
            "",
            r"
                ex:A a owl:Class .
                ex:B a owl:Class .
                ex:AB a owl:Class ; rdfs:subClassOf ex:A, ex:B .
                ex:both a rdf:Property ; rdfs:domain ex:A, ex:B .
                ex:either a rdf:Property ; rdfs:domain [
                    owl:unionOf ( ex:A ex:B )
                ] .
            ",
            SchemaSurfaceMode::OntologyComplete,
        )
        .expect("expression surface");
        assert_eq!(
            class_status(
                &surface,
                "https://example.org/schema/both",
                "https://example.org/schema/A"
            ),
            SchemaCoverageStatus::ExcludedDomain
        );
        assert_eq!(
            class_status(
                &surface,
                "https://example.org/schema/both",
                "https://example.org/schema/AB"
            ),
            SchemaCoverageStatus::IncludedUnshaped
        );
        for class in ["A", "B", "AB"] {
            assert_eq!(
                class_status(
                    &surface,
                    "https://example.org/schema/either",
                    &format!("https://example.org/schema/{class}")
                ),
                SchemaCoverageStatus::IncludedUnshaped
            );
        }
    }

    #[test]
    fn hierarchy_cycles_are_legal_and_preserve_domain_membership() {
        let surface = surface(
            "",
            r"
                ex:A a owl:Class ; rdfs:subClassOf ex:B .
                ex:B a owl:Class ; rdfs:subClassOf ex:A .
                ex:p a rdf:Property ; rdfs:domain ex:A .
            ",
            SchemaSurfaceMode::OntologyComplete,
        )
        .expect("cyclic hierarchy");
        assert_eq!(
            class_status(
                &surface,
                "https://example.org/schema/p",
                "https://example.org/schema/B"
            ),
            SchemaCoverageStatus::IncludedUnshaped
        );
    }

    #[test]
    fn malformed_and_cyclic_expression_lists_fail_with_typed_errors() {
        let malformed = surface(
            "",
            r"
                ex:p a rdf:Property ; rdfs:domain [ owl:unionOf _:list ] .
                _:list rdf:first ex:A ; rdf:first ex:B ; rdf:rest rdf:nil .
            ",
            SchemaSurfaceMode::OntologyComplete,
        )
        .expect_err("multiple rdf:first values must fail");
        assert!(matches!(
            malformed,
            SchemaCompileError::InvalidOntology { .. }
        ));

        let cyclic = surface(
            "",
            r"
                ex:p a rdf:Property ; rdfs:domain [ owl:intersectionOf _:list ] .
                _:list rdf:first ex:A ; rdf:rest _:tail .
                _:tail rdf:first ex:B ; rdf:rest _:list .
            ",
            SchemaSurfaceMode::OntologyComplete,
        )
        .expect_err("cyclic RDF list must fail");
        assert!(matches!(cyclic, SchemaCompileError::InvalidOntology { .. }));
    }

    #[test]
    fn ontology_and_shapes_blank_nodes_are_standardized_apart() {
        let surface = surface(
            r"
                _:shared rdf:first ex:ShapeOnly ; rdf:rest rdf:nil .
            ",
            r"
                ex:A a owl:Class .
                ex:B a owl:Class .
                ex:Holder a owl:Class .
                ex:choice a owl:ObjectProperty ;
                    rdfs:domain ex:Holder ;
                    rdfs:range [ owl:unionOf _:shared ] .
                _:shared rdf:first ex:A ; rdf:rest _:tail .
                _:tail rdf:first ex:B ; rdf:rest rdf:nil .
            ",
            SchemaSurfaceMode::OntologyComplete,
        )
        .expect("same-label blanks from separate datasets remain independent");

        assert_eq!(
            surface.classes["https://example.org/schema/Holder"].properties
                ["https://example.org/schema/choice"]
                .ranges,
            vec![OntologyExpression::Union(vec![
                OntologyExpression::Named("https://example.org/schema/A".to_owned()),
                OntologyExpression::Named("https://example.org/schema/B".to_owned()),
            ])]
        );
    }

    #[test]
    fn propagation_and_coverage_preflights_enforce_exact_small_limits() {
        let chain = vec![
            BTreeSet::from([1_usize]),
            BTreeSet::from([2_usize]),
            BTreeSet::new(),
        ];
        let seeds = vec![
            BTreeSet::from([0_u8]),
            BTreeSet::from([1_u8]),
            BTreeSet::from([2_u8]),
        ];
        let transitive = propagate_sets_with_limit(&chain, seeds, "test facts", 5)
            .expect_err("transitive extension must stop at the limit");
        assert!(matches!(
            transitive,
            SchemaCompileError::LimitExceeded {
                resource: "test facts",
                limit: 5,
                observed: 6
            }
        ));

        let cycle = vec![
            BTreeSet::from([1_usize]),
            BTreeSet::from([2_usize]),
            BTreeSet::from([0_usize]),
        ];
        let seeds = vec![
            BTreeSet::from([0_u8]),
            BTreeSet::from([1_u8]),
            BTreeSet::from([2_u8]),
        ];
        let output = propagate_sets_with_limit(&cycle, seeds, "test facts", 8)
            .expect_err("per-node output cloning must stop at the limit");
        assert!(matches!(
            output,
            SchemaCompileError::LimitExceeded {
                resource: "test facts",
                limit: 8,
                observed: 9
            }
        ));

        let coverage = coverage_cell_count(2, 3, 1, 6)
            .expect_err("external shaped rows count toward coverage");
        assert!(matches!(
            coverage,
            SchemaCompileError::LimitExceeded {
                resource: "class/property coverage cells",
                limit: 6,
                observed: 7
            }
        ));
    }

    #[test]
    fn incompatible_property_kind_and_range_fail_with_typed_error() {
        let error = surface(
            "",
            r"
                ex:Person a owl:Class .
                ex:p a owl:DatatypeProperty ; rdfs:range ex:Person .
            ",
            SchemaSurfaceMode::OntologyComplete,
        )
        .expect_err("datatype property with class range must fail");
        assert!(matches!(
            error,
            SchemaCompileError::InvalidOntology { subject, .. }
                if subject == "https://example.org/schema/p"
        ));
    }

    #[test]
    fn report_and_surface_are_permutation_deterministic_and_conservative() {
        let first = surface(
            "",
            r"
                ex:C a owl:Class .
                ex:b a owl:DatatypeProperty ; rdfs:domain ex:C ; rdfs:range xsd:string .
                ex:a a owl:DatatypeProperty ; rdfs:domain ex:C ; rdfs:range xsd:dateTime .
            ",
            SchemaSurfaceMode::OntologyComplete,
        )
        .expect("first surface");
        let second = surface(
            "",
            r"
                ex:a rdfs:range xsd:dateTime ; rdfs:domain ex:C ; a owl:DatatypeProperty .
                ex:b rdfs:range xsd:string ; a owl:DatatypeProperty ; rdfs:domain ex:C .
                ex:C a owl:Class .
            ",
            SchemaSurfaceMode::OntologyComplete,
        )
        .expect("permuted surface");
        assert_eq!(first.report.to_json(), second.report.to_json());
        let catalog: Vec<&str> = first
            .report
            .properties
            .iter()
            .map(|property| property.property_iri.as_str())
            .collect();
        assert_eq!(
            catalog.len(),
            catalog.iter().copied().collect::<BTreeSet<_>>().len()
        );
        assert_eq!(first.classes.len(), second.classes.len());
    }

    fn complete(ontology_body: &str) -> Result<SchemaSurface, SchemaCompileError> {
        surface("", ontology_body, SchemaSurfaceMode::OntologyComplete)
    }

    /// Assert that `invalid` is refused with a typed `InvalidOntology` whose
    /// reason contains `needle`, and that its valid neighbour is accepted.
    fn refusal_with_neighbour(invalid: &str, needle: &str, valid: &str) {
        let error = complete(invalid).expect_err("malformed input must be refused");
        match &error {
            SchemaCompileError::InvalidOntology { reason, .. } => assert!(
                reason.contains(needle),
                "refusal reason {reason:?} does not name {needle:?}"
            ),
            other => panic!("expected InvalidOntology, got {other:?}"),
        }
        complete(valid).expect("the valid neighbour of a refusal must be accepted");
    }

    #[test]
    fn restriction_without_on_property_is_refused_and_with_it_accepted() {
        refusal_with_neighbour(
            "ex:A rdfs:subClassOf [ a owl:Restriction ; owl:someValuesFrom ex:B ] .",
            "declares no owl:onProperty",
            "ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:p ; owl:someValuesFrom ex:B ] .",
        );
    }

    #[test]
    fn conflicting_cardinality_values_are_refused_and_separate_restrictions_accepted() {
        refusal_with_neighbour(
            "ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:p ;
                owl:minCardinality 1 , 2 ] .",
            "conflicting values",
            "ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:p ; owl:minCardinality 1 ] ,
                [ a owl:Restriction ; owl:onProperty ex:p ; owl:minCardinality 2 ] .",
        );
    }

    #[test]
    fn several_facets_on_one_restriction_node_read_as_their_conjunction() {
        let surface = complete(
            "ex:A a owl:Class ; rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:p ;
                owl:minCardinality 1 ; owl:maxCardinality 3 ] .
             ex:p a owl:DatatypeProperty .",
        )
        .expect("an OWL 1 multi-facet restriction is accepted");
        assert_eq!(
            surface.classes["https://example.org/schema/A"].properties
                ["https://example.org/schema/p"]
                .restrictions
                .len(),
            2
        );
    }

    #[test]
    fn ill_typed_cardinalities_are_refused_and_integers_accepted() {
        for (invalid, needle) in [
            ("owl:minCardinality -1", "non-negative integer"),
            ("owl:minCardinality \"1\"", "non-negative integer literal"),
            ("owl:minCardinality 1.0", "non-negative integer literal"),
            ("owl:minCardinality ex:one", "non-negative integer literal"),
            (
                "owl:minCardinality 99999999999999999999999",
                "no larger than",
            ),
        ] {
            refusal_with_neighbour(
                &format!(
                    "ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:p ; {invalid} ] ."
                ),
                needle,
                "ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:p ; owl:minCardinality 0 ] .",
            );
        }
        for valid in [
            "owl:minCardinality \"2\"^^xsd:nonNegativeInteger",
            "owl:minCardinality \"+2\"^^xsd:positiveInteger",
            "owl:minCardinality \"-0\"^^xsd:integer",
            "owl:minCardinality \"7\"^^xsd:unsignedByte",
        ] {
            complete(&format!(
                "ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:p ; {valid} ] ."
            ))
            .unwrap_or_else(|error| panic!("{valid} must be accepted: {error}"));
        }
    }

    #[test]
    fn malformed_and_cyclic_superclass_lists_are_refused_and_proper_lists_accepted() {
        refusal_with_neighbour(
            "ex:A rdfs:subClassOf [ owl:unionOf _:list ] .
             _:list rdf:first ex:B ; rdf:rest _:tail .
             _:tail rdf:first ex:C ; rdf:rest _:list .",
            "OWL expression list",
            "ex:A rdfs:subClassOf [ owl:unionOf ( ex:B ex:C ) ] .",
        );
        refusal_with_neighbour(
            "ex:A rdfs:subClassOf [ owl:intersectionOf _:list ] .
             _:list rdf:first ex:B , ex:C ; rdf:rest rdf:nil .",
            "OWL expression list",
            "ex:A rdfs:subClassOf [ owl:intersectionOf ( ex:B ex:C ) ] .",
        );
        refusal_with_neighbour(
            "ex:A rdfs:subClassOf [ owl:oneOf ex:notAList ] .",
            "OWL expression list",
            "ex:A rdfs:subClassOf [ owl:oneOf ( ex:a ) ] .",
        );
    }

    #[test]
    fn self_containing_expression_is_refused_and_shared_subexpression_accepted() {
        refusal_with_neighbour(
            "ex:A rdfs:subClassOf _:x . _:x owl:complementOf _:x .",
            "contains itself",
            "ex:A rdfs:subClassOf [ owl:unionOf ( _:shared [ owl:complementOf _:shared ] ) ] .
             _:shared a owl:Restriction ; owl:onProperty ex:p ; owl:someValuesFrom ex:B .",
        );
    }

    #[test]
    fn qualifier_mismatches_are_refused_and_matched_qualifiers_accepted() {
        let valid = "ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:p ;
                owl:minQualifiedCardinality 1 ; owl:onClass ex:B ] .";
        refusal_with_neighbour(
            "ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:p ;
                owl:minQualifiedCardinality 1 ] .",
            "requires an owl:onClass or owl:onDataRange qualifier",
            valid,
        );
        refusal_with_neighbour(
            "ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:p ;
                owl:someValuesFrom ex:B ; owl:onClass ex:B ] .",
            "qualifies no qualified cardinality",
            valid,
        );
        refusal_with_neighbour(
            "ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:p ;
                owl:minQualifiedCardinality 1 ; owl:onClass ex:B ; owl:onDataRange xsd:string ] .",
            "at most one owl:onClass or owl:onDataRange",
            valid,
        );
    }

    #[test]
    fn has_self_false_is_refused_and_true_accepted() {
        refusal_with_neighbour(
            "ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:p ; owl:hasSelf false ] .",
            "owl:hasSelf requires the literal true",
            "ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:p ; owl:hasSelf true ] .",
        );
    }

    #[test]
    fn constructless_and_mixed_expressions_are_refused_and_single_constructs_accepted() {
        refusal_with_neighbour(
            "ex:A rdfs:subClassOf [ a owl:Class ] .",
            "declares no class-expression",
            "ex:A rdfs:subClassOf [ a owl:Class ; owl:complementOf ex:B ] .",
        );
        refusal_with_neighbour(
            "ex:A rdfs:subClassOf [ owl:unionOf ( ex:B ex:C ) ; owl:onProperty ex:p ;
                owl:someValuesFrom ex:B ] .",
            "mixes owl:unionOf and an owl:Restriction",
            "ex:A rdfs:subClassOf [ owl:unionOf ( ex:B ex:C ) ] .",
        );
        refusal_with_neighbour(
            "ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:p ] .",
            "declares no constraint",
            "ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:p ; owl:hasValue ex:v ] .",
        );
    }

    #[test]
    fn data_range_in_class_position_is_refused_and_as_a_filler_accepted() {
        refusal_with_neighbour(
            "ex:A rdfs:subClassOf [ a rdfs:Datatype ; owl:onDatatype xsd:integer ;
                owl:withRestrictions ( [ xsd:minInclusive 0 ] ) ] .",
            "stands where a class expression is required",
            "ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:p ; owl:allValuesFrom
                [ a rdfs:Datatype ; owl:onDatatype xsd:integer ;
                  owl:withRestrictions ( [ xsd:minInclusive 0 ] ) ] ] .",
        );
        refusal_with_neighbour(
            "ex:A rdfs:subClassOf [ owl:oneOf ( \"a\" \"b\" ) ] .",
            "stands where a class expression is required",
            "ex:A rdfs:subClassOf [ owl:oneOf ( ex:a ex:b ) ] .",
        );
        refusal_with_neighbour(
            "ex:A rdfs:subClassOf [ owl:oneOf ( ex:a \"b\" ) ] .",
            "mixes individuals and literals",
            "ex:A rdfs:subClassOf [ owl:oneOf ( ex:a ex:b ) ] .",
        );
    }

    #[test]
    fn ill_typed_restriction_fillers_are_refused_and_well_typed_accepted() {
        refusal_with_neighbour(
            "ex:B a owl:Class .
             ex:p a owl:DatatypeProperty .
             ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:p ; owl:someValuesFrom ex:B ] .",
            "owl:DatatypeProperty is restricted to the class expression",
            "ex:B a owl:Class .
             ex:p a owl:DatatypeProperty .
             ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:p ; owl:someValuesFrom xsd:string ] .",
        );
        refusal_with_neighbour(
            "ex:q a owl:ObjectProperty .
             ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:q ; owl:hasValue \"v\" ] .",
            "owl:ObjectProperty has the literal",
            "ex:q a owl:ObjectProperty .
             ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:q ; owl:hasValue ex:v ] .",
        );
        refusal_with_neighbour(
            "ex:p a owl:DatatypeProperty .
             ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:p ; owl:hasSelf true ] .",
            "owl:hasSelf restricts an owl:DatatypeProperty",
            "ex:p a owl:ObjectProperty .
             ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:p ; owl:hasSelf true ] .",
        );
    }

    #[test]
    fn n_ary_restriction_accepts_only_some_and_all() {
        refusal_with_neighbour(
            "ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperties ( ex:p ex:q ) ;
                owl:minCardinality 1 ] .",
            "defined only with owl:someValuesFrom or owl:allValuesFrom",
            "ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperties ( ex:p ex:q ) ;
                owl:someValuesFrom xsd:string ] .",
        );
    }

    #[test]
    fn anonymous_property_expressions_must_be_inverses() {
        refusal_with_neighbour(
            "ex:p rdfs:subPropertyOf [ rdfs:label \"no inverse\" ] .",
            "must be owl:inverseOf exactly one named property",
            "ex:p rdfs:subPropertyOf [ owl:inverseOf ex:q ] .",
        );
    }

    #[test]
    fn inverse_property_expressions_propagate_like_inverse_of() {
        let surface = complete(
            "ex:Agent a owl:Class .
             ex:Document a owl:Class .
             ex:author a owl:ObjectProperty ; rdfs:domain ex:Document ; rdfs:range ex:Agent .
             ex:wrote a owl:ObjectProperty ; owl:equivalentProperty [ owl:inverseOf ex:author ] .
             ex:penned a owl:ObjectProperty ; rdfs:subPropertyOf [ owl:inverseOf ex:author ] .
             [ owl:inverseOf ex:author ] rdfs:subPropertyOf ex:creatorOf .
             ex:creatorOf a owl:ObjectProperty .",
        )
        .expect("inverse property expressions are accepted");
        for property in ["wrote", "penned"] {
            assert_eq!(
                class_status(
                    &surface,
                    &format!("https://example.org/schema/{property}"),
                    "https://example.org/schema/Agent"
                ),
                SchemaCoverageStatus::IncludedUnshaped,
                "the inverse of author has author's range as its domain"
            );
            assert_eq!(
                class_status(
                    &surface,
                    &format!("https://example.org/schema/{property}"),
                    "https://example.org/schema/Document"
                ),
                SchemaCoverageStatus::ExcludedDomain
            );
        }
        let wrote = property(&surface, "https://example.org/schema/wrote");
        assert!(wrote.classes[0].provenance.iter().any(|provenance| {
            provenance.object == "inverse(<https://example.org/schema/author>)"
        }));
    }

    /// A chain of `levels` unions, each referencing the next level twice, so
    /// the expanded tree doubles per level while the graph grows linearly.
    fn doubling_chain(levels: usize) -> String {
        use std::fmt::Write as _;

        let mut turtle = String::from("ex:Root rdfs:subClassOf _:u0 .\n");
        for level in 0..levels {
            let next = level + 1;
            let _ = writeln!(
                turtle,
                "_:u{level} owl:unionOf ( _:u{next} [ owl:complementOf _:u{next} ] ) ."
            );
        }
        let _ = writeln!(turtle, "_:u{levels} owl:complementOf ex:Leaf .");
        turtle
    }

    #[test]
    fn expansion_budget_bounds_shared_subexpressions() {
        let dataset = crate::text_ingest::parse_turtle_to_dataset(
            &format!("{PREFIXES}{}", doubling_chain(24)),
            None,
        )
        .expect("Turtle");
        let root = objects_of(
            dataset.as_ref(),
            &Term::NamedNode(NamedNode::from("https://example.org/schema/Root")),
            rdfs::SUB_CLASS_OF,
        )
        .pop()
        .expect("root superclass");
        let error = ExpressionReader::with_budget(dataset.as_ref(), 4_096)
            .expression(&root, 0)
            .expect_err("an exponentially shared expression exhausts the budget");
        assert!(matches!(
            error,
            SchemaCompileError::LimitExceeded {
                resource: "OWL expression nodes",
                limit: 4_096,
                ..
            }
        ));
        // The valid neighbour: the same structure eight levels deep expands
        // within the budget.
        let shallow = crate::text_ingest::parse_turtle_to_dataset(
            &format!("{PREFIXES}{}", doubling_chain(8)),
            None,
        )
        .expect("Turtle");
        let root = objects_of(
            shallow.as_ref(),
            &Term::NamedNode(NamedNode::from("https://example.org/schema/Root")),
            rdfs::SUB_CLASS_OF,
        )
        .pop()
        .expect("root superclass");
        ExpressionReader::with_budget(shallow.as_ref(), 4_096)
            .expression(&root, 0)
            .expect("a shallow shared expression expands within the budget");
        complete(&doubling_chain(8)).expect("and the whole surface accepts it");
    }

    #[test]
    fn anonymous_superclasses_reach_subclasses_and_drive_domains() {
        let surface = complete(
            "ex:Person a owl:Class ; rdfs:subClassOf
                 [ a owl:Restriction ; owl:onProperty ex:name ; owl:someValuesFrom xsd:string ] .
             ex:Student a owl:Class ; rdfs:subClassOf ex:Person .
             ex:Robot a owl:Class .
             ex:name a owl:DatatypeProperty ; rdfs:domain ex:Robot ; rdfs:range xsd:string .
             ex:Party a owl:Class ;
                 owl:equivalentClass [ owl:unionOf ( ex:Person ex:Organization ) ] .
             ex:Organization a owl:Class .
             ex:member a owl:ObjectProperty ; rdfs:domain ex:Party .
             ex:tagged a owl:ObjectProperty ;
                 rdfs:domain [ a owl:Restriction ; owl:onProperty ex:name ;
                               owl:someValuesFrom xsd:string ] .",
        )
        .expect("anonymous superclasses are modelled");
        for class in ["Person", "Student"] {
            let class_iri = format!("https://example.org/schema/{class}");
            assert_eq!(
                class_status(&surface, "https://example.org/schema/name", &class_iri),
                SchemaCoverageStatus::IncludedUnshaped,
                "an existential restriction entails the property's domain"
            );
            assert_eq!(
                surface.classes[&class_iri].properties["https://example.org/schema/name"]
                    .restrictions,
                vec![Restriction::SomeValues(OntologyExpression::Named(
                    "http://www.w3.org/2001/XMLSchema#string".to_owned()
                ))]
            );
            assert_eq!(
                class_status(&surface, "https://example.org/schema/member", &class_iri),
                SchemaCoverageStatus::IncludedUnshaped,
                "a union equivalence makes each named member a subclass"
            );
            assert_eq!(
                class_status(&surface, "https://example.org/schema/tagged", &class_iri),
                SchemaCoverageStatus::IncludedUnshaped,
                "a restriction domain matches a class asserted to be a subclass of it"
            );
        }
        let robot_tagged = property(&surface, "https://example.org/schema/tagged")
            .classes
            .iter()
            .find(|row| row.class_iri == "https://example.org/schema/Robot")
            .expect("Robot row");
        assert_eq!(robot_tagged.status, SchemaCoverageStatus::ExcludedDomain);
        assert_eq!(
            robot_tagged.precision,
            SchemaCoveragePrecision::RepresentationApproximation,
            "an exclusion against a structural domain is not a proof"
        );
        let name = property(&surface, "https://example.org/schema/name");
        assert!(
            name.declarations
                .iter()
                .any(|declaration| declaration == OWL_ON_PROPERTY)
        );
        let person_row = name
            .classes
            .iter()
            .find(|row| row.class_iri == "https://example.org/schema/Person")
            .expect("Person row");
        assert_eq!(
            person_row.precision,
            SchemaCoveragePrecision::RepresentationApproximation
        );
        assert!(person_row.provenance.iter().any(|provenance| {
            provenance.subject == "https://example.org/schema/Person"
                && provenance.predicate == rdfs::SUB_CLASS_OF
                && provenance.object
                    == "some(<https://example.org/schema/name>,<http://www.w3.org/2001/XMLSchema#string>)"
        }));
    }

    #[test]
    fn class_expression_report_is_permutation_deterministic() {
        let first = complete(
            "ex:A a owl:Class ; rdfs:subClassOf
                 [ a owl:Restriction ; owl:onProperty ex:p ; owl:hasValue [ rdfs:label \"x\" ] ] ,
                 [ owl:unionOf ( [ a owl:Restriction ; owl:onProperty ex:p ; owl:minCardinality 1 ]
                                 [ a owl:Restriction ; owl:onProperty ex:q ; owl:minCardinality 1 ] ) ] .
             ex:p a owl:ObjectProperty .
             ex:q a owl:ObjectProperty .",
        )
        .expect("first ordering");
        let second = complete(
            "ex:q a owl:ObjectProperty .
             ex:p a owl:ObjectProperty .
             ex:A rdfs:subClassOf
                 [ owl:unionOf ( [ owl:minCardinality 1 ; owl:onProperty ex:q ; a owl:Restriction ]
                                 [ owl:minCardinality 1 ; a owl:Restriction ; owl:onProperty ex:p ] ) ] ,
                 [ owl:hasValue [ rdfs:label \"x\" ] ; owl:onProperty ex:p ; a owl:Restriction ] ;
                 a owl:Class .",
        )
        .expect("second ordering");
        assert_eq!(
            first.class_expressions.to_json(),
            second.class_expressions.to_json()
        );
        assert_eq!(first.report.to_json(), second.report.to_json());
        assert!(
            first
                .class_expressions
                .to_json()
                .contains("has_value(<https://example.org/schema/p>,[])"),
            "an anonymous individual renders without its parse-local label"
        );
    }

    #[test]
    fn data_range_operands_are_not_admitted_as_classes() {
        let surface = complete(
            "ex:A a owl:Class ; rdfs:subClassOf
                 [ a owl:Restriction ; owl:onProperty ex:p ; owl:allValuesFrom
                   [ a rdfs:Datatype ; owl:onDatatype ex:Celsius ;
                     owl:withRestrictions ( [ xsd:minInclusive 0 ] ) ] ] ,
                 [ a owl:Restriction ; owl:onProperty ex:q ; owl:allValuesFrom
                   [ a rdfs:Datatype ; owl:datatypeComplementOf ex:Kelvin ] ] ,
                 [ a owl:Restriction ; owl:onProperty ex:r ; owl:someValuesFrom ex:B ] .",
        )
        .expect("data ranges as fillers are accepted");
        assert!(surface.classes.contains_key("https://example.org/schema/B"));
        assert!(
            !surface
                .classes
                .contains_key("https://example.org/schema/Celsius")
        );
        assert!(
            !surface
                .classes
                .contains_key("https://example.org/schema/Kelvin")
        );
    }

    #[test]
    fn broken_pattern_facet_is_refused_and_valid_pattern_accepted() {
        let restriction = |pattern: &str| {
            format!(
                "ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:p ; owl:allValuesFrom
                    [ a rdfs:Datatype ; owl:onDatatype xsd:string ;
                      owl:withRestrictions ( [ xsd:pattern \"{pattern}\" ] ) ] ] ."
            )
        };
        refusal_with_neighbour(
            &restriction("[a-z"),
            "is not an XSD regular expression",
            &restriction("[a-z]+"),
        );
        // `^` and `$` are ordinary characters in an XSD pattern.
        assert_eq!(xsd_pattern_as_xpath("^a$[^$]"), "^(\\^a\\$[^$])$");
        complete(&restriction("^a$")).expect("anchors are literal characters in XSD");
    }

    #[test]
    fn equivalences_with_data_ranges_define_datatypes() {
        let surface = complete(
            "ex:Percent owl:equivalentClass [ a rdfs:Datatype ; owl:onDatatype xsd:integer ;
                 owl:withRestrictions ( [ xsd:minInclusive 0 ] ) ] .
             ex:Count owl:equivalentClass xsd:nonNegativeInteger .
             ex:Label owl:equivalentClass ex:Text .
             ex:Text owl:equivalentClass [ a rdfs:Datatype ; owl:datatypeComplementOf xsd:integer ] .
             ex:score a owl:DatatypeProperty ; rdfs:range ex:Percent .
             ex:n a owl:DatatypeProperty ; rdfs:range ex:Count .
             ex:label a owl:DatatypeProperty ; rdfs:range ex:Label .",
        )
        .expect("datatype definitions are accepted as ranges of datatype properties");
        for datatype in ["Percent", "Count", "Label", "Text"] {
            let iri = format!("https://example.org/schema/{datatype}");
            assert!(surface.datatypes.contains(&iri), "{datatype} is a datatype");
            assert!(
                !surface.classes.contains_key(&iri),
                "{datatype} is no class"
            );
            assert!(surface.datatype_definitions.contains_key(&iri));
        }
        assert_eq!(
            surface
                .class_expressions
                .axioms
                .iter()
                .map(|axiom| axiom.provenance.subject.as_str())
                .collect::<Vec<_>>(),
            vec![
                "https://example.org/schema/Percent",
                "https://example.org/schema/Text"
            ],
            "only the anonymous definitions are class-expression axioms"
        );
        // The neighbour: an equivalence with a class expression still makes
        // the named side a class.
        let classes = complete(
            "ex:Parent owl:equivalentClass [ a owl:Restriction ; owl:onProperty ex:child ;
                 owl:someValuesFrom ex:Person ] .",
        )
        .expect("class equivalence");
        assert!(
            classes
                .classes
                .contains_key("https://example.org/schema/Parent")
        );
    }
}

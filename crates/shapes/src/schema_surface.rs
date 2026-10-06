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
//! malformed input — a restriction without `owl:onProperty`, an ill-typed
//! cardinality, an ill-formed or cyclic RDF list, an expression that contains
//! itself — fails, with a typed error. A blank node carrying several readings
//! (several facets or values on one restriction, several constructs) is their
//! conjunction.
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
    MAX_OWL_EXPRESSION_DEPTH, MAX_SCHEMA_CLASS_MEMBERSHIPS, MAX_SCHEMA_CLASSES,
    MAX_SCHEMA_PROPERTIES, MAX_SCHEMA_RELATIONS, SchemaClassExpressionAxiom,
    SchemaClassExpressionCoverage, SchemaClassExpressionReport, SchemaClassPropertyCoverage,
    SchemaCompileError, SchemaCompileRequest, SchemaCoveragePrecision, SchemaCoverageProvenance,
    SchemaCoverageReport, SchemaCoverageStatus, SchemaExpressionComponent, SchemaExpressionOutcome,
    SchemaPropertyCoverage, SchemaSurfaceMode,
};
use crate::model::{rdf, rdfs};
use crate::shapes::{ClosedMode, Constraint, Path, Shape, Target};
use crate::term::{NamedNode, Term};

use purrdf_iri::vocab::owl::{
    ALL_DISJOINT_CLASSES as OWL_ALL_DISJOINT_CLASSES, DISJOINT_UNION_OF as OWL_DISJOINT_UNION_OF,
    DISJOINT_WITH as OWL_DISJOINT_WITH, HAS_KEY as OWL_HAS_KEY, MEMBERS as OWL_MEMBERS,
    NOTHING as OWL_NOTHING, NS as OWL_NS, SYMMETRIC_PROPERTY as OWL_SYMMETRIC_PROPERTY,
    THING as OWL_THING,
};
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
use purrdf_iri::vocab::rdf::{
    DIR_LANG_STRING as RDF_DIR_LANG_STRING, HTML as RDF_HTML, JSON as RDF_JSON,
    LANG_STRING as RDF_LANG_STRING, PLAIN_LITERAL as RDF_PLAIN_LITERAL, PROPERTY as RDF_PROPERTY,
    XML_LITERAL as RDF_XML_LITERAL,
};
use purrdf_iri::vocab::rdfs::{
    DATATYPE as RDFS_DATATYPE, DOMAIN as RDFS_DOMAIN, LITERAL as RDFS_LITERAL,
    SUB_PROPERTY_OF as RDFS_SUB_PROPERTY_OF,
};
use purrdf_xsd::datatype::{
    OWL_RATIONAL, OWL_REAL, XSD_BOOLEAN, XSD_LENGTH, XSD_MAX_EXCLUSIVE, XSD_MAX_INCLUSIVE,
    XSD_MAX_LENGTH, XSD_MIN_EXCLUSIVE, XSD_MIN_INCLUSIVE, XSD_MIN_LENGTH, XSD_PATTERN,
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

    pub(crate) fn fillers(&self) -> impl Iterator<Item = &OntologyExpression> {
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
    /// Whether the expression's extension is empty by its form: `owl:Nothing`,
    /// the empty enumeration, the complement of `owl:Thing`, a union of empty
    /// members, or an intersection with one.
    pub(crate) fn is_nothing(&self) -> bool {
        match self {
            Self::Named(iri) => iri == OWL_NOTHING,
            Self::OneOf(members) => members.is_empty(),
            Self::Complement(inner) => inner.is_thing(),
            Self::Union(members) => !members.is_empty() && members.iter().all(Self::is_nothing),
            Self::Intersection(members) => members.iter().any(Self::is_nothing),
            _ => false,
        }
    }

    /// Whether the expression is `owl:Thing` by its form: `owl:Thing` or the
    /// complement of an empty expression, or a restriction every individual
    /// meets: a minimum of zero, a universal over `owl:Thing`, or a maximum
    /// (or an exact count of zero) over an empty qualifier.
    fn is_thing(&self) -> bool {
        match self {
            Self::Named(iri) => iri == OWL_THING,
            Self::Complement(inner) => inner.is_nothing(),
            Self::Restriction(_, restriction) => match restriction.as_ref() {
                Restriction::Min(0, _) => true,
                Restriction::AllValues(filler) => filler.is_thing(),
                Restriction::Max(_, Some(qualifier)) | Restriction::Exact(0, Some(qualifier)) => {
                    qualifier.is_nothing()
                }
                _ => false,
            },
            _ => false,
        }
    }

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

    fn matches_class(
        &self,
        supertypes: &BTreeSet<String>,
        anonymous: &AnonymousSupers<'_>,
    ) -> bool {
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

    /// Replace each restriction on `inverse(p)` whose inverse is named by a
    /// restriction on that named property.
    fn resolve_inverses(&mut self, inverses: &BTreeMap<String, String>) {
        match self {
            Self::Named(_) | Self::OneOf(_) | Self::DatatypeRestriction(..) => {}
            Self::Union(members) | Self::Intersection(members) => {
                for member in members.iter_mut() {
                    member.resolve_inverses(inverses);
                }
                members.sort();
                members.dedup();
            }
            Self::Complement(inner) | Self::DatatypeComplement(inner) => {
                inner.resolve_inverses(inverses);
            }
            Self::Restriction(on, restriction) => {
                if let RestrictedProperty::One(PropertyExpression::Inverse(iri)) = on
                    && let Some(named) = inverses.get(iri.as_str())
                {
                    *on = RestrictedProperty::One(PropertyExpression::Named(named.clone()));
                }
                match restriction.as_mut() {
                    Restriction::SomeValues(filler) | Restriction::AllValues(filler) => {
                        filler.resolve_inverses(inverses);
                    }
                    Restriction::Min(_, Some(filler))
                    | Restriction::Max(_, Some(filler))
                    | Restriction::Exact(_, Some(filler)) => filler.resolve_inverses(inverses),
                    _ => {}
                }
            }
        }
    }

    /// The named properties restricted on the focus node itself: by a
    /// restriction, or a member of a union or intersection of them — not
    /// inside a complement, and not in a filler, which describes a value.
    fn focus_restricted<'s>(&'s self, out: &mut BTreeSet<&'s str>) {
        match self {
            Self::Restriction(on, _) => {
                if let Some(iri) = on.named() {
                    out.insert(iri);
                }
            }
            Self::Union(members) | Self::Intersection(members) => {
                for member in members {
                    member.focus_restricted(out);
                }
            }
            _ => {}
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
    /// The value schema judges every literal exactly but one typed
    /// `owl:rational`, which a real-number datatype's value space may hold
    /// and which it admits unjudged: exact over every other literal, so a
    /// maximum counted over it never rejects a conforming value.
    Judged,
    /// A named class: the value is an open node reference, as an
    /// `rdfs:range` class projects.
    ClassLike,
    /// The value schema admits more than the expression's values.
    Approximate,
}

/// The datatypes one compilation knows: every declared or defined one, and
/// each defined one's defining data range.
#[derive(Debug, Clone, Copy)]
pub(crate) struct DatatypeScope<'d> {
    pub(crate) names: &'d BTreeSet<String>,
    pub(crate) definitions: &'d BTreeMap<String, OntologyExpression>,
}

/// What an `owl:withRestrictions` facet list over `base` restricts once a
/// defined base is expanded through its definition. OWL 2 Structural
/// Specification §9.4 gives a defined datatype no facets and no literals of its
/// own, but makes it a synonym of its defining range: a facet over it is read
/// as a facet over that range, the only values it can have.
pub(crate) enum DefinedBase<'d> {
    /// The base has no definition.
    Plain,
    /// The base is defined by a datatype, or by a restriction of one: the
    /// facets restrict that datatype, conjoined with the definition's own.
    Expanded(String, Vec<(String, ExpressionTerm)>),
    /// The base is defined by a data range facets cannot restrict (a union,
    /// an enumeration, a complement), or its definitions cycle.
    Opaque(Option<&'d OntologyExpression>),
}

/// Expand `base[facets]` through the base's definition, following a chain
/// of defined datatypes to the first that is not defined.
pub(crate) fn expand_defined_base<'d>(
    base: &str,
    facets: &[(String, ExpressionTerm)],
    definitions: &'d BTreeMap<String, OntologyExpression>,
) -> DefinedBase<'d> {
    let Some(mut definition) = definitions.get(base) else {
        return DefinedBase::Plain;
    };
    let mut conjoined: Vec<(String, ExpressionTerm)> = facets.to_vec();
    // Each step names a further definition, so a chain longer than there are
    // definitions is a cycle.
    for _ in 0..=definitions.len() {
        let next = match definition {
            OntologyExpression::Named(iri) => iri,
            OntologyExpression::DatatypeRestriction(iri, own) => {
                conjoined.splice(0..0, own.iter().cloned());
                iri
            }
            other => return DefinedBase::Opaque(Some(other)),
        };
        match definitions.get(next) {
            Some(further) => definition = further,
            None => return DefinedBase::Expanded(next.clone(), conjoined),
        }
    }
    DefinedBase::Opaque(None)
}

/// The precision of the value schema an expression projects as.
pub(crate) fn value_precision(
    expression: &OntologyExpression,
    datatypes: DatatypeScope<'_>,
) -> ValuePrecision {
    precision_within(expression, datatypes, datatypes.definitions.len())
}

/// [`value_precision`], expanding at most `expansions` more definitions: a
/// definition reached through itself is not expanded again, as the schema
/// compiler does not expand it again.
fn precision_within(
    expression: &OntologyExpression,
    datatypes: DatatypeScope<'_>,
    expansions: usize,
) -> ValuePrecision {
    match expression {
        OntologyExpression::Named(iri) => {
            if matches!(
                iri.as_str(),
                OWL_REAL | OWL_RATIONAL | RDF_JSON | RDF_XML_LITERAL | RDF_HTML
            ) {
                // Projected as literals of the datatype or its members without
                // judging the lexical form.
                ValuePrecision::Approximate
            } else if let Some(definition) = datatypes.definitions.get(iri) {
                // A defined datatype admits a literal typed with it by name, or
                // a value that meets its definition, so it is as exact as its
                // definition's schema.
                expansions
                    .checked_sub(1)
                    .map_or(ValuePrecision::Exact, |rest| {
                        precision_within(definition, datatypes, rest)
                    })
            } else if iri == OWL_THING || iri == OWL_NOTHING {
                ValuePrecision::Exact
            } else if is_datatype(iri, datatypes.names) {
                // Read by its value space, a real-number datatype also admits
                // every literal typed `owl:rational`, whose value no pattern
                // judges.
                if crate::owl_value_space::value_space(iri)
                    .is_some_and(|space| space.unjudged_rationals)
                {
                    ValuePrecision::Judged
                } else {
                    ValuePrecision::Exact
                }
            } else {
                ValuePrecision::ClassLike
            }
        }
        OntologyExpression::Union(members) | OntologyExpression::Intersection(members) => members
            .iter()
            .map(|member| precision_within(member, datatypes, expansions))
            .max()
            .unwrap_or(ValuePrecision::Exact),
        OntologyExpression::OneOf(members) => {
            // A literal member is matched by value: exactly for strings,
            // booleans, IEEE numbers and language-tagged strings; with every
            // `owl:rational` literal admitted for a real number; and by any
            // literal of its datatype otherwise.
            members
                .iter()
                .map(|member| match &member.term {
                    Term::Literal(literal)
                        if literal.language().is_some()
                            || crate::owl_value_space::equality_is_exact(
                                literal.datatype_str(),
                            ) =>
                    {
                        ValuePrecision::Exact
                    }
                    Term::Literal(literal)
                        if crate::owl_value_space::is_numeric(literal.datatype_str()) =>
                    {
                        ValuePrecision::Judged
                    }
                    _ => ValuePrecision::Approximate,
                })
                .max()
                .unwrap_or(ValuePrecision::Exact)
        }
        OntologyExpression::DatatypeRestriction(base, facets) => {
            if !is_datatype(base, datatypes.names) {
                return ValuePrecision::Approximate;
            }
            match expand_defined_base(base, facets, datatypes.definitions) {
                DefinedBase::Plain => {
                    let space = crate::owl_value_space::value_space(base);
                    if !facets
                        .iter()
                        .all(|(facet, value)| facet_supported(base, facet, &value.term))
                    {
                        ValuePrecision::Approximate
                    } else if space.is_some_and(|space| space.unjudged_rationals) {
                        ValuePrecision::Judged
                    } else {
                        ValuePrecision::Exact
                    }
                }
                DefinedBase::Expanded(base, facets) => precision_within(
                    &OntologyExpression::DatatypeRestriction(base, facets),
                    datatypes,
                    expansions,
                ),
                DefinedBase::Opaque(_) => ValuePrecision::Approximate,
            }
        }
        // A datatype complement is judged on the literal's datatype tag, not on
        // its value space: `"-3"^^xsd:integer` is a negative integer although
        // it is not tagged `xsd:negativeInteger`.
        OntologyExpression::DatatypeComplement(_)
        | OntologyExpression::Complement(_)
        | OntologyExpression::Restriction(..) => ValuePrecision::Approximate,
    }
}

/// Whether a filler's value schema judges every literal it can (all but
/// `owl:rational` ones) exactly, so that a maximum counted over those values,
/// or a complement of them, never rejects a conforming value.
pub(crate) fn counts_exactly(
    expression: &OntologyExpression,
    datatypes: DatatypeScope<'_>,
) -> bool {
    matches!(
        value_precision(expression, datatypes),
        ValuePrecision::Exact | ValuePrecision::Judged
    )
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
    if is_builtin_non_xsd_datatype(base) {
        // `owl:real`, `rdf:PlainLiteral` and the rest have no SHACL datatype
        // projection to compile a facet against.
        return false;
    }
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
            base.strip_prefix(purrdf_xsd::datatype::XSD_NS)
                .is_some_and(crate::owl_value_space::is_string_datatype)
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
struct AnonymousSupers<'a> {
    canonical: BTreeSet<&'a str>,
    unions: Vec<&'a BTreeSet<String>>,
}

impl AnonymousSupers<'_> {
    fn entails(&self, expression: &OntologyExpression) -> bool {
        !self.canonical.is_empty() && self.canonical.contains(expression.canonical().as_str())
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
    const fn kind(&self) -> OntologyPropertyKind {
        // Several declarations on one IRI (punning, as PROV-O declares
        // `prov:specializationOf` an annotation and an object property) are
        // read by the OWL 2 RDF-Based Semantics (§5.3): every property is an
        // owl:ObjectProperty and may be an annotation property, while an
        // owl:DatatypeProperty's values are data values. So a datatype
        // declaration decides, then an object declaration; an annotation
        // property's value may be any term.
        if self.datatype_property {
            OntologyPropertyKind::Datatype
        } else if self.object_property {
            OntologyPropertyKind::Object
        } else if self.annotation_property {
            OntologyPropertyKind::Annotation
        } else {
            OntologyPropertyKind::Generic
        }
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
    /// The classes whose restriction fragments on this property the class
    /// references: itself where it owns restrictions on it, otherwise its
    /// nearest ancestors that do. Each fragment references its own nearest
    /// owning ancestors, so a class's schema grows with the restrictions it
    /// owns, not with its depth.
    pub(crate) restriction_owners: Vec<String>,
    /// Whether some restriction anywhere in the ontology gives this object
    /// property a literal value: a data-range filler or a literal
    /// `owl:hasValue`. Read by the OWL 2 Full (RDF-Based) Semantics, §5.3,
    /// the property then takes literals on every class that carries it.
    pub(crate) takes_literals: bool,
    /// Whether some restriction gives this datatype property a node value: an
    /// individual `owl:hasValue`, or `owl:hasSelf`. Read the same way, the
    /// property then takes nodes on every class that carries it.
    pub(crate) takes_nodes: bool,
}

/// One existing named class represented by a schema `$def`.
#[derive(Debug, Clone, Default)]
pub(crate) struct SurfaceClass {
    pub(crate) synthesized_open: bool,
    /// Whether the class is a subclass of `owl:Nothing` (`A ⊑ ⊔()`, say), so
    /// that nothing is an instance of it.
    pub(crate) unsatisfiable: bool,
    pub(crate) properties: BTreeMap<String, SurfaceProperty>,
    /// Class-level anonymous superclass expressions projected on the focus
    /// node itself: an enumeration of named individuals, the complement of a
    /// named class, or a disjunction of property restrictions.
    pub(crate) focus: Vec<OntologyExpression>,
    /// Each anonymous superclass expression component the class's schema
    /// cannot represent, rendered `expression: reason`, sorted: those the
    /// class owns, and those whose outcome differs from its owner's.
    pub(crate) unrepresented: Vec<String>,
    /// The classes whose disjunction fragments the class references: itself
    /// where it owns projected disjunctions, otherwise its nearest ancestors
    /// that do. Empty where an inherited disjunction classifies on the class
    /// otherwise than on its owner: the class then states the disjunctions it
    /// projects in `focus`.
    pub(crate) disjunction_owners: Vec<String>,
}

/// The restrictions or disjunctions one class owns on one property (or, for
/// disjunctions, on the focus node), and the nearest strict ancestors whose
/// fragments for the same slot it references.
#[derive(Debug, Clone, Default)]
pub(crate) struct Fragment {
    pub(crate) restrictions: Vec<Restriction>,
    pub(crate) disjunctions: Vec<OntologyExpression>,
    pub(crate) parents: Vec<String>,
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
    /// Restriction fragments by `(owner class, Some(property))` and
    /// disjunction fragments by `(owner class, None)`.
    pub(crate) fragments: BTreeMap<(String, Option<String>), Fragment>,
    /// One emitted surface property per property IRI, for the value schemas
    /// of fragments whose owner does not itself emit the property.
    pub(crate) property_templates: BTreeMap<String, SurfaceProperty>,
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
            }
        }
        // Bounded by the coverage-cell ceiling checked before assembly. The
        // range expressions and provenance records per cell are bounded by the
        // ontology's own axioms, not by a separate ceiling: an IRI-only ontology
        // with any number of them within the cell ceiling compiles.
        debug_assert!(emitted <= MAX_SCHEMA_RELATIONS);
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
const THING_RANGE_REASON: &str =
    "a universal restriction on owl:Thing is the property's rdfs:range for every class";
const THING_INVERSE_RANGE_REASON: &str = "a universal restriction on owl:Thing over the inverse of a \
     property is that property's rdfs:domain for every class";
const DISJOINT_REASON: &str = "disjointness relates the memberships of two classes, which a \
     developer schema judging one node against one class cannot state";
const DISJOINT_UNION_REASON: &str = "a disjoint union relates the memberships of several \
     classes, which a developer schema judging one node against one class cannot state";
const CLASS_ASSERTION_REASON: &str = "a class assertion types an individual; developer schemas \
     describe classes, not individuals";
const NOT_A_CLASS_EXPRESSION_REASON: &str =
    "the resource is typed by a blank node that declares no OWL class-expression construct";
const MALFORMED_REASON: &str = "the anonymous expression is not a well-formed OWL class \
     expression; the axiom was skipped before anonymous expressions were read, so it is \
     reported rather than refused, and nothing is projected";
const HAS_KEY_REASON: &str = "a key identifies individuals of a class by their values, which no \
     developer schema judging one node states";
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
    /// Whether the axiom was skipped before anonymous expressions were read,
    /// so that an ill-typed one is reported rather than refused.
    lenient: bool,
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
            lenient: false,
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
        if declared.is_empty() {
            return Err(malformed(
                "anonymous OWL expression declares no class-expression or data-range \
                 construct (owl:unionOf, owl:intersectionOf, owl:complementOf, owl:oneOf, an \
                 owl:Restriction, owl:onDatatype or owl:datatypeComplementOf)"
                    .to_owned(),
            ));
        }
        // OWL 2 Mapping to RDF Graphs §3.2.1: a node is at most one of a class
        // expression and a data range. A construct only Table 13 reads beside
        // one only Table 12 reads is that clash.
        let class_only = declared
            .iter()
            .find(|name| matches!(**name, "owl:complementOf" | "an owl:Restriction"));
        let range_only = declared.iter().find(|name| {
            matches!(
                **name,
                "a datatype restriction" | "owl:datatypeComplementOf"
            )
        });
        if let (Some(class_only), Some(range_only)) = (class_only, range_only) {
            return Err(malformed(format!(
                "anonymous OWL expression mixes {class_only}, a class expression, and \
                 {range_only}, a data range; a node is at most one of the two"
            )));
        }

        // Every construct on the node, and every value of each, is one reading
        // of it. The OWL 2 RDF-Based Semantics (§5) gives each reading's class
        // extension to the node, so the readings coincide and the node is read
        // as their conjunction, as an OWL 1 restriction node carrying several
        // facets is. (The OWL 2 DL mapping reads a single pattern per node and
        // leaves the rest of the node's triples unparsed.)
        // A node typed `rdfs:Datatype` (or `owl:DataRange`) is a data range, so
        // an empty union or enumeration is the empty data range and an empty
        // intersection is `rdfs:Literal`, not `owl:Nothing` and `owl:Thing`.
        let data_range = take(rdf::TYPE).iter().any(|type_| {
            matches!(type_, Term::NamedNode(node) if matches!(node.as_str(), RDFS_DATATYPE | OWL_DATA_RANGE))
        });
        let mut readings: Vec<OntologyExpression> = Vec::new();
        for head in take(OWL_UNION_OF) {
            let members = self.boolean_members(head, depth + 1, key)?;
            readings.push(union_of(members, data_range));
        }
        for head in take(OWL_INTERSECTION_OF) {
            let members = self.boolean_members(head, depth + 1, key)?;
            readings.push(intersection_of(members, data_range));
        }
        for inner in take(OWL_COMPLEMENT_OF) {
            readings.push(OntologyExpression::Complement(Box::new(
                self.expression(inner, depth + 1)?,
            )));
        }
        for inner in take(OWL_DATATYPE_COMPLEMENT_OF) {
            readings.push(OntologyExpression::DatatypeComplement(Box::new(
                self.expression(inner, depth + 1)?,
            )));
        }
        for head in take(OWL_ONE_OF) {
            readings.push(self.one_of(head, key, data_range)?);
        }
        if declared.contains(&"a datatype restriction") {
            let (bases, heads) = (take(OWL_ON_DATATYPE), take(OWL_WITH_RESTRICTIONS));
            if bases.is_empty() || heads.is_empty() {
                // Report the missing half of the pair.
                single(bases, OWL_ON_DATATYPE, key)?;
                single(heads, OWL_WITH_RESTRICTIONS, key)?;
            }
            for base in bases {
                let Term::NamedNode(node) = base else {
                    return Err(malformed(format!(
                        "owl:onDatatype must name a datatype IRI; found {base}"
                    )));
                };
                for head in heads {
                    let reading = self.datatype_restriction(node.as_str().to_owned(), head, key)?;
                    readings.push(reading);
                }
            }
        }
        if restriction {
            readings.extend(self.restriction_readings(&fields, key, depth)?);
        }
        conjunction(readings)
    }

    /// The restrictions one `owl:Restriction` node states: one per pair of a
    /// property expression it restricts and a facet value it carries (and,
    /// for a qualified cardinality, a qualifier).
    fn restriction_readings(
        &mut self,
        fields: &BTreeMap<String, Vec<Term>>,
        key: &str,
        depth: usize,
    ) -> Result<Vec<OntologyExpression>, SchemaCompileError> {
        let take = |name: &str| fields.get(name).map_or(&[][..], Vec::as_slice);
        let malformed = |reason: String| SchemaCompileError::InvalidOntology {
            subject: key.to_owned(),
            reason,
        };
        let mut properties = Vec::new();
        for one in take(OWL_ON_PROPERTY) {
            properties.push(RestrictedProperty::One(self.property_expression(one)?));
        }
        for head in take(OWL_ON_PROPERTIES) {
            let mut members = Vec::new();
            for item in self.list_items(head, key)? {
                match item {
                    Term::NamedNode(node) => members.push(node.into_string()),
                    other => {
                        return Err(malformed(format!(
                            "owl:onProperties members must be data property IRIs; found {other}"
                        )));
                    }
                }
            }
            if members.is_empty() {
                return Err(malformed(
                    "owl:onProperties requires at least one property".to_owned(),
                ));
            }
            members.sort();
            members.dedup();
            properties.push(RestrictedProperty::Many(members));
        }
        if properties.is_empty() {
            return Err(malformed(
                "owl:Restriction declares no owl:onProperty; a restriction must name the \
                 property it restricts"
                    .to_owned(),
            ));
        }
        properties.sort();
        properties.dedup();
        let mut qualifiers = Vec::new();
        for one in take(OWL_ON_CLASS).iter().chain(take(OWL_ON_DATA_RANGE)) {
            qualifiers.push(self.expression(one, depth + 1)?);
        }
        qualifiers.sort();
        qualifiers.dedup();
        let mut restrictions = Vec::new();
        let mut qualified = false;
        for (facet, kind) in RESTRICTION_FACETS {
            for value in take(facet) {
                let qualified_by =
                    |count: u64, make: fn(u64, Option<OntologyExpression>) -> Restriction| {
                        if qualifiers.is_empty() {
                            return Err(malformed(format!(
                                "<{facet}> requires an owl:onClass or owl:onDataRange qualifier"
                            )));
                        }
                        Ok(qualifiers
                            .iter()
                            .map(|qualifier| make(count, Some(qualifier.clone())))
                            .collect::<Vec<_>>())
                    };
                match kind {
                    FacetKind::Some => {
                        restrictions
                            .push(Restriction::SomeValues(self.expression(value, depth + 1)?));
                    }
                    FacetKind::All => {
                        restrictions
                            .push(Restriction::AllValues(self.expression(value, depth + 1)?));
                    }
                    FacetKind::HasValue => {
                        restrictions
                            .push(Restriction::HasValue(ExpressionTerm::new(value.clone())));
                    }
                    FacetKind::HasSelf => {
                        has_self_value(value, key)?;
                        restrictions.push(Restriction::HasSelf);
                    }
                    FacetKind::Min => {
                        restrictions.push(Restriction::Min(cardinality(value, facet, key)?, None));
                    }
                    FacetKind::Max => {
                        restrictions.push(Restriction::Max(cardinality(value, facet, key)?, None));
                    }
                    FacetKind::Exact => {
                        restrictions
                            .push(Restriction::Exact(cardinality(value, facet, key)?, None));
                    }
                    FacetKind::QualifiedMin => {
                        qualified = true;
                        restrictions.extend(qualified_by(
                            cardinality(value, facet, key)?,
                            Restriction::Min,
                        )?);
                    }
                    FacetKind::QualifiedMax => {
                        qualified = true;
                        restrictions.extend(qualified_by(
                            cardinality(value, facet, key)?,
                            Restriction::Max,
                        )?);
                    }
                    FacetKind::QualifiedExact => {
                        qualified = true;
                        restrictions.extend(qualified_by(
                            cardinality(value, facet, key)?,
                            Restriction::Exact,
                        )?);
                    }
                }
            }
        }
        if !qualifiers.is_empty() && !qualified {
            return Err(malformed(
                "owl:onClass/owl:onDataRange qualifies no qualified cardinality".to_owned(),
            ));
        }
        if restrictions.is_empty() {
            return Err(malformed(format!(
                "owl:Restriction on {} declares no constraint (owl:someValuesFrom, \
                 owl:allValuesFrom, owl:hasValue, owl:hasSelf or a cardinality)",
                properties[0].canonical()
            )));
        }
        restrictions.sort();
        restrictions.dedup();
        if properties
            .iter()
            .any(|property| matches!(property, RestrictedProperty::Many(_)))
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
        self.count_nodes(
            properties
                .len()
                .saturating_mul(restrictions.len())
                .saturating_sub(1),
        )?;
        let mut members = Vec::with_capacity(properties.len().saturating_mul(restrictions.len()));
        for property in &properties {
            for restriction in &restrictions {
                members.push(OntologyExpression::Restriction(
                    property.clone(),
                    Box::new(restriction.clone()),
                ));
            }
        }
        Ok(members)
    }

    /// Whether a blank node declares any OWL class-expression or data-range
    /// construct, the predicates [`Self::expression`] dispatches on.
    fn declares_construct(&self, term: &Term) -> bool {
        native_quads(self.dataset, Some(term), None, None, GraphFilter::AnyGraph)
            .iter()
            .any(|(_, predicate, _)| {
                [
                    OWL_UNION_OF,
                    OWL_INTERSECTION_OF,
                    OWL_COMPLEMENT_OF,
                    OWL_ONE_OF,
                    OWL_ON_PROPERTY,
                    OWL_ON_PROPERTIES,
                    OWL_ON_CLASS,
                    OWL_ON_DATA_RANGE,
                    OWL_ON_DATATYPE,
                    OWL_WITH_RESTRICTIONS,
                    OWL_DATATYPE_COMPLEMENT_OF,
                ]
                .into_iter()
                .chain(RESTRICTION_FACETS.iter().map(|(facet, _)| *facet))
                .any(|construct| predicate.as_str() == construct)
            })
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
        members.sort();
        members.dedup();
        Ok(members)
    }

    /// An `owl:oneOf` list. An empty one denotes nothing: the OWL 2 Mapping
    /// (Table 18) reads `C owl:oneOf ()` as `C ≡ owl:Nothing`.
    fn one_of(
        &mut self,
        head: &Term,
        owner: &str,
        data_range: bool,
    ) -> Result<OntologyExpression, SchemaCompileError> {
        let malformed = |reason: String| SchemaCompileError::InvalidOntology {
            subject: owner.to_owned(),
            reason,
        };
        let items = self.list_items(head, owner)?;
        if items.is_empty() {
            return Ok(if data_range {
                OntologyExpression::OneOf(Vec::new())
            } else {
                OntologyExpression::Named(OWL_NOTHING.to_owned())
            });
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
        // An empty `owl:withRestrictions` restricts nothing: the range is the
        // base datatype itself.
        let items = self.list_items(head, owner)?;
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
    /// Whether the axiom was skipped before anonymous expressions were read,
    /// so that a malformed one is reported rather than refused.
    lenient: bool,
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
pub(crate) fn is_data_range(expression: &OntologyExpression, datatypes: &BTreeSet<String>) -> bool {
    !expression.has_class_only_construct()
        && (expression.has_data_only_construct()
            || expression.all_named_members_match(&|iri| is_datatype(iri, datatypes)))
}

/// `owl:unionOf` over `members` (sorted and deduplicated), by its meaning
/// under the OWL 2 RDF-Based Semantics: the union of no class is
/// `owl:Nothing` (of no data range, the empty enumeration), and the union of
/// one is that one.
fn union_of(mut members: Vec<OntologyExpression>, data_range: bool) -> OntologyExpression {
    match members.len() {
        0 if data_range => OntologyExpression::OneOf(Vec::new()),
        0 => OntologyExpression::Named(OWL_NOTHING.to_owned()),
        1 => members.pop().expect("one member"),
        _ => OntologyExpression::Union(members),
    }
}

/// `owl:intersectionOf` over `members`: the intersection of no class is
/// `owl:Thing` (of no data range, `rdfs:Literal`), and of one is that one.
fn intersection_of(mut members: Vec<OntologyExpression>, data_range: bool) -> OntologyExpression {
    match members.len() {
        0 if data_range => OntologyExpression::Named(RDFS_LITERAL.to_owned()),
        0 => OntologyExpression::Named(OWL_THING.to_owned()),
        1 => members.pop().expect("one member"),
        _ => OntologyExpression::Intersection(members),
    }
}

/// One expression for the readings of one node: the reading itself, or the
/// intersection of the distinct readings, nested intersections flattened.
fn conjunction(
    readings: Vec<OntologyExpression>,
) -> Result<OntologyExpression, SchemaCompileError> {
    let mut members = Vec::with_capacity(readings.len());
    for reading in readings {
        match reading {
            OntologyExpression::Intersection(inner) => members.extend(inner),
            other => members.push(other),
        }
    }
    members.sort();
    members.dedup();
    match members.len() {
        0 => unreachable!("a node with a construct has a reading"),
        1 => Ok(members.pop().expect("one reading")),
        _ => Ok(OntologyExpression::Intersection(members)),
    }
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
    saturating_count(literal.value()).ok_or_else(|| {
        malformed(format!(
            "<{facet}> requires a non-negative integer literal; found {value}"
        ))
    })
}

/// A cardinality's count: the non-negative integer a lexical form denotes
/// (after the `collapse` whitespace facet), with every count beyond
/// `u64::MAX` read as `u64::MAX`. No finite set of values tells those counts
/// apart: a maximum that large holds of every instance, and a minimum that
/// large of none, exactly as the larger count does.
fn saturating_count(lexical: &str) -> Option<u64> {
    let lexical = lexical.trim_matches(|c| matches!(c, ' ' | '\t' | '\n' | '\r'));
    let (negative, digits) = match lexical.as_bytes().first() {
        Some(b'+') => (false, &lexical[1..]),
        Some(b'-') => (true, &lexical[1..]),
        _ => (false, lexical),
    };
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    if digits.bytes().all(|byte| byte == b'0') {
        return Some(0);
    }
    if negative {
        return None;
    }
    Some(digits.bytes().fold(0_u64, |value, byte| {
        value
            .saturating_mul(10)
            .saturating_add(u64::from(byte - b'0'))
    }))
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

/// Catalog every property a restriction in `expression` names. The axiom
/// that names it is provenance only on the classes that carry the axiom.
fn catalog_restrictions(
    expression: &OntologyExpression,
    properties: &mut BTreeMap<String, PropertyFacts>,
) -> Result<(), SchemaCompileError> {
    expression.visit_restrictions(&mut |on, _restriction| {
        for iri in on.iris() {
            let facts = property_entry(properties, iri)?;
            facts.declarations.insert(OWL_ON_PROPERTY.to_owned());
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
    let mut other_axioms: Vec<AxiomLevel> = Vec::new();
    let mut named_constructors: BTreeMap<String, String> = BTreeMap::new();
    let mut symmetric: BTreeSet<String> = BTreeSet::new();
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
                // An axiom with an anonymous subject was skipped before
                // anonymous expressions were read, so a malformed one is
                // reported rather than refused; one with a named subject and an
                // anonymous object was refused, and still is when malformed.
                let lenient = !matches!(row.subject, Term::NamedNode(_));
                let read = (|| {
                    let subject = reader.expression(&row.subject, 0)?;
                    let object = reader.expression(&row.object, 0)?;
                    if !equivalent {
                        refuse_data_range_as_class(&subject, &row.subject)?;
                        refuse_data_range_as_class(&object, &row.object)?;
                    }
                    Ok::<_, SchemaCompileError>((subject, object))
                })();
                let (subject, object) = match read {
                    Ok(sides) => sides,
                    Err(_) if lenient => {
                        other_axioms.push(malformed_axiom(row));
                        continue;
                    }
                    Err(error) => return Err(error),
                };
                let provenance = SchemaCoverageProvenance {
                    subject: subject.provenance_subject(),
                    predicate: row.predicate.clone(),
                    object: object.canonical(),
                };
                catalog_restrictions(&subject, &mut properties)?;
                catalog_restrictions(&object, &mut properties)?;
                pending_axioms.push(PendingAxiom {
                    subject_term: row.subject.clone(),
                    object_term: row.object.clone(),
                    subject,
                    object,
                    equivalent,
                    lenient,
                    provenance,
                });
            }
            RDFS_SUB_PROPERTY_OF | OWL_EQUIVALENT_PROPERTY | OWL_INVERSE_OF => {
                // `_:x owl:inverseOf p` with a blank subject defines an inverse
                // property expression; it is read where that expression is used.
                if row.predicate == OWL_INVERSE_OF && matches!(row.subject, Term::BlankNode(_)) {
                    continue;
                }
                let sides = reader
                    .property_expression(&row.subject)
                    .and_then(|left| Ok((left, reader.property_expression(&row.object)?)));
                let (left, mut right) = match sides {
                    Ok(sides) => sides,
                    // Skipped before anonymous expressions were read: reported.
                    Err(_) if !matches!(row.subject, Term::NamedNode(_)) => {
                        other_axioms.push(malformed_axiom(row));
                        continue;
                    }
                    Err(error) => return Err(error),
                };
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
            // Each of these was skipped before anonymous expressions were read,
            // so a malformed one is reported rather than refused.
            OWL_DISJOINT_WITH => match disjoint_axiom(&mut reader, row) {
                Ok(Some(axiom)) => other_axioms.push(axiom),
                Ok(None) => {}
                Err(_) => other_axioms.push(malformed_axiom(row)),
            },
            OWL_DISJOINT_UNION_OF | OWL_MEMBERS => {
                if row.predicate == OWL_MEMBERS
                    && !objects_of(&union, &row.subject, rdf::TYPE)
                        .iter()
                        .any(|kind| named_iri(kind) == Some(OWL_ALL_DISJOINT_CLASSES))
                {
                    continue;
                }
                match members_axiom(&mut reader, row) {
                    Ok(Some(axiom)) => other_axioms.push(axiom),
                    Ok(None) => {}
                    Err(_) => other_axioms.push(malformed_axiom(row)),
                }
            }
            OWL_HAS_KEY if !matches!(row.subject, Term::NamedNode(_)) => {
                match has_key_axiom(&mut reader, row) {
                    Ok(axiom) => other_axioms.push(axiom),
                    Err(_) => other_axioms.push(malformed_axiom(row)),
                }
            }
            rdf::TYPE if matches!(row.object, Term::BlankNode(_)) => {
                match class_assertion_axiom(&mut reader, row) {
                    Ok(axiom) => other_axioms.push(axiom),
                    Err(_) => other_axioms.push(malformed_axiom(row)),
                }
            }
            predicate => {
                let Some(subject_iri) = named_iri(&row.subject) else {
                    continue;
                };
                if is_construct_predicate(predicate) {
                    // A class constructor on an IRI: the IRI names the class the
                    // construct describes (OWL 2 Mapping to RDF Graphs reads it
                    // as an equivalence), read once every row is seen.
                    named_constructors
                        .entry(subject_iri.to_owned())
                        .or_insert_with(|| predicate.to_owned());
                }
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
                            OWL_SYMMETRIC_PROPERTY => {
                                symmetric.insert(subject_iri.to_owned());
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
                        catalog_restrictions(&expression, &mut properties)?;
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
    for (class, predicate) in &named_constructors {
        let term = Term::NamedNode(NamedNode::new_unchecked(class.as_str()));
        match reader.anonymous(&term, class, 0) {
            Ok(object) => {
                catalog_restrictions(&object, &mut properties)?;
                pending_axioms.push(PendingAxiom {
                    subject_term: term.clone(),
                    object_term: term,
                    provenance: SchemaCoverageProvenance {
                        subject: class.clone(),
                        predicate: predicate.clone(),
                        object: object.canonical(),
                    },
                    subject: OntologyExpression::Named(class.clone()),
                    object,
                    equivalent: true,
                    lenient: true,
                });
            }
            // Skipped before anonymous expressions were read: reported.
            Err(_) => other_axioms.push((
                SchemaCoverageProvenance {
                    subject: class.clone(),
                    predicate: predicate.clone(),
                    object: ANONYMOUS_INDIVIDUAL.to_owned(),
                },
                vec![axiom_component(
                    ANONYMOUS_INDIVIDUAL.to_owned(),
                    SchemaExpressionOutcome::Unrepresented,
                    MALFORMED_REASON,
                )],
            )),
        }
    }

    // The datatypes known before any definition is read: the declared ones.
    // An object property ranging over a datatype only these name is refused,
    // as it always was; one ranging over a defined or newly recognised
    // datatype is accepted, as it always was.
    let declared_datatypes = datatypes.clone();
    let mut datatype_definitions: BTreeMap<String, OntologyExpression> = BTreeMap::new();
    let mut datatype_axioms: Vec<AxiomLevel> = Vec::new();
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
                datatype_axioms.push((
                    axiom.provenance,
                    vec![axiom_component(
                        range.canonical(),
                        SchemaExpressionOutcome::Approximated,
                        DATATYPE_DEFINITION_REASON,
                    )],
                ));
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
    // A named property's inverse may itself be named (`made owl:inverseOf
    // maker`, or a symmetric property): a restriction on `inverse(made)` is a
    // restriction on `maker`.
    let inverse_names = named_inverses(&property_relations, &symmetric);
    if !inverse_names.is_empty() {
        for axiom in &mut pending_axioms {
            axiom.subject.resolve_inverses(&inverse_names);
            axiom.object.resolve_inverses(&inverse_names);
        }
    }
    let mut class_axioms: Vec<ClassAxiom> = Vec::with_capacity(pending_axioms.len());
    for axiom in pending_axioms {
        let checked = refuse_data_range_as_class(&axiom.subject, &axiom.subject_term)
            .and_then(|()| refuse_data_range_as_class(&axiom.object, &axiom.object_term));
        match checked {
            Ok(()) => {}
            Err(_) if axiom.lenient => {
                datatype_axioms.push((
                    axiom.provenance,
                    vec![axiom_component(
                        ANONYMOUS_INDIVIDUAL.to_owned(),
                        SchemaExpressionOutcome::Unrepresented,
                        MALFORMED_REASON,
                    )],
                ));
                continue;
            }
            Err(error) => return Err(error),
        }
        let mut classified = ClassAxiom::classify(
            &axiom.subject,
            &axiom.object,
            axiom.equivalent,
            axiom.provenance,
        );
        classified.lenient = axiom.lenient;
        class_axioms.push(classified);
    }
    datatype_axioms.extend(other_axioms);
    datatype_axioms.sort();

    globalize_thing_universals(&mut class_axioms, &mut properties)?;

    enforce_limit("properties", properties.len(), MAX_SCHEMA_PROPERTIES)?;
    propagate_property_facts(&mut properties, &property_relations)?;
    // A property whose range contradicts its declared kind (an
    // owl:ObjectProperty over xsd:string, an owl:DatatypeProperty over a class)
    // is read by the OWL 2 Full (RDF-Based) Semantics (§5.3), not refused: its
    // values are of the range's kind, and the reading is an approximation.
    // So is a restriction whose filler or value is of the other kind: a
    // literal `owl:hasValue` on an object property is `∃p.{v}`, and the
    // property takes literals (`cross_kind_valued_properties`).
    existential_domain_edges(
        &class_axioms,
        &properties,
        &datatypes,
        &mut subclass_relations,
        &mut explicit_classes,
    );

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
            // A named skeleton's names are its members, inserted just above.
            if !domain.expression.is_named_skeleton() {
                class_names(&domain.expression, &mut explicit_classes);
            }
        }
        let kind = facts.kind();
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
            prior: declared_datatypes,
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
    /// Every declared or defined datatype.
    declared: BTreeSet<String>,
    /// The declared datatypes alone, which an object property's range may
    /// name as a literal (see `is_prior_datatype`).
    prior: BTreeSet<String>,
    definitions: BTreeMap<String, OntologyExpression>,
    /// Axioms reported on the axiom itself rather than on a class: datatype
    /// definitions, disjointness, and class assertions.
    axioms: Vec<AxiomLevel>,
}

/// One axiom the manifest reports with axiom-level components only.
type AxiomLevel = (SchemaCoverageProvenance, Vec<SchemaExpressionComponent>);

fn axiom_component(
    expression: String,
    outcome: SchemaExpressionOutcome,
    reason: &str,
) -> SchemaExpressionComponent {
    SchemaExpressionComponent {
        expression,
        property_iri: None,
        outcome,
        reason: reason.to_owned(),
    }
}

/// Each named property whose inverse is also named: `inverse(p) = q` for every
/// `p owl:inverseOf q` (either way round, through `owl:equivalentProperty`
/// too) and `inverse(p) = p` for a symmetric `p`. The smallest IRI wins where
/// several are named.
fn named_inverses(
    relations: &[PropertyRelation],
    symmetric: &BTreeSet<String>,
) -> BTreeMap<String, String> {
    let mut candidates: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for property in symmetric {
        candidates.entry(property).or_default().insert(property);
    }
    for relation in relations {
        if matches!(relation.kind, PropertyRelationKind::Equivalent)
            && relation.left.is_inverse() != relation.right.is_inverse()
        {
            let (left, right) = (relation.left.iri(), relation.right.iri());
            candidates.entry(left).or_default().insert(right);
            candidates.entry(right).or_default().insert(left);
        }
    }
    candidates
        .into_iter()
        .filter_map(|(property, inverses)| {
            inverses
                .first()
                .map(|inverse| (property.to_owned(), (*inverse).to_owned()))
        })
        .collect()
}

/// A universal restriction asserted of `owl:Thing` holds of every individual:
/// `owl:Thing ⊑ ∀p.F` is `p`'s range `F`, and `owl:Thing ⊑ ∀inverse(p).F` its
/// domain. Each is moved onto the property, propagated with its other
/// ranges and domains, and reported on the axiom.
fn globalize_thing_universals(
    class_axioms: &mut [ClassAxiom],
    properties: &mut BTreeMap<String, PropertyFacts>,
) -> Result<(), SchemaCompileError> {
    for axiom in class_axioms {
        let ClassAxiom {
            provenance,
            carriers,
            uncarried,
            ..
        } = axiom;
        for (carrier, conjuncts) in carriers.iter_mut() {
            if carrier != OWL_THING {
                continue;
            }
            let mut kept = Vec::with_capacity(conjuncts.len());
            for conjunct in std::mem::take(conjuncts) {
                let OntologyExpression::Restriction(RestrictedProperty::One(property), restriction) =
                    &conjunct
                else {
                    kept.push(conjunct);
                    continue;
                };
                let Restriction::AllValues(filler) = restriction.as_ref() else {
                    kept.push(conjunct);
                    continue;
                };
                let facts = property_entry(properties, property.iri())?;
                let sourced = SourcedExpression {
                    expression: filler.clone(),
                    provenance: provenance.clone(),
                };
                facts.provenance.insert(provenance.clone());
                let reason = if property.is_inverse() {
                    facts.domains.insert(sourced);
                    THING_INVERSE_RANGE_REASON
                } else {
                    facts.ranges.insert(sourced);
                    THING_RANGE_REASON
                };
                uncarried.push(SchemaExpressionComponent {
                    expression: conjunct.canonical(),
                    property_iri: Some(property.iri().to_owned()),
                    outcome: SchemaExpressionOutcome::Projected,
                    reason: reason.to_owned(),
                });
            }
            *conjuncts = kept;
        }
        uncarried.sort();
        uncarried.dedup();
    }
    Ok(())
}

/// A class that a restriction requires a value of is within the restricted
/// property's domain (OWL 2 Direct Semantics: `C ⊑ ∃p.X` and `dom(p) = D`
/// entail `C ⊑ D`), and, through an inverse, within its range. Each named
/// class of such a domain becomes a superclass in the hierarchy, so the other
/// properties of that domain reach the class too.
fn existential_domain_edges(
    class_axioms: &[ClassAxiom],
    properties: &BTreeMap<String, PropertyFacts>,
    datatypes: &BTreeSet<String>,
    subclass_relations: &mut Vec<(String, String)>,
    explicit_classes: &mut BTreeSet<String>,
) {
    for axiom in class_axioms {
        for (carrier, conjuncts) in &axiom.carriers {
            if carrier == OWL_THING {
                continue;
            }
            for conjunct in conjuncts {
                let OntologyExpression::Restriction(RestrictedProperty::One(property), restriction) =
                    conjunct
                else {
                    continue;
                };
                if !restriction.is_existential() {
                    continue;
                }
                let Some(facts) = properties.get(property.iri()) else {
                    continue;
                };
                let scopes = if property.is_inverse() {
                    &facts.ranges
                } else {
                    &facts.domains
                };
                for scope in scopes {
                    for part in scope.expression.conjuncts() {
                        if let OntologyExpression::Named(class) = part
                            && class != carrier
                            && !is_datatype(class, datatypes)
                        {
                            explicit_classes.insert(class.clone());
                            subclass_relations.push((carrier.clone(), class.clone()));
                        }
                    }
                }
            }
        }
    }
}

/// The predicates that make an IRI subject a class constructor.
fn is_construct_predicate(predicate: &str) -> bool {
    // Every constructor predicate is in the OWL namespace: one prefix test
    // settles the rows of an ontology without constructors.
    predicate.starts_with(OWL_NS)
        && ([
            OWL_UNION_OF,
            OWL_INTERSECTION_OF,
            OWL_COMPLEMENT_OF,
            OWL_ONE_OF,
            OWL_ON_PROPERTY,
            OWL_ON_PROPERTIES,
            OWL_ON_CLASS,
            OWL_ON_DATA_RANGE,
            OWL_ON_DATATYPE,
            OWL_WITH_RESTRICTIONS,
            OWL_DATATYPE_COMPLEMENT_OF,
        ]
        .contains(&predicate)
            || RESTRICTION_FACETS
                .iter()
                .any(|(facet, _)| *facet == predicate))
}

/// The rendering of an axiom side in a malformed axiom's provenance: a named
/// subject bare, a named object in angle brackets, anything anonymous as
/// `[]` (a blank-node label is local to one parse, and would make the report
/// change with triple order).
fn malformed_axiom(row: &TripleRow) -> AxiomLevel {
    let subject =
        named_iri(&row.subject).map_or_else(|| ANONYMOUS_INDIVIDUAL.to_owned(), str::to_owned);
    let object = match &row.object {
        Term::NamedNode(node) => format!("<{}>", node.as_str()),
        Term::Literal(_) => row.object_key.clone(),
        _ => ANONYMOUS_INDIVIDUAL.to_owned(),
    };
    (
        SchemaCoverageProvenance {
            subject,
            predicate: row.predicate.clone(),
            object,
        },
        vec![axiom_component(
            ANONYMOUS_INDIVIDUAL.to_owned(),
            SchemaExpressionOutcome::Unrepresented,
            MALFORMED_REASON,
        )],
    )
}

/// `owl:disjointWith` with an anonymous side.
fn disjoint_axiom(
    reader: &mut ExpressionReader<'_>,
    row: &TripleRow,
) -> Result<Option<AxiomLevel>, SchemaCompileError> {
    if named_iri(&row.subject).is_some() && named_iri(&row.object).is_some() {
        return Ok(None);
    }
    let subject = reader.expression(&row.subject, 0)?;
    let object = reader.expression(&row.object, 0)?;
    refuse_data_range_as_class(&subject, &row.subject)?;
    refuse_data_range_as_class(&object, &row.object)?;
    let components = [&subject, &object]
        .into_iter()
        .filter(|side| !matches!(side, OntologyExpression::Named(_)))
        .map(|side| {
            axiom_component(
                side.canonical(),
                SchemaExpressionOutcome::Unrepresented,
                DISJOINT_REASON,
            )
        })
        .collect();
    Ok(Some((
        SchemaCoverageProvenance {
            subject: subject.provenance_subject(),
            predicate: row.predicate.clone(),
            object: object.canonical(),
        },
        components,
    )))
}

fn render_members(members: &[OntologyExpression]) -> String {
    let mut rendered = String::from("members(");
    for (index, member) in members.iter().enumerate() {
        if index > 0 {
            rendered.push(',');
        }
        member.write_canonical(&mut rendered);
    }
    rendered.push(')');
    rendered
}

/// `owl:disjointUnionOf`, or the `owl:members` of an `owl:AllDisjointClasses`,
/// with an anonymous member.
fn members_axiom(
    reader: &mut ExpressionReader<'_>,
    row: &TripleRow,
) -> Result<Option<AxiomLevel>, SchemaCompileError> {
    let owner = row.subject.to_string();
    let mut members = Vec::new();
    for item in reader.list_items(&row.object, &owner)? {
        let member = reader.expression(&item, 1)?;
        refuse_data_range_as_class(&member, &item)?;
        members.push(member);
    }
    if members
        .iter()
        .all(|member| matches!(member, OntologyExpression::Named(_)))
    {
        return Ok(None);
    }
    let reason = if row.predicate == OWL_MEMBERS {
        DISJOINT_REASON
    } else {
        DISJOINT_UNION_REASON
    };
    let components = members
        .iter()
        .filter(|member| !matches!(member, OntologyExpression::Named(_)))
        .map(|member| {
            axiom_component(
                member.canonical(),
                SchemaExpressionOutcome::Unrepresented,
                reason,
            )
        })
        .collect();
    Ok(Some((
        SchemaCoverageProvenance {
            subject: named_iri(&row.subject)
                .map_or_else(|| ANONYMOUS_INDIVIDUAL.to_owned(), str::to_owned),
            predicate: row.predicate.clone(),
            object: render_members(&members),
        },
        components,
    )))
}

/// `owl:hasKey` on an anonymous class expression.
fn has_key_axiom(
    reader: &mut ExpressionReader<'_>,
    row: &TripleRow,
) -> Result<AxiomLevel, SchemaCompileError> {
    let class = reader.expression(&row.subject, 0)?;
    refuse_data_range_as_class(&class, &row.subject)?;
    let owner = class.canonical();
    let mut keys = Vec::new();
    for item in reader.list_items(&row.object, &owner)? {
        keys.push(PropertyExpression::Named(
            named_iri(&item)
                .ok_or_else(|| SchemaCompileError::InvalidOntology {
                    subject: owner.clone(),
                    reason: "owl:hasKey members must be property IRIs".to_owned(),
                })?
                .to_owned(),
        ));
    }
    let mut rendered = String::from("keys(");
    for (index, key) in keys.iter().enumerate() {
        if index > 0 {
            rendered.push(',');
        }
        key.write_canonical(&mut rendered);
    }
    rendered.push(')');
    Ok((
        SchemaCoverageProvenance {
            subject: owner.clone(),
            predicate: row.predicate.clone(),
            object: rendered,
        },
        vec![axiom_component(
            owner,
            SchemaExpressionOutcome::Unrepresented,
            HAS_KEY_REASON,
        )],
    ))
}

/// A class assertion `x a [ … ]` whose class is anonymous.
fn class_assertion_axiom(
    reader: &mut ExpressionReader<'_>,
    row: &TripleRow,
) -> Result<AxiomLevel, SchemaCompileError> {
    // A class assertion whose class is anonymous types an individual; a
    // developer schema describes classes.
    let subject =
        named_iri(&row.subject).map_or_else(|| ANONYMOUS_INDIVIDUAL.to_owned(), str::to_owned);
    let component = if reader.declares_construct(&row.object) {
        let class = reader.expression(&row.object, 0)?;
        refuse_data_range_as_class(&class, &row.object)?;
        axiom_component(
            class.canonical(),
            SchemaExpressionOutcome::Excluded,
            CLASS_ASSERTION_REASON,
        )
    } else {
        axiom_component(
            ANONYMOUS_INDIVIDUAL.to_owned(),
            SchemaExpressionOutcome::Unrepresented,
            NOT_A_CLASS_EXPRESSION_REASON,
        )
    };
    Ok((
        SchemaCoverageProvenance {
            subject,
            predicate: row.predicate.clone(),
            object: component.expression.clone(),
        },
        vec![component],
    ))
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

/// The named properties some carried restriction gives a literal value (a
/// data-range filler or a literal `owl:hasValue`), and those it gives a node
/// value (an individual `owl:hasValue`, or `owl:hasSelf`).
/// Read by the OWL 2 Full (RDF-Based) Semantics, §5.3, an object property of
/// the first kind takes literals and a datatype property of the second kind
/// takes nodes, wherever it is carried. A class filler on a datatype property
/// is not counted: it is read as a literal whose class membership is not
/// judged, as a class range is.
fn cross_kind_valued_properties<'a>(
    axioms: &'a [ClassAxiom],
    datatypes: &BTreeSet<String>,
) -> (BTreeSet<&'a str>, BTreeSet<&'a str>) {
    let mut literal_valued = BTreeSet::new();
    let mut node_valued = BTreeSet::new();
    for axiom in axioms {
        for (_, conjuncts) in &axiom.carriers {
            for conjunct in conjuncts {
                let _: Result<(), ()> = conjunct.visit_restrictions(&mut |on, restriction| {
                    let Some(iri) = on.named() else {
                        return Ok(());
                    };
                    let literal = match restriction {
                        Restriction::HasValue(value) => value.is_literal(),
                        Restriction::HasSelf => false,
                        _ => restriction
                            .fillers()
                            .any(|filler| is_data_range(filler, datatypes)),
                    };
                    let node = match restriction {
                        Restriction::HasValue(value) => !value.is_literal(),
                        Restriction::HasSelf => true,
                        _ => false,
                    };
                    if literal {
                        literal_valued.insert(iri);
                    }
                    if node {
                        node_valued.insert(iri);
                    }
                    Ok(())
                });
            }
        }
    }
    (literal_valued, node_valued)
}

/// Whether `iri` is a datatype without any declaration: an XSD datatype,
/// `rdfs:Literal`, a member of the OWL 2 datatype map outside XSD
/// (`owl:real`, `owl:rational`, `rdf:PlainLiteral`, `rdf:XMLLiteral`), or an
/// RDF 1.2 datatype (`rdf:langString`, `rdf:dirLangString`, `rdf:HTML`,
/// `rdf:JSON`).
pub(crate) fn is_builtin_datatype(iri: &str) -> bool {
    iri.starts_with(crate::model::xsd::BASE) || is_builtin_non_xsd_datatype(iri)
}

/// The builtin datatypes whose IRIs are not in the XSD namespace.
pub(crate) fn is_builtin_non_xsd_datatype(iri: &str) -> bool {
    matches!(
        iri,
        RDFS_LITERAL
            | RDF_LANG_STRING
            | RDF_DIR_LANG_STRING
            | RDF_PLAIN_LITERAL
            | RDF_XML_LITERAL
            | RDF_HTML
            | RDF_JSON
            | OWL_REAL
            | OWL_RATIONAL
    )
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
    let effective = propagate_sets_with_limit(
        &graph,
        seeds,
        "propagated class memberships",
        MAX_SCHEMA_CLASS_MEMBERSHIPS,
    )?;
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

/// The canonical rendering of one carried conjunct, and of a union's members,
/// computed once however many classes inherit the conjunct, keyed by the
/// carried conjunct's address.
#[derive(Debug)]
struct ConjunctInfo {
    canonical: String,
    union_members: Option<BTreeSet<String>>,
}

fn conjunct_infos(class_axioms: &[ClassAxiom]) -> BTreeMap<usize, ConjunctInfo> {
    let mut infos = BTreeMap::new();
    for axiom in class_axioms {
        for (_, conjuncts) in &axiom.carriers {
            for conjunct in conjuncts {
                infos
                    .entry(std::ptr::from_ref(conjunct).addr())
                    .or_insert_with(|| ConjunctInfo {
                        canonical: conjunct.canonical(),
                        union_members: match conjunct {
                            OntologyExpression::Union(members) => {
                                Some(members.iter().map(OntologyExpression::canonical).collect())
                            }
                            _ => None,
                        },
                    });
            }
        }
    }
    infos
}

/// What one eligible class inherits from the anonymous class axioms on it and
/// its superclasses, and which of those it owns: the ones no eligible strict
/// ancestor also inherits.
#[derive(Debug, Default)]
struct ClassExpressionFacts<'a> {
    /// `(axiom index, conjunct)` inherited, canonically ordered.
    entries: Vec<(usize, &'a OntologyExpression)>,
    /// The inherited entries no eligible strict ancestor inherits.
    owned: BTreeSet<(usize, &'a OntologyExpression)>,
    /// The eligible classes strictly above this one (not in its equivalence
    /// cycle).
    ancestors: BTreeSet<&'a str>,
    anonymous: AnonymousSupers<'a>,
    /// Restrictions on one named property, asserted as top-level conjuncts.
    restrictions: BTreeMap<&'a str, BTreeSet<&'a Restriction>>,
    /// Properties a restriction asserted of the class itself restricts: a top-
    /// level conjunct, or a disjunct or conjunct of one.
    admitted: BTreeSet<&'a str>,
    /// Every named property any conjunct restricts, at any depth.
    mentioned: BTreeSet<&'a str>,
    /// Named properties an inherited top-level `owl:hasSelf` restricts: no
    /// schema keyword states it, so the cell is no exact representation.
    self_restricted: BTreeSet<&'a str>,
    /// Owned top-level restrictions on each named property, with their axiom.
    owned_restrictions: BTreeMap<&'a str, Vec<(usize, &'a Restriction)>>,
    /// The nearest owners of restrictions on each property (filled once every
    /// class's ownership is known).
    restriction_owners: BTreeMap<&'a str, Vec<&'a str>>,
}

impl<'a> ClassExpressionFacts<'a> {
    fn new(
        entries: Vec<(usize, &'a OntologyExpression)>,
        owned: BTreeSet<(usize, &'a OntologyExpression)>,
        ancestors: BTreeSet<&'a str>,
        infos: &'a BTreeMap<usize, ConjunctInfo>,
    ) -> Self {
        let mut facts = Self {
            entries,
            owned,
            ancestors,
            ..Self::default()
        };
        for &(_, conjunct) in &facts.entries {
            conjunct.focus_restricted(&mut facts.admitted);
            match conjunct {
                OntologyExpression::Named(_) => continue,
                OntologyExpression::Restriction(on, restriction) => {
                    if let Some(iri) = on.named()
                        && matches!(**restriction, Restriction::HasSelf)
                    {
                        facts.self_restricted.insert(iri);
                    }
                    if let Some(iri) = on.named()
                        && !matches!(**restriction, Restriction::HasSelf)
                    {
                        facts
                            .restrictions
                            .entry(iri)
                            .or_default()
                            .insert(restriction);
                    }
                }
                _ => {}
            }
            if let Some(info) = infos.get(&std::ptr::from_ref(conjunct).addr()) {
                facts.anonymous.canonical.insert(info.canonical.as_str());
                if let Some(members) = &info.union_members {
                    facts.anonymous.unions.push(members);
                }
            }
            let mentioned = &mut facts.mentioned;
            let _: Result<(), ()> = conjunct.visit_restrictions(&mut |on, _| {
                if let Some(iri) = on.named() {
                    mentioned.insert(iri);
                }
                Ok(())
            });
        }
        for &(axiom, conjunct) in &facts.owned {
            if let OntologyExpression::Restriction(on, restriction) = conjunct
                && let Some(iri) = on.named()
                && !matches!(**restriction, Restriction::HasSelf)
            {
                facts
                    .owned_restrictions
                    .entry(iri)
                    .or_default()
                    .push((axiom, restriction));
            }
        }
        facts
    }
}

/// A subset of `classes` every member of `classes` is at or above, visiting
/// the deepest first (a class below another has more supertypes) and keeping
/// each one no kept class is below.
fn lowest_of<'a>(
    classes: &BTreeSet<&'a str>,
    supers_of: &impl Fn(&str) -> &'a BTreeSet<String>,
) -> Vec<&'a str> {
    let mut deepest_first: Vec<&'a str> = classes.iter().copied().collect();
    deepest_first.sort_by_cached_key(|&class| std::cmp::Reverse(supers_of(class).len()));
    let mut kept: Vec<&'a str> = Vec::new();
    for class in deepest_first {
        if !kept.iter().any(|&below| supers_of(below).contains(class)) {
            kept.push(class);
        }
    }
    kept
}

/// The minimal elements of `candidates` under the class hierarchy: those with
/// no other candidate strictly below them, in `candidates`' order.
///
/// A class strictly below another has strictly more eligible strict
/// ancestors, so the candidates are visited deepest first: each is minimal
/// unless it is above one already kept. That compares each candidate with the
/// kept ones only (in a chain, one), not with every other candidate, so the
/// nearest owners along a hierarchy `d` deep cost `O(d log d)`, not `O(d²)`.
fn nearest<'a>(
    candidates: &[&'a str],
    class_facts: &BTreeMap<&str, ClassExpressionFacts<'a>>,
) -> Vec<&'a str> {
    let depth = |class: &str| {
        class_facts
            .get(class)
            .map_or(0, |facts| facts.ancestors.len())
    };
    let mut deepest_first: Vec<&'a str> = candidates.to_vec();
    deepest_first.sort_by_cached_key(|&class| std::cmp::Reverse(depth(class)));
    let mut kept: Vec<&'a str> = Vec::new();
    for candidate in deepest_first {
        let above_a_kept_one = kept.iter().any(|&below| {
            class_facts
                .get(below)
                .is_some_and(|facts| facts.ancestors.contains(candidate))
        });
        if !above_a_kept_one {
            kept.push(candidate);
        }
    }
    let kept: BTreeSet<&str> = kept.into_iter().collect();
    candidates
        .iter()
        .copied()
        .filter(|candidate| kept.contains(candidate))
        .collect()
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
        prior: prior_datatypes,
        definitions: datatype_definitions,
        axioms: datatype_axioms,
    } = datatype_facts;
    let scope = DatatypeScope {
        names: &datatypes,
        definitions: &datatype_definitions,
    };
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
                    unsatisfiable: supertypes
                        .get(class)
                        .is_some_and(|types| types.contains(OWL_NOTHING)),
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

    let infos = conjunct_infos(class_axioms);
    let class_facts = class_expression_facts(class_axioms, &eligible_classes, supertypes, &infos)?;
    // A class carrying an expression empty by its form (`¬(≤1 p.owl:Nothing)`,
    // which is `¬owl:Thing`) admits no instance, as a subclass of
    // `owl:Nothing` does.
    for (class_iri, facts) in &class_facts {
        if facts
            .entries
            .iter()
            .any(|(_, conjunct)| conjunct.is_nothing())
            && let Some(class) = classes.get_mut(*class_iri)
        {
            class.unsatisfiable = true;
        }
    }
    let mut property_templates: BTreeMap<String, SurfaceProperty> = BTreeMap::new();
    let needs_templates = !class_facts.is_empty();
    let no_anonymous = AnonymousSupers::default();
    let mut statuses: BTreeMap<(String, String), SchemaCoverageStatus> = BTreeMap::new();
    let (literal_valued, node_valued) = cross_kind_valued_properties(class_axioms, &datatypes);
    let object_properties: BTreeSet<String> = properties
        .iter()
        .filter(|(_, facts)| facts.kind() == OntologyPropertyKind::Object)
        .map(|(iri, _)| iri.clone())
        .collect();
    let empty_ranged: BTreeSet<String> = properties
        .iter()
        .filter(|(_, facts)| {
            facts
                .ranges
                .iter()
                .any(|range| range.expression.is_nothing())
        })
        .map(|(iri, _)| iri.clone())
        .collect();

    for (property_iri, facts) in properties {
        let mut template_taken = false;
        // Decided once per property rather than once per class.
        let caller_owned = request.namespaces().is_caller_owned(&property_iri);
        let kind = facts.kind();
        let mut datatype_iris = BTreeSet::new();
        for range in &facts.ranges {
            range.expression.named_members(&mut datatype_iris);
        }
        if kind == OntologyPropertyKind::Object {
            datatype_iris.retain(|iri| prior_datatypes.contains(iri));
        } else {
            datatype_iris.retain(|iri| datatypes.contains(iri));
        }
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
        // A range is reported as an approximation when its value schema
        // admits more than the range. An object property projects a named
        // datatype range as a node, as it always has, so only its anonymous
        // ranges are judged that way.
        // A range that admits no value is stated exactly (`false`), and so is
        // every restriction on the property then.
        let empty_range = facts
            .ranges
            .iter()
            .any(|range| range.expression.is_nothing());
        let approximate_range = !empty_range
            && facts.ranges.iter().any(|range| {
                // An object property over a datatype is read by the OWL 2 Full
                // Semantics: literal values, an approximation of its declaration.
                let mut named = BTreeSet::new();
                range.expression.named_members(&mut named);
                if kind == OntologyPropertyKind::Object
                    && named.iter().any(|iri| is_datatype(iri, &datatypes))
                {
                    return true;
                }
                (kind != OntologyPropertyKind::Object || !range.expression.is_named_skeleton())
                    && match value_precision(&range.expression, scope) {
                        ValuePrecision::Judged | ValuePrecision::Approximate => true,
                        // A datatype property's class range admits any literal.
                        ValuePrecision::ClassLike => kind == OntologyPropertyKind::Datatype,
                        ValuePrecision::Exact => false,
                    }
            });
        // Membership in a domain beyond the named hierarchy is read
        // structurally, so an exclusion against one is not a proof.
        let undecided_domain = facts
            .domains
            .iter()
            .any(|domain| !domain.expression.is_named_skeleton());

        for class_iri in &eligible_classes {
            let shape_info = shape_classes.get(class_iri);
            // An ontology without anonymous class expressions skips the lookup.
            let class_expressions = if class_facts.is_empty() {
                None
            } else {
                class_facts.get(class_iri.as_str())
            };
            let has_shape = shape_info
                .is_some_and(|info| info.direct_properties.contains(property_iri.as_str()));
            // A class that a restriction on the property is asserted of carries
            // the property, whatever its declared domain: an instance with a
            // value is in the domain by RDFS, and one the restriction requires
            // a value of is in it by OWL.
            let restricted = class_expressions
                .is_some_and(|expressions| expressions.admitted.contains(property_iri.as_str()));
            // The restriction axioms of the nearest classes that own
            // restrictions on the property (this class, where it owns some):
            // each class's row names what it owns or the nearest owner it
            // references, so provenance grows with the restrictions, not with
            // the depth.
            let owners: &[&str] = class_expressions
                .and_then(|expressions| expressions.restriction_owners.get(property_iri.as_str()))
                .map_or(&[], Vec::as_slice);
            // A class referencing restriction owners adds their axioms; any
            // other cell's provenance is the property's own, copied where each
            // copy is stored (the property entry's, then the row's), so an
            // ontology without restrictions allocates as before.
            let owned_provenance = (!owners.is_empty()).then(|| {
                let mut provenance = base_provenance.clone();
                for &owner in owners {
                    if let Some(owned) = class_facts
                        .get(owner)
                        .and_then(|facts| facts.owned_restrictions.get(property_iri.as_str()))
                    {
                        provenance.extend(
                            owned
                                .iter()
                                .map(|&(axiom, _)| class_axioms[axiom].provenance.clone()),
                        );
                    }
                }
                provenance.sort();
                provenance.dedup();
                provenance
            });

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
            } else if !caller_owned {
                (
                    SchemaCoverageStatus::ExcludedNamespace,
                    SchemaCoveragePrecision::Exact,
                )
            } else if !restricted
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
                    .map_or_else(Vec::new, |set| {
                        set.iter().map(|&restriction| restriction.clone()).collect()
                    });
                let restricted_approximately = restrictions.iter().any(|restriction| {
                    restriction_outcomes(
                        restriction,
                        scope,
                        kind == OntologyPropertyKind::Object,
                        empty_range,
                    )
                    .iter()
                    .any(|(outcome, _)| *outcome != SchemaExpressionOutcome::Projected)
                });
                // A self restriction, which no schema keyword states, and a
                // property whose values another class's restriction widens
                // to the other kind (OWL 2 Full) are approximations too.
                let self_restricted = class_expressions.is_some_and(|expressions| {
                    expressions.self_restricted.contains(property_iri.as_str())
                });
                let cross_kind = (kind == OntologyPropertyKind::Object
                    && literal_valued.contains(property_iri.as_str()))
                    || (kind == OntologyPropertyKind::Datatype
                        && node_valued.contains(property_iri.as_str()));
                // A range that admits no value states the cell exactly: no
                // value, widened or not, meets `false`.
                let precision = if !self_restricted
                    && (empty_range
                        || (facts.functional.is_empty()
                            && !restricted_approximately
                            && !approximate_range
                            && !cross_kind))
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
                        provenance: owned_provenance
                            .as_ref()
                            .unwrap_or(&base_provenance)
                            .clone(),
                        restrictions,
                        restriction_owners: owners.iter().map(|&owner| owner.to_owned()).collect(),
                        takes_literals: kind == OntologyPropertyKind::Object
                            && literal_valued.contains(property_iri.as_str()),
                        takes_nodes: kind == OntologyPropertyKind::Datatype
                            && node_valued.contains(property_iri.as_str()),
                    },
                );
                // Fragments need a template only where some class owns
                // restrictions; an ontology without them pays nothing here.
                if needs_templates && !template_taken {
                    template_taken = true;
                    let mut template = class.properties[&property_iri].clone();
                    template.restrictions.clear();
                    template.restriction_owners.clear();
                    property_templates.insert(property_iri.clone(), template);
                }
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
                provenance: owned_provenance.unwrap_or_else(|| base_provenance.clone()),
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
            outcomes.insert(if !caller_owned {
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

    let (class_expressions, fragments) = class_expression_report(
        &ReportInputs {
            mode: request.mode(),
            class_axioms,
            class_facts: &class_facts,
            statuses: &statuses,
            supertypes,
            datatypes: scope,
            infos: &infos,
            object_properties: &object_properties,
            empty_ranged: &empty_ranged,
        },
        &mut classes,
    )?;
    let class_expressions = with_axiom_level_components(class_expressions, &datatype_axioms);

    let surface = SchemaSurface {
        classes,
        report: SchemaCoverageReport {
            mode: request.mode(),
            properties: report_properties,
        },
        class_expressions,
        datatypes,
        datatype_definitions,
        fragments,
        property_templates,
    };
    surface.assert_conservation();
    Ok(surface)
}

/// The anonymous class axiom conjuncts each eligible class inherits: those
/// carried by the class itself and by every one of its superclasses.
fn class_expression_facts<'a>(
    class_axioms: &'a [ClassAxiom],
    eligible_classes: &'a [String],
    supertypes: &'a BTreeMap<String, BTreeSet<String>>,
    infos: &'a BTreeMap<usize, ConjunctInfo>,
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
    let eligible: BTreeSet<&str> = eligible_classes.iter().map(String::as_str).collect();
    let no_supertypes = BTreeSet::new();
    let supers_of = |class: &str| supertypes.get(class).unwrap_or(&no_supertypes);
    let mut assertions = 0_usize;
    for class_iri in eligible_classes {
        let Some(types) = supertypes.get(class_iri) else {
            continue;
        };
        // The eligible classes strictly above: a supertype in the class's own
        // equivalence cycle is not above it.
        let ancestors: BTreeSet<&str> = types
            .iter()
            .map(String::as_str)
            .filter(|&supertype| {
                supertype != class_iri
                    && eligible.contains(supertype)
                    && !supers_of(supertype).contains(class_iri)
            })
            .collect();
        let mut entries: BTreeSet<(usize, &OntologyExpression)> = BTreeSet::new();
        let mut owned: BTreeSet<(usize, &OntologyExpression)> = BTreeSet::new();
        // The ancestors no other ancestor is below, found once and only when
        // an ineligible carrier asks: every ancestor is above one of them, so
        // one of them reaches whatever any ancestor reaches.
        let mut lowest_ancestors: Option<Vec<&str>> = None;
        // Every class is a subclass of `owl:Thing`.
        for supertype in types
            .iter()
            .map(String::as_str)
            .chain(std::iter::once(OWL_THING))
        {
            let Some(list) = carried.get(supertype) else {
                continue;
            };
            entries.extend(list.iter().copied());
            // Owned when no eligible strict ancestor inherits it too: the
            // carrier is this class or in its cycle, or an ineligible carrier
            // (or `owl:Thing`) no eligible strict ancestor reaches.
            let owns = if supertype == OWL_THING {
                ancestors.is_empty()
            } else if eligible.contains(supertype) {
                !ancestors.contains(supertype)
            } else {
                !lowest_ancestors
                    .get_or_insert_with(|| lowest_of(&ancestors, &supers_of))
                    .iter()
                    .any(|&ancestor| supers_of(ancestor).contains(supertype))
            };
            if owns {
                owned.extend(list.iter().copied());
            }
        }
        if entries.is_empty() {
            continue;
        }
        // Each class holds references to the conjuncts it inherits, not
        // copies; the bound keeps that bookkeeping proportionate to the
        // ontology (a 50,000-class tree 15 deep with two restrictions per
        // class holds 1.5 million).
        assertions = assertions.saturating_add(entries.len());
        enforce_limit(
            "inherited class-expression assertions",
            assertions,
            MAX_SCHEMA_CLASS_MEMBERSHIPS,
        )?;
        facts.insert(
            class_iri.as_str(),
            ClassExpressionFacts::new(entries.into_iter().collect(), owned, ancestors, infos),
        );
    }
    // The nearest owners of each restricted property, now that every class's
    // ownership is known.
    let mut owners: BTreeMap<&str, BTreeMap<&str, Vec<&str>>> = BTreeMap::new();
    for (&class_iri, class) in &facts {
        let mut per_property: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
        for &property in class.restrictions.keys() {
            let candidates: Vec<&str> = std::iter::once(class_iri)
                .chain(class.ancestors.iter().copied())
                .filter(|owner| {
                    facts
                        .get(owner)
                        .is_some_and(|facts| facts.owned_restrictions.contains_key(property))
                })
                .collect();
            per_property.insert(property, nearest(&candidates, &facts));
        }
        owners.insert(class_iri, per_property);
    }
    for (class_iri, per_property) in owners {
        if let Some(class) = facts.get_mut(class_iri) {
            class.restriction_owners = per_property;
        }
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
const EMPTY_COMPLEMENT_REASON: &str = "the complement of an expression every individual meets \
     admits none, so the class admits no instance";
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
const ALL_REASON: &str =
    "every value is held to the filler's value schema, which states it exactly";
const ALL_NOTHING_REASON: &str =
    "owl:Nothing admits no value, so the property is absent on every instance";
const SOME_NOTHING_REASON: &str =
    "the filler admits no value, so no instance meets the restriction and the class admits none";
const ALL_OBJECT_DATA_RANGE_REASON: &str = "every value is held to the data range's literals or is \
     a node: under the OWL 2 RDF-Based Semantics an IRI may denote a data value, which no \
     schema keyword judges";
const ALL_CLASS_REASON: &str = "every value is held to the property's kind of value, as a class \
     rdfs:range is (a node reference, or for an owl:DatatypeProperty, read by the OWL 2 Full \
     Semantics, a literal); the value's class membership is not visible at the value";
const ALL_APPROXIMATE_REASON: &str = "every value is held to the filler's value schema, which \
     admits more than the filler: a referenced node's class membership is not visible at the value";
const HAS_VALUE_REASON: &str = "required, with the value among the property's values: the \
     projection reads OWL's open-world existential closed-world";
const HAS_VALUE_ANONYMOUS_REASON: &str = "required; the anonymous individual has no stable @id, \
     so the value itself is not pinned";
const TRIVIAL_MIN_REASON: &str = "a minimum of zero constrains nothing";
const EMPTY_RANGE_REQUIRED_REASON: &str = "the property's range admits no value, so no instance \
     meets a restriction that needs one and the class admits none";
const EMPTY_RANGE_TRIVIAL_REASON: &str =
    "the property's range admits no value, so a restriction needing none holds of every instance";
const TRIVIAL_NOTHING_REASON: &str =
    "no value meets the empty qualifier, so a bound of no more values over it constrains nothing";
const MIN_REASON: &str = "required, with at least the minimum number of values: the projection \
     reads OWL's open-world minimum closed-world";
const QUALIFIED_MIN_REASON: &str = "required, with at least the minimum number of values in the \
     qualifier: the projection reads OWL's open-world minimum closed-world";
const MAX_REASON: &str = "at most the maximum number of values: the projection counts distinct \
     terms, a unique-name reading of OWL's maximum";
const QUALIFIED_MAX_REASON: &str = "at most the maximum number of values in the qualifier: the \
     projection counts distinct terms, a unique-name reading of OWL's maximum";
const ALL_RATIONAL_REASON: &str = "every value meets the data range, judged by value; a literal \
     typed owl:rational is admitted without judging whether its value is in the range";
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
    datatypes: DatatypeScope<'_>,
    object_property: bool,
    empty_range: bool,
) -> Vec<(SchemaExpressionOutcome, &'static str)> {
    use SchemaExpressionOutcome::{Approximated, Projected, Unrepresented};
    match restriction {
        // A property whose range admits no value has none: a restriction
        // that needs one leaves the class no instance, and any other holds.
        Restriction::HasSelf => vec![(Unrepresented, HAS_SELF_REASON)],
        Restriction::SomeValues(_)
        | Restriction::HasValue(_)
        | Restriction::Min(1.., _)
        | Restriction::Exact(1.., _)
            if empty_range =>
        {
            vec![(Projected, EMPTY_RANGE_REQUIRED_REASON)]
        }
        _ if empty_range => vec![(Projected, EMPTY_RANGE_TRIVIAL_REASON)],
        // An empty filler is stated exactly: no value meets it.
        Restriction::SomeValues(filler) if filler.is_nothing() => {
            vec![(Projected, SOME_NOTHING_REASON)]
        }
        Restriction::Min(count, Some(qualifier)) if *count > 0 && qualifier.is_nothing() => {
            vec![(Projected, SOME_NOTHING_REASON)]
        }
        Restriction::Exact(count, Some(qualifier)) if *count > 0 && qualifier.is_nothing() => {
            vec![(Projected, SOME_NOTHING_REASON)]
        }
        Restriction::Max(_, Some(qualifier)) | Restriction::Exact(0, Some(qualifier))
            if qualifier.is_nothing() =>
        {
            vec![(Projected, TRIVIAL_NOTHING_REASON)]
        }
        Restriction::SomeValues(_) => vec![(Approximated, SOME_REASON)],
        Restriction::AllValues(filler) if filler.is_nothing() => {
            vec![(Projected, ALL_NOTHING_REASON)]
        }
        // An object property's data-range filler also admits nodes, unjudged:
        // under the OWL 2 RDF-Based Semantics an IRI may denote a data value.
        Restriction::AllValues(filler)
            if object_property && is_data_range(filler, datatypes.names) =>
        {
            vec![(Approximated, ALL_OBJECT_DATA_RANGE_REASON)]
        }
        Restriction::AllValues(filler) => match value_precision(filler, datatypes) {
            ValuePrecision::Exact => vec![(Projected, ALL_REASON)],
            ValuePrecision::Judged => vec![(Approximated, ALL_RATIONAL_REASON)],
            ValuePrecision::ClassLike => vec![(Approximated, ALL_CLASS_REASON)],
            ValuePrecision::Approximate => vec![(Approximated, ALL_APPROXIMATE_REASON)],
        },
        Restriction::HasValue(value) => {
            if value.is_anonymous() {
                vec![(Approximated, HAS_VALUE_ANONYMOUS_REASON)]
            } else {
                vec![(Approximated, HAS_VALUE_REASON)]
            }
        }
        Restriction::Min(0, _) => vec![(Projected, TRIVIAL_MIN_REASON)],
        Restriction::Min(_, None) => vec![(Approximated, MIN_REASON)],
        Restriction::Min(_, Some(_)) => vec![(Approximated, QUALIFIED_MIN_REASON)],
        Restriction::Max(_, None) => vec![(Approximated, MAX_REASON)],
        Restriction::Max(_, Some(qualifier)) => {
            if counts_exactly(qualifier, datatypes) {
                vec![(Approximated, QUALIFIED_MAX_REASON)]
            } else {
                vec![(Unrepresented, QUALIFIED_MAX_CLASS_REASON)]
            }
        }
        Restriction::Exact(_, None) => vec![(Approximated, EXACT_REASON)],
        Restriction::Exact(count, Some(qualifier)) => {
            if counts_exactly(qualifier, datatypes) {
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
    datatypes: DatatypeScope<'c>,
    /// The `owl:ObjectProperty` IRIs, whose data-range fillers admit nodes.
    object_properties: &'c BTreeSet<String>,
    /// The properties a range admits no value of (`owl:Nothing`, say).
    empty_ranged: &'c BTreeSet<String>,
}

impl ConjunctContext<'_> {
    fn is_object(&self, property_iri: &str) -> bool {
        self.object_properties.contains(property_iri)
    }

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
                if restriction_outcomes(
                    restriction,
                    self.datatypes,
                    self.is_object(iri),
                    self.empty_ranged.contains(iri),
                )
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
    fn classify(
        &self,
        conjunct: &OntologyExpression,
        expression: &str,
    ) -> Vec<(SchemaExpressionComponent, bool)> {
        use SchemaExpressionOutcome::{Approximated, Excluded, Projected, Unrepresented};
        let restricted = match conjunct {
            OntologyExpression::Restriction(on, _) => on.named().map(str::to_owned),
            _ => None,
        };
        let component = |outcome, reason: &str| SchemaExpressionComponent {
            expression: expression.to_owned(),
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
                    Some(SchemaCoverageStatus::IncludedUnshaped) => restriction_outcomes(
                        restriction,
                        self.datatypes,
                        self.is_object(iri),
                        self.empty_ranged.contains(iri),
                    )
                    .into_iter()
                    .map(|(outcome, reason)| (component(outcome, reason), false))
                    .collect(),
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
            // The complement of what every individual meets admits none.
            OntologyExpression::Complement(_) if conjunct.is_nothing() => {
                vec![(component(Projected, EMPTY_COMPLEMENT_REASON), false)]
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
                .flat_map(|member| self.classify(member, &member.canonical()))
                .collect(),
            OntologyExpression::DatatypeRestriction(..)
            | OntologyExpression::DatatypeComplement(_) => {
                vec![(component(Unrepresented, DATA_RANGE_REASON), false)]
            }
        }
    }
}

/// Add each anonymous datatype definition to the manifest as an axiom whose
/// one component is the defining data range.
fn with_axiom_level_components(
    mut report: SchemaClassExpressionReport,
    axioms: &[AxiomLevel],
) -> SchemaClassExpressionReport {
    if axioms.is_empty() {
        return report;
    }
    // One axiom row per provenance: an axiom-level component joins the row
    // its provenance already has.
    let mut rows: BTreeMap<SchemaCoverageProvenance, SchemaClassExpressionAxiom> = report
        .axioms
        .drain(..)
        .map(|axiom| (axiom.provenance.clone(), axiom))
        .collect();
    for (provenance, components) in axioms {
        let row = rows
            .entry(provenance.clone())
            .or_insert_with(|| SchemaClassExpressionAxiom {
                provenance: provenance.clone(),
                components: Vec::new(),
                classes: Vec::new(),
            });
        row.components.extend(
            components
                .iter()
                .map(|component| in_mode(report.mode, component.clone())),
        );
        row.components.sort();
        row.components.dedup();
    }
    report.axioms = rows.into_values().collect();
    report
}

/// A component as `mode` carries it: shaped-only mode projects no ontology
/// axiom, so what would be projected or approximated is excluded there.
fn in_mode(
    mode: SchemaSurfaceMode,
    mut component: SchemaExpressionComponent,
) -> SchemaExpressionComponent {
    if mode == SchemaSurfaceMode::ShapedOnly
        && matches!(
            component.outcome,
            SchemaExpressionOutcome::Projected | SchemaExpressionOutcome::Approximated
        )
    {
        component.outcome = SchemaExpressionOutcome::Excluded;
        SHAPED_ONLY_REASON.clone_into(&mut component.reason);
    }
    component
}

/// The most fragments one fragment reaches through its chain of references.
/// A validator follows the chain at one instance location, and bounds it (the
/// purrdf-jsonschema validator stops past 250), so a hierarchy deeper than this
/// restarts the chain with a fragment that states its ancestors' restrictions
/// itself.
pub(crate) const MAX_FRAGMENT_CHAIN: usize = 64;

/// Keep every fragment's chain of references at most [`MAX_FRAGMENT_CHAIN`]
/// long. Visiting owners shallowest first, a fragment whose chain would grow
/// past the bound states the distinct restrictions (or disjunctions) of every
/// fragment it reaches, and references none. In a subclass chain, one
/// fragment in every [`MAX_FRAGMENT_CHAIN`] + 1 states the distinct
/// restrictions above it; the others reference their parent as before.
fn bound_fragment_chains(
    fragments: &mut Fragments,
    class_facts: &BTreeMap<&str, ClassExpressionFacts<'_>>,
) {
    let mut shallowest_first: Vec<(String, Option<String>)> = fragments.keys().cloned().collect();
    // A fragment's parents are strict ancestors of its owner, with fewer
    // ancestors of their own, so they come first.
    shallowest_first.sort_by_cached_key(|(owner, _)| {
        class_facts
            .get(owner.as_str())
            .map_or(0, |facts| facts.ancestors.len())
    });
    let mut chain: BTreeMap<(String, Option<String>), usize> = BTreeMap::new();
    for key in shallowest_first {
        let slot = &key.1;
        let length = 1 + fragments[&key]
            .parents
            .iter()
            .filter_map(|parent| chain.get(&(parent.clone(), slot.clone())).copied())
            .max()
            .unwrap_or(0);
        if length <= MAX_FRAGMENT_CHAIN {
            chain.insert(key, length);
            continue;
        }
        let mut restrictions: BTreeSet<Restriction> = BTreeSet::new();
        let mut disjunctions: BTreeSet<OntologyExpression> = BTreeSet::new();
        let mut reached: BTreeSet<String> = BTreeSet::new();
        let mut pending = vec![key.0.clone()];
        while let Some(owner) = pending.pop() {
            if !reached.insert(owner.clone()) {
                continue;
            }
            if let Some(fragment) = fragments.get(&(owner, slot.clone())) {
                restrictions.extend(fragment.restrictions.iter().cloned());
                disjunctions.extend(fragment.disjunctions.iter().cloned());
                pending.extend(fragment.parents.iter().cloned());
            }
        }
        if let Some(fragment) = fragments.get_mut(&key) {
            fragment.restrictions = restrictions.into_iter().collect();
            fragment.disjunctions = disjunctions.into_iter().collect();
            fragment.parents.clear();
        }
        chain.insert(key, 1);
    }
}

/// What the class-expression manifest is built from.
struct ReportInputs<'r, 'a> {
    mode: SchemaSurfaceMode,
    class_axioms: &'r [ClassAxiom],
    class_facts: &'r BTreeMap<&'a str, ClassExpressionFacts<'a>>,
    statuses: &'r BTreeMap<(String, String), SchemaCoverageStatus>,
    supertypes: &'r BTreeMap<String, BTreeSet<String>>,
    datatypes: DatatypeScope<'r>,
    infos: &'r BTreeMap<usize, ConjunctInfo>,
    object_properties: &'r BTreeSet<String>,
    empty_ranged: &'r BTreeSet<String>,
}

type Fragments = BTreeMap<(String, Option<String>), Fragment>;

/// Classify what each class owns, and each inherited restriction whose
/// outcome on the class differs from its owner's; route the class-level
/// projections onto the surface; build the fragments the classes reference;
/// and assemble the manifest. A class that inherits a component with its
/// owner's outcome has no row of its own for it: its row is its owner's,
/// reached through the class hierarchy, so the manifest grows with the
/// restrictions rather than with the depth.
#[allow(
    clippy::too_many_lines,
    reason = "one pass over every class's owned and inherited conjuncts"
)]
fn class_expression_report(
    inputs: &ReportInputs<'_, '_>,
    classes: &mut BTreeMap<String, SurfaceClass>,
) -> Result<(SchemaClassExpressionReport, Fragments), SchemaCompileError> {
    let ReportInputs {
        mode,
        class_axioms,
        class_facts,
        statuses,
        supertypes,
        datatypes,
        infos,
        object_properties,
        empty_ranged,
    } = *inputs;
    let no_supertypes = BTreeSet::new();
    let mut per_axiom: Vec<BTreeMap<&str, BTreeSet<SchemaExpressionComponent>>> =
        vec![BTreeMap::new(); class_axioms.len()];
    let mut projected_disjunctions: BTreeMap<&str, Vec<OntologyExpression>> = BTreeMap::new();
    // The classes that state their disjunctions on their own definition
    // rather than through their owners' fragments.
    let mut inline_disjunctions: BTreeSet<&str> = BTreeSet::new();
    let mut cells = 0_usize;
    // The classes owning each inherited conjunct, in class order, so that a
    // class finds its first owning ancestor without scanning its ancestors.
    let mut owners_of: BTreeMap<(usize, &OntologyExpression), Vec<&str>> = BTreeMap::new();
    for (&class_iri, facts) in class_facts {
        for &entry in &facts.owned {
            owners_of.entry(entry).or_default().push(class_iri);
        }
    }
    let owner_of = |facts: &ClassExpressionFacts<'_>, axiom: usize, conjunct| {
        owners_of.get(&(axiom, conjunct)).and_then(|owners| {
            owners
                .iter()
                .copied()
                .find(|owner| facts.ancestors.contains(owner))
        })
    };
    for (&class_iri, facts) in class_facts {
        let context = ConjunctContext {
            mode,
            class_iri,
            supertypes: supertypes.get(class_iri).unwrap_or(&no_supertypes),
            statuses,
            datatypes,
            object_properties,
            empty_ranged,
        };
        let mut focus: BTreeSet<OntologyExpression> = BTreeSet::new();
        let mut unrepresented: BTreeSet<String> = BTreeSet::new();
        // Whether an inherited disjunction classifies on this class otherwise
        // than on its owner, so that the owner's fragment cannot stand for it.
        let mut divergent = false;
        for &(axiom, conjunct) in &facts.entries {
            let owned = facts.owned.contains(&(axiom, conjunct));
            let record = owned
                || match conjunct {
                    // A disjunction the class's own hierarchy entails, though
                    // its owner's does not, differs from the owner's outcome.
                    OntologyExpression::Union(_) => {
                        owner_of(facts, axiom, conjunct).is_none_or(|owner| {
                            let owner_context = ConjunctContext {
                                mode,
                                class_iri: owner,
                                supertypes: supertypes.get(owner).unwrap_or(&no_supertypes),
                                statuses,
                                datatypes,
                                object_properties,
                                empty_ranged,
                            };
                            let outcomes = |components: Vec<(SchemaExpressionComponent, bool)>| {
                                components
                                    .into_iter()
                                    .map(|(component, _)| (component.outcome, component.reason))
                                    .collect::<Vec<_>>()
                            };
                            outcomes(context.classify(conjunct, ""))
                                != outcomes(owner_context.classify(conjunct, ""))
                        })
                    }
                    OntologyExpression::Restriction(on, _) => on.named().is_some_and(|property| {
                        let key = (property.to_owned(), class_iri.to_owned());
                        owner_of(facts, axiom, conjunct).is_none_or(|owner| {
                            statuses.get(&key)
                                != statuses.get(&(property.to_owned(), owner.to_owned()))
                        })
                    }),
                    _ => false,
                };
            if record && !owned && matches!(conjunct, OntologyExpression::Union(_)) {
                divergent = true;
            }
            let inline_focus = matches!(
                conjunct,
                OntologyExpression::OneOf(_) | OntologyExpression::Complement(_)
            );
            if !record && !inline_focus {
                continue;
            }
            let expression = infos
                .get(&std::ptr::from_ref(conjunct).addr())
                .map_or_else(|| conjunct.canonical(), |info| info.canonical.clone());
            for (component, on_focus) in context.classify(conjunct, &expression) {
                if on_focus {
                    if inline_focus {
                        focus.insert(conjunct.clone());
                    } else if owned {
                        projected_disjunctions
                            .entry(class_iri)
                            .or_default()
                            .push(conjunct.clone());
                    }
                }
                if !record {
                    continue;
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
        if divergent {
            // A fragment reference would enforce an inherited disjunction the
            // class reports differently (one a direct or closed SHACL shape
            // owns a property of, say), or omit one it projects although its
            // owner cannot. The class states its own disjunctions instead:
            // exactly those it projects on its focus node.
            inline_disjunctions.insert(class_iri);
            for &(_, conjunct) in &facts.entries {
                if matches!(conjunct, OntologyExpression::Union(_))
                    && context
                        .classify(conjunct, "")
                        .iter()
                        .any(|&(_, on_focus)| on_focus)
                {
                    focus.insert(conjunct.clone());
                }
            }
        }
        if let Some(class) = classes.get_mut(class_iri) {
            class.focus = focus.into_iter().collect();
            class.unrepresented = unrepresented.into_iter().collect();
        }
    }

    // The fragments each owner holds, each referencing the nearest owning
    // ancestors for the same slot.
    let mut fragments: Fragments = BTreeMap::new();
    for (&class_iri, facts) in class_facts {
        for (&property, owned) in &facts.owned_restrictions {
            let candidates: Vec<&str> = facts
                .ancestors
                .iter()
                .copied()
                .filter(|ancestor| {
                    class_facts
                        .get(ancestor)
                        .is_some_and(|facts| facts.owned_restrictions.contains_key(property))
                })
                .collect();
            let mut restrictions: Vec<Restriction> = owned
                .iter()
                .map(|&(_, restriction)| restriction.clone())
                .collect();
            restrictions.sort();
            restrictions.dedup();
            fragments.insert(
                (class_iri.to_owned(), Some(property.to_owned())),
                Fragment {
                    restrictions,
                    disjunctions: Vec::new(),
                    parents: nearest(&candidates, class_facts)
                        .into_iter()
                        .map(str::to_owned)
                        .collect(),
                },
            );
        }
        let candidates: Vec<&str> = std::iter::once(class_iri)
            .chain(facts.ancestors.iter().copied())
            .filter(|owner| projected_disjunctions.contains_key(owner))
            .collect();
        if inline_disjunctions.contains(class_iri) {
            continue;
        }
        if let Some(class) = classes.get_mut(class_iri) {
            class.disjunction_owners = nearest(&candidates, class_facts)
                .into_iter()
                .map(str::to_owned)
                .collect();
        }
    }
    for (&class_iri, disjunctions) in &projected_disjunctions {
        let candidates: Vec<&str> = class_facts.get(class_iri).map_or_else(Vec::new, |facts| {
            facts
                .ancestors
                .iter()
                .copied()
                .filter(|ancestor| projected_disjunctions.contains_key(ancestor))
                .collect()
        });
        let mut disjunctions = disjunctions.clone();
        disjunctions.sort();
        disjunctions.dedup();
        fragments.insert(
            (class_iri.to_owned(), None),
            Fragment {
                restrictions: Vec::new(),
                disjunctions,
                parents: nearest(&candidates, class_facts)
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
            },
        );
    }

    bound_fragment_chains(&mut fragments, class_facts);

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
        entry.components.extend(
            axiom
                .uncarried
                .iter()
                .map(|component| in_mode(mode, component.clone())),
        );
        for (class_iri, components) in rows {
            entry.classes.push(SchemaClassExpressionCoverage {
                class_iri: class_iri.to_owned(),
                components: components.into_iter().collect(),
            });
        }
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
    Ok((SchemaClassExpressionReport { mode, axioms }, fragments))
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

    /// Whether `ex:x` in `data` validates against `#/$defs/A` of the schema
    /// compiled from `ontology_body` (no shapes, ontology-complete).
    fn accepts_data(ontology_body: &str, data: &str) -> bool {
        use purrdf_lex::json::Value;
        let shapes = from_dataset(
            &crate::text_ingest::parse_turtle_to_dataset(PREFIXES, None).expect("shapes"),
        )
        .expect("shape graph");
        let ontology = crate::text_ingest::parse_turtle_to_dataset(
            &format!("{PREFIXES}{ontology_body}"),
            None,
        )
        .expect("ontology Turtle");
        let compiled = crate::json_schema::compile_schema(&SchemaCompileRequest::new(
            &shapes,
            &namespaces(),
            ontology.as_ref(),
            SchemaSurfaceMode::OntologyComplete,
        ))
        .expect("compiles");
        let data = crate::text_ingest::parse_turtle_to_dataset(&format!("{PREFIXES}{data}"), None)
            .expect("data Turtle");
        let projected = crate::instance::project_graph(&data, &namespaces());
        let node = projected["@graph"]
            .as_array()
            .expect("@graph")
            .iter()
            .find(|node| node["@id"] == format!("{EXS}x").as_str())
            .expect("ex:x is projected")
            .clone();
        let metaschemas = purrdf_jsonschema::Metaschemas::new(
            purrdf_testkit::jsonschema_metaschemas::DRAFT_2020_12
                .iter()
                .map(|&(uri, text)| {
                    let document: Value = purrdf_lex::json::read(text).expect("meta-schema");
                    (uri, document)
                }),
        )
        .expect("meta-schemas");
        let schema: Value =
            purrdf_lex::json::read(&compiled.compiled.schema_json).expect("schema JSON");
        let mut registry = purrdf_jsonschema::Registry::with_metaschemas(&metaschemas);
        registry
            .add_resource("mem:///s.json", schema)
            .expect("registers");
        registry
            .compile("mem:///s.json#/$defs/A")
            .expect("compiles")
            .is_valid(&node)
            .expect("evaluates")
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
    fn property_kinds_over_the_other_kind_of_range_are_read_by_owl_2_full() {
        // OWL 2 RDF-Based Semantics §5.3: an object property over a datatype
        // takes literals, and a datatype property over a class takes literals
        // whose class membership is not judged; both are approximations.
        for (ontology, property) in [
            ("ex:p a owl:ObjectProperty ; rdfs:range xsd:string .", "p"),
            (
                "ex:Person a owl:Class .
                 ex:q a owl:DatatypeProperty ; rdfs:range ex:Person .",
                "q",
            ),
        ] {
            let read = surface("", ontology, SchemaSurfaceMode::OntologyComplete)
                .unwrap_or_else(|error| panic!("{ontology} is read: {error:?}"));
            let row = read
                .report
                .properties
                .iter()
                .find(|row| row.property_iri == format!("{EXS}{property}"))
                .expect("the property row");
            assert!(
                row.classes
                    .iter()
                    .all(|cell| cell.precision
                        == SchemaCoveragePrecision::RepresentationApproximation),
                "{ontology}"
            );
        }
        // Neighbour: a datatype property over a datatype is exact.
        let exact = surface(
            "",
            "ex:A a owl:Class . ex:r a owl:DatatypeProperty ; rdfs:domain ex:A ;
                 rdfs:range xsd:string .",
            SchemaSurfaceMode::OntologyComplete,
        )
        .expect("a datatype property over a datatype");
        assert_eq!(
            exact.report.properties[0].classes[0].precision,
            SchemaCoveragePrecision::Exact
        );
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

    /// The restrictions an expression is the conjunction of, canonically.
    fn conjuncts(expression: &OntologyExpression) -> Vec<String> {
        match expression {
            OntologyExpression::Intersection(members) => {
                members.iter().map(OntologyExpression::canonical).collect()
            }
            other => vec![other.canonical()],
        }
    }

    /// The single anonymous superclass expression `A` is asserted to have.
    fn superclass_expression(ontology: &str) -> OntologyExpression {
        let dataset =
            crate::text_ingest::parse_turtle_to_dataset(&format!("{PREFIXES}\n{ontology}"), None)
                .expect("Turtle");
        let objects = objects_of(
            &dataset,
            &Term::NamedNode(NamedNode::from(format!("{EXS}A").as_str())),
            rdfs::SUB_CLASS_OF,
        );
        let [object] = objects.as_slice() else {
            panic!("one superclass: {objects:?}");
        };
        ExpressionReader::new(&dataset)
            .expression(object, 0)
            .expect("the expression reads")
    }

    #[test]
    fn repeated_facet_values_on_one_restriction_node_read_as_their_conjunction() {
        // OWL 2 RDF-Based Semantics §5.6 gives the node each facet's class
        // extension, so they coincide, and the node is their conjunction: the
        // reading an OWL 1 node with both a minimum and a maximum gets.
        for (node, expected) in [
            (
                "owl:minCardinality 1 , 2",
                vec![format!("min(1,<{EXS}p>)"), format!("min(2,<{EXS}p>)")],
            ),
            (
                "owl:someValuesFrom ex:B ; owl:allValuesFrom ex:C",
                vec![
                    format!("all(<{EXS}p>,<{EXS}C>)"),
                    format!("some(<{EXS}p>,<{EXS}B>)"),
                ],
            ),
            (
                "owl:someValuesFrom ex:B , ex:C",
                vec![
                    format!("some(<{EXS}p>,<{EXS}B>)"),
                    format!("some(<{EXS}p>,<{EXS}C>)"),
                ],
            ),
        ] {
            let ontology = format!(
                "ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:p ; {node} ] ."
            );
            let mut read = conjuncts(&superclass_expression(&ontology));
            read.sort();
            let mut expected = expected;
            expected.sort();
            assert_eq!(read, expected, "{node}");
            complete(&ontology).expect("a multi-valued restriction node compiles");
        }
        // Each value is still judged: a repeated facet with one ill-typed value
        // is refused, beside its well-typed neighbour.
        refusal_with_neighbour(
            "ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:p ;
                owl:minCardinality 1 , \"two\" ] .",
            "non-negative integer literal",
            "ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:p ;
                owl:minCardinality 1 , 2 ] .",
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
            "owl:minCardinality \" 3 \"^^xsd:integer",
            "owl:minCardinality 99999999999999999999999",
            "owl:maxCardinality 18446744073709551616",
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
        // Two qualifiers on one node qualify the cardinality each, as their
        // conjunction.
        let both = "ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:p ;
                owl:minQualifiedCardinality 1 ; owl:onClass ex:B , ex:C ] .";
        let mut read = conjuncts(&superclass_expression(both));
        read.sort();
        assert_eq!(
            read,
            vec![
                format!("min(1,<{EXS}p>,<{EXS}B>)"),
                format!("min(1,<{EXS}p>,<{EXS}C>)")
            ]
        );
        complete(both).expect("two qualifiers compile");
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
        // A class-only construct beside a data-range-only one is a node that
        // is both a class expression and a data range (OWL 2 Mapping §3.2.1).
        refusal_with_neighbour(
            "ex:p a owl:DatatypeProperty .
             ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:p ; owl:allValuesFrom
                 [ owl:complementOf ex:B ; owl:datatypeComplementOf xsd:string ] ] .",
            "a node is at most one of the two",
            "ex:p a owl:DatatypeProperty .
             ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:p ; owl:allValuesFrom
                 [ owl:datatypeComplementOf xsd:string ] ] .",
        );
        // Neighbour: two class constructs on one node are its conjunction.
        let mixed = "ex:A rdfs:subClassOf [ owl:unionOf ( ex:B ex:C ) ; owl:onProperty ex:p ;
                owl:someValuesFrom ex:B ] .";
        let mut read = conjuncts(&superclass_expression(mixed));
        read.sort();
        assert_eq!(
            read,
            vec![
                format!("some(<{EXS}p>,<{EXS}B>)"),
                format!("union(<{EXS}B>,<{EXS}C>)")
            ]
        );
        complete(mixed).expect("a union and a restriction on one node compile");
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
    fn cross_kind_fillers_and_values_are_read_by_owl_2_full() {
        // A class filler on a datatype property, and a data-range filler on an
        // object property, are read by the OWL 2 Full Semantics, not refused.
        // Each is judged on data: the datatype property takes literals, the
        // object property the restricted range's literals.
        let class_filler = "ex:B a owl:Class .
             ex:p a owl:DatatypeProperty .
             ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:p ; owl:someValuesFrom ex:B ] .";
        assert!(accepts_data(class_filler, "ex:x a ex:A ; ex:p \"v\" ."));
        assert!(!accepts_data(class_filler, "ex:x a ex:A ; ex:p ex:node ."));
        let data_filler = "ex:p a owl:ObjectProperty .
             ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:p ; owl:someValuesFrom xsd:string ] .";
        assert!(accepts_data(data_filler, "ex:x a ex:A ; ex:p \"hello\" ."));
        // An IRI may denote a data value, so a node meets the data range too.
        assert!(accepts_data(data_filler, "ex:x a ex:A ; ex:p ex:node ."));
        assert!(!accepts_data(data_filler, "ex:x a ex:A ; ex:p 3 ."));
        assert!(!accepts_data(data_filler, "ex:x a ex:A ."));
        // Neighbour: a datatype property's exact data range rejects a node.
        let datatype_filler = "ex:d a owl:DatatypeProperty .
             ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:d ; owl:someValuesFrom xsd:string ] .";
        assert!(accepts_data(
            datatype_filler,
            "ex:x a ex:A ; ex:d \"hello\" ."
        ));
        assert!(!accepts_data(
            datatype_filler,
            "ex:x a ex:A ; ex:d ex:node ."
        ));
        // A cross-kind owl:hasValue is ∃p.{v}: the right value is accepted,
        // a wrong one rejected.
        let literal_value = "ex:q a owl:ObjectProperty .
             ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:q ; owl:hasValue \"v\" ] .";
        assert!(accepts_data(literal_value, "ex:x a ex:A ; ex:q \"v\" ."));
        assert!(!accepts_data(literal_value, "ex:x a ex:A ; ex:q \"w\" ."));
        assert!(!accepts_data(literal_value, "ex:x a ex:A ; ex:q ex:v ."));
        let individual_value = "ex:d a owl:DatatypeProperty .
             ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:d ; owl:hasValue ex:v ] .";
        assert!(accepts_data(individual_value, "ex:x a ex:A ; ex:d ex:v ."));
        assert!(!accepts_data(individual_value, "ex:x a ex:A ; ex:d ex:w ."));
        assert!(!accepts_data(
            individual_value,
            "ex:x a ex:A ; ex:d \"v\" ."
        ));
        // owl:hasSelf on a datatype property is the self restriction, read
        // (and reported unrepresented) as on an object property: the self
        // value is accepted.
        let self_restriction = "ex:p a owl:DatatypeProperty .
             ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:p ; owl:hasSelf true ] .";
        assert!(accepts_data(self_restriction, "ex:x a ex:A ; ex:p ex:x ."));
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
                [Restriction::SomeValues(OntologyExpression::Named(
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

    /// The outcomes the class-expression manifest reports for `expression` on
    /// `class`, or on the axiom itself when `class` is `None`.
    fn manifest_outcomes(
        surface: &SchemaSurface,
        class: Option<&str>,
        expression: &str,
    ) -> Vec<(SchemaExpressionOutcome, String)> {
        surface
            .class_expressions
            .axioms
            .iter()
            .flat_map(|axiom| {
                let rows: Vec<&SchemaExpressionComponent> = match class {
                    Some(class) => axiom
                        .classes
                        .iter()
                        .filter(|row| {
                            row.class_iri == format!("https://example.org/schema/{class}")
                        })
                        .flat_map(|row| &row.components)
                        .collect(),
                    None => axiom.components.iter().collect(),
                };
                rows
            })
            .filter(|component| component.expression == expression)
            .map(|component| (component.outcome, component.reason.clone()))
            .collect()
    }

    const EXS: &str = "https://example.org/schema/";

    #[test]
    fn restriction_provenance_is_scoped_to_the_classes_that_carry_it() {
        use std::fmt::Write as _;
        let mut ontology = String::from(
            "ex:p0 a owl:ObjectProperty . ex:p1 a owl:ObjectProperty .
             ex:Target a owl:Class .\n",
        );
        for class in 0..50 {
            let _ = writeln!(
                ontology,
                "ex:C{class} a owl:Class ; rdfs:subClassOf
                    [ a owl:Restriction ; owl:onProperty ex:p0 ; owl:someValuesFrom ex:Target ] ,
                    [ a owl:Restriction ; owl:onProperty ex:p1 ; owl:someValuesFrom ex:Target ] ."
            );
        }
        ontology.push_str("ex:Sub a owl:Class ; rdfs:subClassOf ex:C7 .\n");
        let surface = complete(&ontology).expect("restricted classes");
        let p0 = property(&surface, &format!("{EXS}p0"));
        for row in &p0.classes {
            let restriction_axioms: Vec<&SchemaCoverageProvenance> = row
                .provenance
                .iter()
                .filter(|provenance| provenance.predicate == rdfs::SUB_CLASS_OF)
                .collect();
            let expected: Vec<String> = match row.class_iri.strip_prefix(EXS) {
                Some("Sub") => vec![format!("{EXS}C7")],
                Some(local) if local.starts_with('C') => vec![row.class_iri.clone()],
                _ => Vec::new(),
            };
            assert_eq!(
                restriction_axioms
                    .iter()
                    .map(|provenance| provenance.subject.clone())
                    .collect::<Vec<_>>(),
                expected,
                "{}: a row carries only the restriction axioms its class inherits",
                row.class_iri
            );
        }
    }

    #[test]
    fn rdf_and_owl_datatype_map_members_are_datatypes() {
        complete(
            "ex:Text owl:equivalentClass rdf:PlainLiteral .
             ex:text a owl:DatatypeProperty ; rdfs:range ex:Text .
             ex:Dir owl:equivalentClass rdf:dirLangString .
             ex:dir a owl:DatatypeProperty ; rdfs:range ex:Dir .
             ex:A rdfs:subClassOf
                 [ a owl:Restriction ; owl:onProperty ex:dir ; owl:allValuesFrom rdf:dirLangString ] ,
                 [ a owl:Restriction ; owl:onProperty ex:real ; owl:allValuesFrom
                   [ a rdfs:Datatype ; owl:onDatatype owl:real ;
                     owl:withRestrictions ( [ xsd:minInclusive 0 ] ) ] ] ,
                 [ a owl:Restriction ; owl:onProperty ex:html ; owl:someValuesFrom rdf:HTML ] .
             ex:real a owl:DatatypeProperty .
             ex:html a owl:DatatypeProperty ; rdfs:range rdf:HTML .
             ex:json a owl:DatatypeProperty ; rdfs:range rdf:JSON .
             ex:xml a owl:DatatypeProperty ; rdfs:range rdf:XMLLiteral .
             ex:rational a owl:DatatypeProperty ; rdfs:range owl:rational .",
        )
        .expect("the OWL 2 datatype map and the RDF 1.2 datatypes are datatypes");
        // What was refused before stays refused; what was accepted stays accepted.
        complete(
            "ex:Text owl:equivalentClass rdf:PlainLiteral .
             ex:text a owl:ObjectProperty ; rdfs:range ex:Text .
             ex:json a owl:ObjectProperty ; rdfs:range rdf:JSON .",
        )
        .expect("an object property ranging over these names was always accepted");
        // A datatype range or filler on an object property is read by the OWL 2
        // Full Semantics (literal values), never refused.
        for read in [
            "ex:Text a rdfs:Datatype .
             ex:text a owl:ObjectProperty ; rdfs:range ex:Text .",
            "ex:q a owl:ObjectProperty .
             ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:q ;
                 owl:allValuesFrom rdf:dirLangString ] .",
            "ex:q a owl:ObjectProperty .
             ex:A rdfs:subClassOf [ a owl:Restriction ; owl:onProperty ex:q ;
                 owl:allValuesFrom [ a rdfs:Datatype ; owl:onDatatype owl:real ;
                 owl:withRestrictions ( [ xsd:minInclusive 0 ] ) ] ] .",
        ] {
            complete(read).unwrap_or_else(|error| panic!("{read} is read: {error:?}"));
        }
    }

    #[test]
    fn disjointness_and_class_assertions_with_anonymous_classes_are_reported() {
        let surface = complete(
            "ex:A a owl:Class ; owl:disjointWith
                 [ a owl:Restriction ; owl:onProperty ex:p ; owl:someValuesFrom ex:B ] .
             [ a owl:AllDisjointClasses ; owl:members ( ex:A ex:B
                 [ owl:complementOf ex:C ] ) ] .
             ex:D owl:disjointUnionOf ( ex:E [ a owl:Restriction ; owl:onProperty ex:p ;
                 owl:hasValue ex:v ] ) .
             ex:x a [ a owl:Restriction ; owl:onProperty ex:p ; owl:hasValue ex:v ] .
             ex:A owl:disjointWith ex:B .",
        )
        .expect("disjointness and class assertions are read");
        let predicates: BTreeSet<&str> = surface
            .class_expressions
            .axioms
            .iter()
            .map(|axiom| axiom.provenance.predicate.as_str())
            .collect();
        for predicate in [
            "http://www.w3.org/2002/07/owl#disjointWith",
            "http://www.w3.org/2002/07/owl#members",
            "http://www.w3.org/2002/07/owl#disjointUnionOf",
            rdf::TYPE,
        ] {
            assert!(predicates.contains(predicate), "{predicate} is reported");
        }
        for axiom in &surface.class_expressions.axioms {
            assert!(!axiom.components.is_empty(), "{:?}", axiom.provenance);
            for component in &axiom.components {
                assert!(matches!(
                    component.outcome,
                    SchemaExpressionOutcome::Unrepresented | SchemaExpressionOutcome::Excluded
                ));
            }
        }
        // The neighbour: disjointness between named classes is no anonymous
        // class expression, and stays out of the manifest.
        assert!(
            !surface
                .class_expressions
                .axioms
                .iter()
                .any(|axiom| axiom.provenance.object == format!("<{EXS}B>"))
        );
        // A malformed anonymous member is reported, not refused: the axiom was
        // always accepted (ignored), and nothing once accepted is refused.
        let malformed =
            complete("ex:A owl:disjointWith [ a owl:Restriction ; owl:someValuesFrom ex:B ] .")
                .expect("an axiom once ignored is reported, never refused");
        assert_eq!(
            manifest_outcomes(&malformed, None, ANONYMOUS_INDIVIDUAL)
                .iter()
                .map(|(outcome, _)| *outcome)
                .collect::<Vec<_>>(),
            vec![SchemaExpressionOutcome::Unrepresented]
        );
        // The neighbour: the same malformed restriction as the object of
        // rdfs:subClassOf, which was always refused, is still refused.
        let error =
            complete("ex:A rdfs:subClassOf [ a owl:Restriction ; owl:someValuesFrom ex:B ] .")
                .expect_err("this was always refused");
        assert!(matches!(error, SchemaCompileError::InvalidOntology { .. }));
    }

    #[test]
    fn class_fillers_and_datatype_complements_are_approximations() {
        let surface = complete(
            "ex:A a owl:Class ; rdfs:subClassOf
                 [ a owl:Restriction ; owl:onProperty ex:p ; owl:allValuesFrom ex:B ] ,
                 [ a owl:Restriction ; owl:onProperty ex:n ; owl:allValuesFrom
                   [ owl:intersectionOf ( xsd:integer
                       [ a rdfs:Datatype ; owl:datatypeComplementOf xsd:negativeInteger ] ) ] ] ,
                 [ a owl:Restriction ; owl:onProperty ex:s ; owl:allValuesFrom xsd:string ] .
             ex:p a owl:ObjectProperty . ex:B a owl:Class .
             ex:n a owl:DatatypeProperty . ex:s a owl:DatatypeProperty .",
        )
        .expect("universals");
        let outcome = |expression: String| manifest_outcomes(&surface, Some("A"), &expression)[0].0;
        assert_eq!(
            outcome(format!("all(<{EXS}p>,<{EXS}B>)")),
            SchemaExpressionOutcome::Approximated,
            "class membership of a referenced node is not checked"
        );
        assert_eq!(
            outcome(format!(
                "all(<{EXS}n>,intersection(<http://www.w3.org/2001/XMLSchema#integer>,datatype_complement(<http://www.w3.org/2001/XMLSchema#negativeInteger>)))"
            )),
            SchemaExpressionOutcome::Approximated,
            "a datatype complement is judged on the @type tag, not the value space"
        );
        assert_eq!(
            outcome(format!(
                "all(<{EXS}s>,<http://www.w3.org/2001/XMLSchema#string>)"
            )),
            SchemaExpressionOutcome::Projected,
            "a datatype filler is exact"
        );
    }

    #[test]
    fn existential_restrictions_place_a_class_in_the_property_domain() {
        let surface = complete(
            "ex:D a owl:Class . ex:B a owl:Class .
             ex:p a owl:ObjectProperty ; rdfs:domain ex:D .
             ex:q a owl:ObjectProperty ; rdfs:domain ex:D .
             ex:A a owl:Class ; rdfs:subClassOf
                 [ a owl:Restriction ; owl:onProperty ex:p ; owl:someValuesFrom ex:B ] .
             ex:U a owl:Class ; rdfs:subClassOf
                 [ a owl:Restriction ; owl:onProperty ex:p ; owl:allValuesFrom ex:B ] .",
        )
        .expect("existential domain");
        assert_eq!(
            class_status(&surface, &format!("{EXS}q"), &format!("{EXS}A")),
            SchemaCoverageStatus::IncludedUnshaped,
            "A ⊑ ∃p.B and domain(p) = D entail A ⊑ D"
        );
        assert_eq!(
            class_status(&surface, &format!("{EXS}q"), &format!("{EXS}U")),
            SchemaCoverageStatus::ExcludedDomain,
            "a universal restriction entails no domain membership"
        );
    }

    #[test]
    fn a_cross_kind_has_value_places_its_carriers_in_the_property_domain() {
        // A literal owl:hasValue on an object property is ∃p.{v} under the
        // OWL 2 Full Semantics, so the axiom's existential places each member
        // of the union in p's domain, as the same axiom over an individual
        // does.
        let ontology = |filler: &str| {
            format!(
                "ex:A a owl:Class . ex:B a owl:Class . ex:C a owl:Class . ex:D a owl:Class .
                 ex:p a owl:ObjectProperty ; rdfs:domain ex:D .
                 ex:q a owl:ObjectProperty ; rdfs:domain ex:D .
                 [ owl:unionOf ( ex:A ex:B ) ] rdfs:subClassOf
                     [ a owl:Restriction ; owl:onProperty ex:p ; owl:hasValue {filler} ] ."
            )
        };
        for filler in ["\"literal\"", "ex:c"] {
            let surface = complete(&ontology(filler)).expect("a well-formed axiom");
            for class in ["A", "B"] {
                assert_eq!(
                    class_status(&surface, &format!("{EXS}q"), &format!("{EXS}{class}")),
                    SchemaCoverageStatus::IncludedUnshaped,
                    "{class} ⊑ ∃p.{{{filler}}} and domain(p) = D entail {class} ⊑ D"
                );
            }
        }
        // Neighbour: a class outside the union stays out of D.
        let surface = complete(&ontology("\"literal\"")).expect("a well-formed axiom");
        assert_eq!(
            class_status(&surface, &format!("{EXS}q"), &format!("{EXS}C")),
            SchemaCoverageStatus::ExcludedDomain
        );
    }

    #[test]
    fn universal_restrictions_on_owl_thing_are_global_ranges() {
        let surface = complete(
            "ex:A a owl:Class . ex:C a owl:Class .
             ex:p a owl:ObjectProperty .
             owl:Thing rdfs:subClassOf
                 [ a owl:Restriction ; owl:onProperty ex:p ; owl:allValuesFrom ex:A ] .",
        )
        .expect("global range");
        let p = &surface.classes[&format!("{EXS}C")].properties[&format!("{EXS}p")];
        assert_eq!(p.ranges, vec![OntologyExpression::Named(format!("{EXS}A"))]);
        let reported = manifest_outcomes(&surface, None, &format!("all(<{EXS}p>,<{EXS}A>)"));
        assert_eq!(reported.len(), 1, "{reported:?}");
        assert!(reported[0].1.contains("range"), "{reported:?}");
    }

    #[test]
    fn restrictions_on_named_inverses_resolve_to_the_named_property() {
        let surface = complete(
            "ex:made a owl:ObjectProperty ; owl:inverseOf ex:maker .
             ex:maker a owl:ObjectProperty .
             ex:Thingy a owl:Class . ex:Person a owl:Class .
             ex:Z a owl:Class ; rdfs:subClassOf
                 [ a owl:Restriction ; owl:onProperty [ owl:inverseOf ex:made ] ;
                   owl:someValuesFrom ex:Person ] ,
                 [ a owl:Restriction ; owl:onProperty [ owl:inverseOf ex:unpaired ] ;
                   owl:someValuesFrom ex:Person ] .
             ex:unpaired a owl:ObjectProperty .",
        )
        .expect("inverse restrictions");
        let resolved = manifest_outcomes(
            &surface,
            Some("Z"),
            &format!("some(<{EXS}maker>,<{EXS}Person>)"),
        );
        assert_eq!(resolved.len(), 1, "inverse(made) is maker");
        assert_eq!(resolved[0].0, SchemaExpressionOutcome::Approximated);
        let unresolved = manifest_outcomes(
            &surface,
            Some("Z"),
            &format!("some(inverse(<{EXS}unpaired>),<{EXS}Person>)"),
        );
        assert_eq!(unresolved[0].0, SchemaExpressionOutcome::Unrepresented);
    }

    #[test]
    fn any_restriction_emits_its_property_on_the_carrying_class() {
        let surface = complete(
            "ex:Elsewhere a owl:Class . ex:Z a owl:Class ; rdfs:subClassOf
                 [ a owl:Restriction ; owl:onProperty ex:q ; owl:allValuesFrom xsd:integer ] ,
                 [ a owl:Restriction ; owl:onProperty ex:q ; owl:maxCardinality 1 ] .
             ex:Y a owl:Class .
             ex:q a owl:DatatypeProperty ; rdfs:domain ex:Elsewhere .",
        )
        .expect("restricted property");
        assert_eq!(
            class_status(&surface, &format!("{EXS}q"), &format!("{EXS}Z")),
            SchemaCoverageStatus::IncludedUnshaped
        );
        assert_eq!(
            class_status(&surface, &format!("{EXS}q"), &format!("{EXS}Y")),
            SchemaCoverageStatus::ExcludedDomain,
            "a class that carries no restriction stays out of the domain"
        );
    }

    #[test]
    fn axioms_once_ignored_are_reported_not_refused_when_malformed() {
        let surface = complete(
            "[ a owl:Restriction ; owl:someValuesFrom ex:B ] rdfs:subClassOf ex:A .
             [ rdfs:label \"no inverse\" ] rdfs:subPropertyOf ex:p .
             ex:x a [ a owl:Restriction ; owl:onProperty ex:p ] .
             ex:R a owl:Restriction ; owl:someValuesFrom ex:B .",
        )
        .expect("every one of these axioms was always accepted");
        let malformed = surface
            .class_expressions
            .axioms
            .iter()
            .flat_map(|axiom| &axiom.components)
            .filter(|component| component.outcome == SchemaExpressionOutcome::Unrepresented)
            .count();
        assert_eq!(malformed, 4, "{:#?}", surface.class_expressions.axioms);
    }

    #[test]
    fn named_subject_class_constructors_are_equivalences() {
        let surface = complete(
            "ex:Day a owl:Class ; owl:oneOf ( ex:mon ex:tue ) .
             ex:B a owl:Class . ex:C a owl:Class .
             ex:BC a owl:Class ; owl:unionOf ( ex:B ex:C ) .
             ex:NotB a owl:Class ; owl:complementOf ex:B .
             ex:R a owl:Restriction ; owl:onProperty ex:p ; owl:someValuesFrom ex:B .
             ex:A a owl:Class ; rdfs:subClassOf ex:R .
             ex:p a owl:ObjectProperty .
             ex:q a owl:ObjectProperty ; rdfs:domain ex:BC .
             [ a owl:Restriction ; owl:onProperty ex:p ; owl:someValuesFrom ex:B ]
                 owl:hasKey ( ex:p ) .",
        )
        .expect("named-subject constructors");
        let predicates: BTreeSet<&str> = surface
            .class_expressions
            .axioms
            .iter()
            .map(|axiom| axiom.provenance.predicate.as_str())
            .collect();
        for predicate in [
            "http://www.w3.org/2002/07/owl#oneOf",
            "http://www.w3.org/2002/07/owl#unionOf",
            "http://www.w3.org/2002/07/owl#complementOf",
            "http://www.w3.org/2002/07/owl#onProperty",
            "http://www.w3.org/2002/07/owl#hasKey",
        ] {
            assert!(
                predicates.contains(predicate),
                "{predicate} reported: {predicates:?}"
            );
        }
        assert_eq!(
            surface.classes[&format!("{EXS}Day")].focus.len(),
            1,
            "Day ≡ {{mon, tue}} enumerates its @id"
        );
        assert_eq!(
            surface.classes[&format!("{EXS}A")].properties[&format!("{EXS}p")]
                .restrictions
                .len(),
            1,
            "A ⊑ R carries R's restriction"
        );
        assert_eq!(
            class_status(&surface, &format!("{EXS}q"), &format!("{EXS}B")),
            SchemaCoverageStatus::IncludedUnshaped,
            "BC ≡ B ⊔ C makes B a subclass of BC"
        );
        // The neighbour: a named class with no constructor is no axiom.
        let plain = complete("ex:Day a owl:Class .").expect("plain class");
        assert_eq!(plain.class_expressions.axioms.len(), 0);
    }

    #[test]
    fn approximate_named_datatype_ranges_are_reported_as_approximations() {
        let surface = complete(
            "ex:C a owl:Class .
             ex:real a owl:DatatypeProperty ; rdfs:range owl:real .
             ex:rational a owl:DatatypeProperty ; rdfs:range owl:rational .
             ex:json a owl:DatatypeProperty ; rdfs:range rdf:JSON .
             ex:xml a owl:DatatypeProperty ; rdfs:range rdf:XMLLiteral .
             ex:html a owl:DatatypeProperty ; rdfs:range rdf:HTML .
             ex:decimal a owl:DatatypeProperty ; rdfs:range xsd:decimal .
             ex:string a owl:DatatypeProperty ; rdfs:range xsd:string .",
        )
        .expect("datatype ranges");
        let precision = |local: &str| {
            property(&surface, &format!("{EXS}{local}"))
                .classes
                .iter()
                .find(|row| row.class_iri == format!("{EXS}C"))
                .expect("C row")
                .precision
        };
        for local in ["real", "rational", "json", "xml", "html"] {
            assert_eq!(
                precision(local),
                SchemaCoveragePrecision::RepresentationApproximation,
                "{local}: the lexical form is not judged"
            );
        }
        // Read by value, a decimal range admits every literal typed
        // owl:rational, whose value no pattern judges.
        assert_eq!(
            precision("decimal"),
            SchemaCoveragePrecision::RepresentationApproximation
        );
        assert_eq!(precision("string"), SchemaCoveragePrecision::Exact);
    }

    #[test]
    fn dense_iri_only_ranges_within_the_cell_ceiling_are_not_refused() {
        use std::fmt::Write as _;
        // 25,000 classes × 9 domainless properties × 5 ranges is over a
        // million range expressions in 225,045 coverage cells, under the
        // one-million-cell ceiling.
        let mut ontology = String::new();
        for range in 0..5 {
            let _ = writeln!(ontology, "ex:R{range} a owl:Class .");
        }
        for property in 0..9 {
            let _ = write!(ontology, "ex:p{property} a owl:ObjectProperty ; rdfs:range");
            for range in 0..5 {
                let _ = write!(
                    ontology,
                    "{} ex:R{range}",
                    if range == 0 { "" } else { " ," }
                );
            }
            ontology.push_str(" .\n");
        }
        for class in 0..25_000 {
            let _ = writeln!(ontology, "ex:C{class} a owl:Class .");
        }
        let shape_dataset =
            crate::text_ingest::parse_turtle_to_dataset(PREFIXES, None).expect("shape Turtle");
        let shapes = from_dataset(&shape_dataset).expect("shape graph");
        let dataset =
            crate::text_ingest::parse_turtle_to_dataset(&format!("{PREFIXES}{ontology}"), None)
                .expect("ontology Turtle");
        let report = SchemaCompileRequest::new(
            &shapes,
            &namespaces(),
            dataset.as_ref(),
            SchemaSurfaceMode::OntologyComplete,
        )
        .coverage_report()
        .expect("an IRI-only ontology within the cell ceiling compiles");
        assert_eq!(report.properties.len(), 9);
    }
}

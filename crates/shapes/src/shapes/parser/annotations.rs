// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Per-constraint RDF 1.2 reifier annotations: `sh:deactivated`, `sh:severity`
//! and `sh:message` on a reifier of a `(shape, parameter, value)` statement.
//!
//! SHACL 1.2 Core states all three:
//!
//! * "Deactivating Shapes and Constraints": "A triple that has a shape as
//!   subject, a parameter (such as sh:minCount) as predicate can have at most one
//!   reifier with a value for the property sh:deactivated. Let expr be the value
//!   of sh:deactivated in a reifier on a triple that has shape subject and a
//!   parameter as predicate. If evalExpr(expr, data graph, focus node, {})
//!   produces true as its only output node, the constraints that use the triple
//!   are called deactivated constraints. Deactivated constraints are ignored
//!   during validation." — and "In SHACL Core, the only valid values for
//!   sh:deactivated are the constant literal node expressions true and false."
//! * "Declaring the Severity of a Shape or Constraint": "the property sh:severity
//!   can also be used on a reifier for a triple where the shape is the subject and
//!   one of the parameters of the constraint is the predicate. Let T be the set of
//!   triples that represent a constraint in a shape. A shapes graph can specify at
//!   most one value for the property sh:severity in the reifiers of the triples
//!   in T."
//! * "Declaring Messages for a Shape or Constraint": "the property sh:message can
//!   also be used on a reifier for a triple where the shape is the subject and one
//!   of the parameters of the constraint is the predicate. Let T be the set of
//!   triples that represent a constraint in a shape. A shapes graph can specify at
//!   most one value for the property sh:message in the reifiers of the triples in
//!   T." The same section's own example puts TWO language-tagged messages in one
//!   annotation (`sh:maxLength 10 {| sh:message "Too many characters"@en ;
//!   sh:message "Zu viele Zeichen"@de |}`), so "one value" is read as one message
//!   SET: a reifier's messages are taken together, and two reifiers that state
//!   different sets are the conflict the sentence forbids.
//!
//! # What this module decides
//!
//! * Annotations are read from the SHAPES graph only, and only on reifiers of
//!   ASSERTED statements. The constraints are read from asserted triples, so an
//!   annotation on an unasserted triple term annotates no constraint: it creates
//!   none and deactivates none.
//! * Two reifiers of one statement that disagree — `true` against `false`, two
//!   severities, two message sets — are a load error, and so are two statements of
//!   one constraint (T) that disagree on a severity or a message set. Reifiers
//!   that AGREE load: a duplicate that says the same thing contradicts nothing.
//! * An annotation is honoured or refused, never dropped. The parser records every
//!   annotated statement it applied, and a parse that leaves one unapplied is a
//!   load error (see [`Parser::check_annotations_applied`]) — so a parameter the
//!   parser reads through a route that forgot to consult its annotations fails
//!   loudly instead of validating as if the annotation were absent.

use ::purrdf_rdf::{FastMap, TermId, TermRef};
use purrdf_core::SmallVec;

use crate::data::{GraphFilter, native_quads};
use crate::expression::NodeExpr;
use crate::model::sh;
use crate::report::Severity;
use crate::shapes::Parser;
use crate::term::{Literal, NamedNode, Term};

/// The three per-constraint annotation predicates.
pub(crate) const CONSTRAINT_ANNOTATIONS: [&str; 3] = [sh::DEACTIVATED, sh::SEVERITY, sh::MESSAGE];

/// The reifier side of the shapes graph, indexed once per parse.
///
/// Both maps are empty — and were built without allocating — for a shapes graph
/// that reifies nothing, which is nearly every shapes graph; every lookup below
/// returns at once in that case.
#[derive(Debug, Default)]
pub(crate) struct AnnotationIndex {
    /// Each reified statement's triple term → its reifiers, in id order.
    reifiers: FastMap<TermId, SmallVec<[TermId; 1]>>,
    /// Each reifier → its annotation rows `(predicate, object)`.
    rows: FastMap<TermId, SmallVec<[(TermId, TermId); 2]>>,
}

impl AnnotationIndex {
    /// Index the reifier and annotation side tables of `data`.
    pub(crate) fn build(data: &::purrdf_rdf::RdfDataset) -> Self {
        let mut index = Self::default();
        for quad in data.reifier_quads() {
            let reifiers = index.reifiers.entry(quad.o).or_default();
            if !reifiers.contains(&quad.s) {
                reifiers.push(quad.s);
            }
        }
        if index.reifiers.is_empty() {
            return index;
        }
        for quad in data.annotation_quads() {
            let rows = index.rows.entry(quad.s).or_default();
            if !rows.contains(&(quad.p, quad.o)) {
                rows.push((quad.p, quad.o));
            }
        }
        index
    }

    /// Whether the shapes graph reifies nothing.
    #[inline]
    pub(crate) fn is_empty(&self) -> bool {
        self.reifiers.is_empty()
    }
}

/// What the reifiers of one asserted shape statement say.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct StatementAnnotation {
    /// Whether a reifier states `sh:deactivated true`.
    pub(crate) deactivated: bool,
    /// The structured node expressions the reifiers state as `sh:deactivated`, each
    /// the blank node that is the expression, in the order the reifiers hold them.
    /// They are evaluated at each focus node, so no verdict is available here.
    pub(crate) expressions: Vec<Term>,
    /// The one `sh:severity` the reifiers state.
    pub(crate) severity: Option<Severity>,
    /// The one `sh:message` set the reifiers state, as literal terms in canonical
    /// order; empty when they state none.
    pub(crate) messages: Vec<Term>,
    /// Whether any reifier carries any of the three annotations — `false` and
    /// `sh:deactivated false` included.
    pub(crate) present: bool,
}

/// What the annotations of one constraint's statements (its T) resolve to.
#[derive(Debug, Clone)]
pub(crate) enum Annotated {
    /// No reifier annotates the constraint.
    Plain,
    /// A reifier deactivates the constraint: it is not emitted.
    Deactivated,
    /// A reifier overrides the constraint's severity and/or message, or states a
    /// `sh:deactivated` node expression that is evaluated at each focus node.
    Override {
        /// The override severity.
        severity: Option<Severity>,
        /// The override messages — the whole set, canonical order; empty for none.
        messages: Vec<Literal>,
        /// The `sh:deactivated` node expressions: the constraint is deactivated at a
        /// focus node where any of them produces `true`.
        deactivated_when: Vec<NodeExpr>,
    },
}

impl Annotated {
    /// Whether `self` and `other` annotate their constraints identically. Two
    /// `sh:deactivated` node expressions are never compared, so an annotation that
    /// states one is the same as no other.
    pub(crate) fn same_as(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Plain, Self::Plain) | (Self::Deactivated, Self::Deactivated) => true,
            (
                Self::Override {
                    severity,
                    messages,
                    deactivated_when,
                },
                Self::Override {
                    severity: other_severity,
                    messages: other_messages,
                    deactivated_when: other_deactivated_when,
                },
            ) => {
                severity == other_severity
                    && messages == other_messages
                    && deactivated_when.is_empty()
                    && other_deactivated_when.is_empty()
            }
            _ => false,
        }
    }

    /// This annotation with `expressions` added to the constraint's
    /// `sh:deactivated` node expressions. A constraint that is already deactivated
    /// stays so, and one with no annotation becomes an override that changes nothing
    /// but the deactivation.
    pub(crate) fn with_deactivation(self, expressions: Vec<NodeExpr>) -> Self {
        if expressions.is_empty() {
            return self;
        }
        match self {
            Self::Deactivated => Self::Deactivated,
            Self::Plain => Self::Override {
                severity: None,
                messages: Vec::new(),
                deactivated_when: expressions,
            },
            Self::Override {
                severity,
                messages,
                mut deactivated_when,
            } => {
                deactivated_when.extend(expressions);
                Self::Override {
                    severity,
                    messages,
                    deactivated_when,
                }
            }
        }
    }
}

impl Parser<'_> {
    /// The annotation rows `(predicate, object)` on every reifier of the ASSERTED
    /// statement `(shape, predicate, object)`, reifier by reifier. Empty when the
    /// statement is not asserted, has no reifier, or its reifiers carry nothing.
    pub(crate) fn reifier_rows(
        &self,
        shape: &Term,
        predicate: &str,
        object: &Term,
    ) -> Vec<Vec<(NamedNode, Term)>> {
        if self.annotation_index.is_empty() {
            return Vec::new();
        }
        let (Some(s), Some(p), Some(o)) = (
            crate::data::resolve_id(self.data, shape),
            self.data.term_id_by_iri(predicate),
            crate::data::resolve_id(self.data, object),
        ) else {
            return Vec::new();
        };
        let Some(statement) = self.data.term_id_by_triple(s, p, o) else {
            return Vec::new();
        };
        let Some(reifiers) = self.annotation_index.reifiers.get(&statement) else {
            return Vec::new();
        };
        // Only an ASSERTED statement is a constraint; a reifier of the same triple
        // term that the shapes graph never asserts annotates nothing.
        let asserted = !native_quads(
            self.data,
            Some(shape),
            Some(&Term::NamedNode(NamedNode::from(predicate))),
            Some(object),
            GraphFilter::AnyGraph,
        )
        .is_empty();
        if !asserted {
            return Vec::new();
        }
        reifiers
            .iter()
            .map(|reifier| {
                self.annotation_index
                    .rows
                    .get(reifier)
                    .map_or_default(|rows| {
                        rows.iter()
                            .filter_map(|&(p, o)| match self.data.resolve(p) {
                                TermRef::Iri(iri) => Some((
                                    NamedNode::from(iri),
                                    crate::term::term_id_to_native(self.data, o),
                                )),
                                // The IR admits only IRI predicates; a row that is
                                // not one cannot be read as an annotation, and the
                                // well-formedness pass reports it.
                                _ => None,
                            })
                            .collect()
                    })
            })
            .collect()
    }

    /// What the reifiers of the asserted statement `(shape, predicate, object)`
    /// say, with every value checked and every disagreement refused.
    ///
    /// # Errors
    ///
    /// A value of the wrong kind (`sh:deactivated` not an `xsd:boolean`,
    /// `sh:severity` not an IRI, `sh:message` not a string literal), or two
    /// reifiers — or two values on one — that disagree.
    pub(crate) fn statement_annotation(
        &self,
        shape: &Term,
        predicate: &str,
        object: &Term,
    ) -> Result<StatementAnnotation, String> {
        let mut out = StatementAnnotation::default();
        let mut deactivated: Option<bool> = None;
        let mut message_set: Option<Vec<Term>> = None;
        // How many reifiers state a `sh:deactivated` value, and whether any such value
        // is a node expression: Core allows at most one reifier with a value for
        // `sh:deactivated`, and the booleans are held to agreement instead (below).
        let mut valued_reifiers = 0usize;
        let mut expression_valued = false;
        let where_ = || format!("the reifiers of shape {shape}'s <{predicate}> {object} statement");
        for rows in self.reifier_rows(shape, predicate, object) {
            let mut messages: Vec<Term> = Vec::new();
            let mut states_deactivated = false;
            for (annotation, value) in rows {
                match annotation.as_str() {
                    sh::DEACTIVATED
                        if matches!(value, Term::BlankNode(_))
                            && !self.is_bare_blank_node(&value) =>
                    {
                        // A structured node expression. SHACL 1.2 Core: "Let expr be
                        // the value of sh:deactivated in a reifier on a triple that
                        // has shape subject and a parameter as predicate. If
                        // evalExpr(expr, data graph, focus node, {}) produces true as
                        // its only output node, the constraints that use the triple
                        // are called deactivated constraints." The 2017 Recommendation
                        // allows `true` and `false` alone.
                        if self.profile == crate::profile::ShaclProfile::REC_20170720 {
                            return Err(format!(
                                "sh:deactivated on {} is a node expression, but SHACL REC \
                                 2017-07-20 allows only the xsd:boolean literals true and false",
                                where_()
                            ));
                        }
                        out.present = true;
                        states_deactivated = true;
                        expression_valued = true;
                        if !out.expressions.contains(&value) {
                            out.expressions.push(value);
                        }
                    }
                    sh::DEACTIVATED => {
                        out.present = true;
                        states_deactivated = true;
                        let flag = super::node_expr::boolean_value(&value).ok_or_else(|| {
                            format!(
                                "sh:deactivated on {} must be an xsd:boolean literal or a node \
                                 expression that has properties, got {value}",
                                where_()
                            )
                        })?;
                        match deactivated {
                            Some(seen) if seen != flag => {
                                return Err(format!(
                                    "{} state both sh:deactivated true and false; a statement \
                                     has at most one sh:deactivated value",
                                    where_()
                                ));
                            }
                            _ => deactivated = Some(flag),
                        }
                    }
                    sh::SEVERITY => {
                        out.present = true;
                        let Term::NamedNode(level) = &value else {
                            return Err(format!(
                                "sh:severity on {} must be an IRI, got {value}",
                                where_()
                            ));
                        };
                        let level = Severity::from_iri_open(level.as_str());
                        match &out.severity {
                            Some(seen) if *seen != level => {
                                return Err(format!(
                                    "{} state two severities, <{}> and <{}>; a shapes graph can \
                                     specify at most one value for sh:severity in the reifiers of \
                                     a constraint's triples",
                                    where_(),
                                    seen.iri(),
                                    level.iri()
                                ));
                            }
                            _ => out.severity = Some(level),
                        }
                    }
                    sh::MESSAGE => {
                        out.present = true;
                        if !super::wellformed::is_text_literal(&value, true) {
                            return Err(format!(
                                "sh:message on {} must be an xsd:string, rdf:langString, \
                                 rdf:dirLangString or rdf:HTML literal, got {value}",
                                where_()
                            ));
                        }
                        messages.push(value);
                    }
                    _ => {}
                }
            }
            valued_reifiers += usize::from(states_deactivated);
            if !messages.is_empty() {
                crate::term::sort_terms_canonical(&mut messages);
                messages.dedup();
                match &message_set {
                    Some(seen) if *seen != messages => {
                        return Err(format!(
                            "{} state two different sh:message sets; a shapes graph can specify \
                             at most one value for sh:message in the reifiers of a constraint's \
                             triples",
                            where_()
                        ));
                    }
                    _ => message_set = Some(messages),
                }
            }
        }
        if valued_reifiers > 1 && expression_valued {
            return Err(format!(
                "{} state sh:deactivated on {valued_reifiers} reifiers and at least one value is a \
                 node expression; SHACL 1.2 Core: a triple \"can have at most one reifier with a \
                 value for the property sh:deactivated\"",
                where_()
            ));
        }
        out.deactivated = deactivated == Some(true);
        out.messages = message_set.unwrap_or_default();
        Ok(out)
    }

    /// What the annotations of one constraint resolve to, given the statements
    /// `triples` (its T) that represent it on `shape`. Records each annotated
    /// statement as APPLIED.
    ///
    /// # Errors
    ///
    /// As [`Self::statement_annotation`], and when two statements of T state
    /// different severities or different message sets.
    pub(crate) fn constraint_annotation(
        &mut self,
        shape: &Term,
        triples: &[(&str, &Term)],
    ) -> Result<Annotated, String> {
        if self.annotation_index.is_empty() {
            return Ok(Annotated::Plain);
        }
        let mut deactivated = false;
        let mut severity: Option<Severity> = None;
        let mut messages: Vec<Term> = Vec::new();
        let mut expressions: Vec<Term> = Vec::new();
        for &(predicate, object) in triples {
            let annotation = self.statement_annotation(shape, predicate, object)?;
            if !annotation.present {
                continue;
            }
            self.annotations_applied
                .insert((shape.clone(), predicate.to_owned(), object.clone()));
            deactivated |= annotation.deactivated;
            for expression in annotation.expressions {
                if !expressions.contains(&expression) {
                    expressions.push(expression);
                }
            }
            if let Some(level) = annotation.severity {
                match &severity {
                    Some(seen) if *seen != level => {
                        return Err(format!(
                            "shape {shape} annotates one constraint with two severities, <{}> and \
                             <{}>, on reifiers of different statements of it; a shapes graph can \
                             specify at most one value for sh:severity in the reifiers of the \
                             triples in T",
                            seen.iri(),
                            level.iri()
                        ));
                    }
                    _ => severity = Some(level),
                }
            }
            if !annotation.messages.is_empty() {
                if !messages.is_empty() && messages != annotation.messages {
                    return Err(format!(
                        "shape {shape} annotates one constraint with two different sh:message \
                         sets, on reifiers of different statements of it; a shapes graph can \
                         specify at most one value for sh:message in the reifiers of the triples \
                         in T"
                    ));
                }
                messages = annotation.messages;
            }
        }
        if deactivated {
            return Ok(Annotated::Deactivated);
        }
        let mut deactivated_when: Vec<NodeExpr> = Vec::with_capacity(expressions.len());
        for expression in &expressions {
            deactivated_when.push(self.parse_deactivation_expression(shape, expression)?);
        }
        if severity.is_none() && messages.is_empty() && deactivated_when.is_empty() {
            return Ok(Annotated::Plain);
        }
        // Every message is checked to be a literal by `statement_annotation`.
        let messages = crate::report::canonical_messages(
            messages
                .into_iter()
                .filter_map(|term| match term {
                    Term::Literal(lit) => Some(lit),
                    _ => None,
                })
                .collect(),
        );
        Ok(Annotated::Override {
            severity,
            messages,
            deactivated_when,
        })
    }

    /// Record the annotated statement `(shape, predicate, object)` as applied
    /// where the parameter it annotates yields no constraint at all — a
    /// `sh:closed false`, a dangling `sh:qualifiedMinCount`, a custom component
    /// SHACL-SPARQL tells the engine to ignore — after checking what it says.
    ///
    /// # Errors
    ///
    /// As [`Self::statement_annotation`].
    pub(crate) fn apply_to_nothing(
        &mut self,
        shape: &Term,
        predicate: &str,
        object: &Term,
    ) -> Result<(), String> {
        if self.annotation_index.is_empty() {
            return Ok(());
        }
        if self.statement_annotation(shape, predicate, object)?.present {
            self.annotations_applied
                .insert((shape.clone(), predicate.to_owned(), object.clone()));
        }
        Ok(())
    }

    /// Refuse a shape parse that left an annotated statement of `shape` unapplied.
    ///
    /// Every route that turns a parameter statement into a constraint consults
    /// [`Self::constraint_annotation`]; this is the backstop that turns a route
    /// which did not into a load error instead of a silently unannotated
    /// constraint.
    ///
    /// # Errors
    ///
    /// Names the first unapplied annotated statement.
    pub(crate) fn check_annotations_applied(&self, shape: &Term) -> Result<(), String> {
        if self.annotation_index.is_empty() {
            return Ok(());
        }
        let mut statements: Vec<(NamedNode, Term)> =
            native_quads(self.data, Some(shape), None, None, GraphFilter::AnyGraph)
                .into_iter()
                .map(|(_, predicate, object)| (predicate, object))
                .collect();
        statements.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));
        for (predicate, object) in statements {
            let annotated = self
                .reifier_rows(shape, predicate.as_str(), &object)
                .iter()
                .flatten()
                .any(|(annotation, _)| CONSTRAINT_ANNOTATIONS.contains(&annotation.as_str()));
            if annotated
                && !self.annotations_applied.contains(&(
                    shape.clone(),
                    predicate.as_str().to_owned(),
                    object.clone(),
                ))
            {
                return Err(format!(
                    "shape {shape} annotates its <{}> {object} statement on a reifier, but no \
                     constraint of the shape is represented by that statement; the annotation is \
                     refused rather than ignored",
                    predicate.as_str()
                ));
            }
        }
        Ok(())
    }
}

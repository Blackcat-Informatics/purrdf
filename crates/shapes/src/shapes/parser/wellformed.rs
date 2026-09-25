// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Shapes-graph well-formedness: every shape node, checked against the census.
//!
//! The parser reads the constraint parameters it knows off the shapes it reaches.
//! Everything else a shape node carries used to be walked past — an unknown
//! `sh:` predicate, a term this engine does not evaluate, a literal where SHACL
//! requires an IRI — and the shapes graph loaded green while checking less than
//! it said. This pass closes that, once, before any shape is parsed:
//!
//! * The nodes checked are the SHAPES of the shapes graph as SHACL 1.2 Core §2.1
//!   defines them — "s is a SHACL instance of sh:NodeShape or sh:PropertyShape;
//!   s is subject of a triple that has sh:targetClass, sh:targetNode,
//!   sh:targetObjectsOf or sh:targetSubjectsOf as predicate; s is subject of a
//!   triple that has a parameter as predicate; s is a value of a shape-expecting,
//!   non-list-taking parameter such as sh:node, or a member of a SHACL list that
//!   is a value of a shape-expecting and list-taking parameter such as sh:or" —
//!   whether or not a target reaches them — and the parameter declarations (the
//!   objects of `sh:parameter`), which SHACL makes `sh:Parameter`s and so property
//!   shapes. Nothing else: a data node, an ontology header or a rule is never
//!   judged here.
//! * On each, every `sh:` / `shnex:` predicate is looked up in the census
//!   ([`crate::spec::census`]): an unknown term, an unimplemented term, and a
//!   term that does not belong on a shape are load errors naming the term and
//!   the node; a constraint parameter's value must meet its value rule, and a
//!   parameter SHACL forbids on node shapes must not appear on one.
//!
//! The pass also enforces the one graph-level MUST SHACL places on a processor
//! that supports no entailment regime: "If a shapes graph contains any triple
//! with the predicate sh:entailment and the object E and the SHACL processor
//! does not support E as an entailment regime for the given data graph then the
//! processor MUST signal a failure."

use ::purrdf::FastSet;

use crate::data::{GraphFilter, native_quads};
use crate::model::{rdf, rdfs, sh, xsd};
use crate::shapes::Parser;
use crate::spec::ValueRule;
use crate::spec::census::{self, Role, TermClass};
use crate::term::{NamedNode, Term};

/// `rdf:langString`.
const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";
/// `rdf:dirLangString`.
const RDF_DIR_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString";
/// `rdf:HTML`.
const RDF_HTML: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#HTML";
/// `sh:entailment`.
const ENTAILMENT: &str = "http://www.w3.org/ns/shacl#entailment";
/// `sh:ShapeClass`.
const SHAPE_CLASS: &str = "http://www.w3.org/ns/shacl#ShapeClass";

/// The terms a SPARQL-based constraint or target node carries (SHACL 1.2 SPARQL
/// Extensions §3 and §5): its query and the prefixes, message, severity and
/// deactivation it may declare.
const SPARQL_EXECUTABLE_TERMS: [&str; 5] = [
    sh::SELECT,
    sh::PREFIXES,
    sh::MESSAGE,
    sh::SEVERITY,
    sh::DEACTIVATED,
];

impl Parser<'_> {
    /// Check every shape of the shapes graph against the census. See the
    /// [module docs](self).
    ///
    /// Runs after the constraint-component registry is parsed, so a parameter a
    /// declared custom component supplies an implementation for is recognised
    /// as that component's parameter.
    ///
    /// # Errors
    ///
    /// The first violation in a deterministic order (shapes in canonical term
    /// order, predicates in IRI order), naming the term and the node.
    pub(crate) fn check_well_formed(&self) -> Result<(), String> {
        if let Some((subject, _, object)) = self
            .quads_with(None, Some(ENTAILMENT), None)
            .into_iter()
            .next()
        {
            return Err(format!(
                "the shapes graph declares {subject} sh:entailment {object}; this processor \
                 supports no entailment regime for validation, and SHACL requires a processor \
                 to signal a failure for a regime it does not support"
            ));
        }

        let parameter_declarations: FastSet<Term> = self
            .quads_with(None, Some(sh::PARAMETER_PROPERTY), None)
            .into_iter()
            .map(|(_, _, object)| object)
            .collect();
        for shape in self.spec_shapes() {
            let is_parameter = parameter_declarations.contains(&shape);
            self.check_shape_node(&shape, is_parameter)?;
        }
        Ok(())
    }

    /// The shapes of the shapes graph, by SHACL 1.2 Core §2.1's definition, in
    /// canonical term order.
    fn spec_shapes(&self) -> Vec<Term> {
        let mut shapes: FastSet<Term> = FastSet::default();
        for class in [sh::NODE_SHAPE, sh::PROPERTY_SHAPE, SHAPE_CLASS] {
            for (subject, _, _) in self.quads_with(None, Some(rdf::TYPE), Some(class)) {
                shapes.insert(subject);
            }
        }
        // A parameter declaration is an `sh:Parameter`, which SHACL makes a
        // subclass of `sh:PropertyShape`: the objects of `sh:parameter` are
        // property shapes whether or not they carry the type.
        for (_, _, object) in self.quads_with(None, Some(sh::PARAMETER_PROPERTY), None) {
            if matches!(object, Term::NamedNode(_) | Term::BlankNode(_)) {
                shapes.insert(object);
            }
        }
        let mut subject_predicates: Vec<&str> = crate::spec::target_predicates().collect();
        subject_predicates.push("http://www.w3.org/ns/shacl#targetWhere");
        for component in crate::spec::implemented().components() {
            for p in component.params() {
                subject_predicates.push(p.path);
            }
        }
        for predicate in subject_predicates {
            for (subject, _, _) in self.quads_with(None, Some(predicate), None) {
                shapes.insert(subject);
            }
        }
        for component in crate::spec::implemented().components() {
            for p in component.params() {
                match p.value {
                    ValueRule::Shape => {
                        for (_, _, object) in self.quads_with(None, Some(p.path), None) {
                            if matches!(object, Term::NamedNode(_) | Term::BlankNode(_)) {
                                shapes.insert(object);
                            }
                        }
                    }
                    ValueRule::ShapeList => {
                        for (_, _, object) in self.quads_with(None, Some(p.path), None) {
                            // An ill-formed list is refused by the value check on
                            // the shape that carries it; here only its members are
                            // wanted, and a list that cannot be walked has none.
                            if let Ok(members) = self.walk_rdf_list(&object, &object) {
                                shapes.extend(members.into_iter().filter(|member| {
                                    matches!(member, Term::NamedNode(_) | Term::BlankNode(_))
                                }));
                            }
                        }
                    }
                    ValueRule::Iri
                    | ValueRule::NodeKind
                    | ValueRule::NonNegativeInteger
                    | ValueRule::Boolean
                    | ValueRule::StringLiteral
                    | ValueRule::Literal
                    | ValueRule::Term
                    | ValueRule::TermList
                    | ValueRule::IriList
                    | ValueRule::LanguageTagList
                    | ValueRule::SparqlConstraint
                    | ValueRule::NodeExpression
                    | ValueRule::IriOrIriList
                    | ValueRule::Path
                    | ValueRule::ClosedValue => {}
                }
            }
        }
        let mut out: Vec<Term> = shapes.into_iter().collect();
        crate::term::sort_terms_canonical(&mut out);
        out
    }

    /// Check one shape node. `is_parameter` marks a parameter declaration (an
    /// object of `sh:parameter`), which SHACL makes a property shape that may also
    /// carry the declaration vocabulary.
    fn check_shape_node(&self, shape: &Term, is_parameter: bool) -> Result<(), String> {
        let is_property_shape = self.first_object_of(shape, sh::PATH).is_some();
        // A shape may ALSO be the SPARQL-based constraint or target it names
        // (`ex:S sh:sparql ex:S`, as the W3C suite's `sparql/node/sparql-002`
        // does), and then it legitimately carries that executable's query.
        let is_sparql_executable = self.has_type(shape, sh::SPARQL_CONSTRAINT)
            || self.has_type(shape, sh::SPARQL_TARGET)
            || [sh::SPARQL, sh::TARGET].into_iter().any(|predicate| {
                !native_quads(
                    self.data,
                    None,
                    Some(&Term::NamedNode(NamedNode::from(predicate))),
                    Some(shape),
                    GraphFilter::AnyGraph,
                )
                .is_empty()
            });
        let mut statements: Vec<(NamedNode, Term)> =
            native_quads(self.data, Some(shape), None, None, GraphFilter::AnyGraph)
                .into_iter()
                .map(|(_, predicate, object)| (predicate, object))
                .collect();
        statements.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));
        for (predicate, object) in &statements {
            let p = predicate.as_str();
            self.check_statement_annotations(shape, predicate, object)?;
            if p == rdf::TYPE {
                self.check_shape_type(shape, object)?;
                continue;
            }
            if !census::is_census_namespace(p) {
                continue;
            }
            // A parameter the shapes graph's own constraint components declare is
            // that component's, whatever its namespace: the component registry
            // evaluates it.
            if self.component_registry.by_parameter_path.contains_key(p) {
                continue;
            }
            // A parameter of a declared component this engine does not evaluate:
            // named with its component, so the author knows which feature it is.
            if let Some((_, component)) =
                crate::spec::unimplemented_component_params().find(|(param, _)| *param == p)
            {
                return Err(format!(
                    "shape {shape} uses <{p}>, a parameter of <{}>, which is a SHACL 1.2 Core \
                     component this engine does not implement; the shape is refused rather than \
                     silently conforming",
                    component.iri()
                ));
            }
            let Some(row) = census::classify(p) else {
                return Err(format!(
                    "shape {shape} carries <{p}>, which is not a term of SHACL 1.2, SHACL \
                     Advanced Features or SHACL-SPARQL; the shape is refused rather than \
                     silently ignoring it (a misspelled parameter checks nothing)"
                ));
            };
            if let TermClass::Unimplemented(why) = row.class {
                if is_parameter && p == sh::DEFAULT_VALUE {
                    continue;
                }
                return Err(format!(
                    "shape {shape} uses <{p}>, which is not evaluated by this engine: {why}; the \
                     shape is refused rather than validated as if it were absent"
                ));
            }
            let allowed = if is_parameter {
                row.on_parameter_declaration()
            } else {
                row.on_shape()
            } || (is_sparql_executable && SPARQL_EXECUTABLE_TERMS.contains(&p));
            if !allowed {
                return Err(format!(
                    "shape {shape} carries <{p}>, which is {} and not a property of a shape",
                    describe_class(row.class)
                ));
            }
            match row.class {
                TermClass::ConstraintParameter { value, .. } => {
                    self.check_parameter_value(shape, p, value, object)?;
                    if !is_property_shape
                        && crate::spec::implemented()
                            .components()
                            .iter()
                            .flat_map(crate::spec::ComponentRow::params)
                            .any(|param| param.path == p && param.property_only)
                    {
                        return Err(format!(
                            "shape {shape} has no sh:path, so it is a node shape, and node shapes \
                             cannot have any value for <{p}> (SHACL 1.2 Core)"
                        ));
                    }
                }
                TermClass::Target => self.check_target_value(shape, p, object)?,
                TermClass::Structural(Role::ShapeCharacteristic) => {
                    self.check_characteristic_value(shape, p, object)?;
                }
                TermClass::NonValidating => check_non_validating_value(shape, p, object)?,
                TermClass::Structural(_) | TermClass::Rule => {}
                TermClass::Unimplemented(_) => {}
            }
        }
        Ok(())
    }

    /// The RDF 1.2 reifiers of the shape statement `(shape, predicate, object)`:
    /// a SHACL term on one is a per-constraint annotation (SHACL 1.2 Core §3.7.1:
    /// "A triple that has a shape as subject, a parameter (such as sh:minCount) as
    /// predicate can have at most one reifier with a value for the property
    /// sh:deactivated"; likewise `sh:severity` and `sh:message`), which this engine
    /// does not evaluate, so it is refused rather than leaving the constraint
    /// unannotated. A non-validating annotation (`sh:formalized` on an
    /// `sh:intent`) changes no answer and loads.
    fn check_statement_annotations(
        &self,
        shape: &Term,
        predicate: &NamedNode,
        object: &Term,
    ) -> Result<(), String> {
        // The statement's triple term exists only when something reifies it.
        let (Some(s), Some(p), Some(o)) = (
            crate::data::resolve_id(self.data, shape),
            self.data.term_id_by_iri(predicate.as_str()),
            crate::data::resolve_id(self.data, object),
        ) else {
            return Ok(());
        };
        let Some(statement) = self.data.term_id_by_triple(s, p, o) else {
            return Ok(());
        };
        for reifier in self
            .data
            .reifier_quads()
            .filter(|quad| quad.o == statement)
            .map(|quad| quad.s)
        {
            for annotation in self
                .data
                .annotation_quads()
                .filter(|quad| quad.s == reifier)
                .map(|quad| quad.p)
            {
                let ::purrdf::TermRef::Iri(a) = self.data.resolve(annotation) else {
                    return Err(format!(
                        "the reifier of shape {shape}'s <{}> statement carries a non-IRI \
                         predicate",
                        predicate.as_str()
                    ));
                };
                if !census::is_census_namespace(a)
                    || census::classify(a).is_some_and(|row| row.class == TermClass::NonValidating)
                {
                    continue;
                }
                return Err(format!(
                    "shape {shape} annotates its <{}> statement with <{a}> on a reifier; a \
                     per-constraint reifier annotation is not evaluated by this engine, so the \
                     shape is refused rather than validated without it",
                    predicate.as_str()
                ));
            }
        }
        Ok(())
    }

    /// A shape's `rdf:type`: an unimplemented SHACL class is refused, and so is a
    /// blank node that is both an `rdfs:Class` and a shape (an implicit class
    /// target has to be named by an IRI).
    fn check_shape_type(&self, shape: &Term, class: &Term) -> Result<(), String> {
        let Term::NamedNode(class) = class else {
            return Ok(());
        };
        if census::is_census_namespace(class.as_str())
            && let Some(row) = census::classify(class.as_str())
            && let TermClass::Unimplemented(why) = row.class
        {
            return Err(format!(
                "shape {shape} is typed <{}>, which is not evaluated by this engine: {why}",
                class.as_str()
            ));
        }
        if matches!(shape, Term::BlankNode(_))
            && (class.as_str() == sh::NODE_SHAPE || class.as_str() == sh::PROPERTY_SHAPE)
            && self.has_type(shape, rdfs::CLASS)
        {
            return Err(format!(
                "shape {shape} is a blank node typed both rdfs:Class and <{}>; an implicit class \
                 target has to be an IRI",
                class.as_str()
            ));
        }
        Ok(())
    }

    /// A constraint parameter's value against its [`ValueRule`].
    fn check_parameter_value(
        &self,
        shape: &Term,
        predicate: &str,
        rule: ValueRule,
        value: &Term,
    ) -> Result<(), String> {
        let is_node = |term: &Term| matches!(term, Term::NamedNode(_) | Term::BlankNode(_));
        let is_iri = |term: &Term| matches!(term, Term::NamedNode(_));
        let is_basic_kind = |term: &Term| {
            matches!(term, Term::NamedNode(n) if crate::shapes::NodeKindValue::from_iri(n.as_str())
                .is_some_and(|kind| kind.is_basic()))
        };
        let is_string = |term: &Term| super::node_expr::string_value(term).is_some();
        // A SHACL-list rule: the value must be a well-formed list, and every
        // member must meet `member` — the first that does not is named.
        let list = |member: &dyn Fn(&Term) -> bool, what: &str| -> Result<bool, String> {
            for item in self.list_members(value, shape, predicate)? {
                if !member(&item) {
                    return Err(format!(
                        "<{predicate}> list on shape {shape} contains {item}, a {what}; the value \
                         must be {}",
                        rule.describe()
                    ));
                }
            }
            Ok(true)
        };
        let ok = match rule {
            ValueRule::Iri => is_iri(value),
            ValueRule::NodeKind => match value {
                Term::NamedNode(n) => crate::shapes::NodeKindValue::from_iri(n.as_str()).is_some(),
                Term::BlankNode(_) => {
                    list(&is_basic_kind, "member that is not a basic sh:NodeKind")?
                }
                Term::Literal(_) | Term::Triple(_) => false,
            },
            ValueRule::NonNegativeInteger => crate::shapes::parse_u64(value).is_some(),
            ValueRule::Boolean => super::node_expr::boolean_value(value).is_some(),
            ValueRule::StringLiteral => is_string(value),
            ValueRule::Literal => matches!(value, Term::Literal(_)),
            ValueRule::Term | ValueRule::NodeExpression => true,
            ValueRule::TermList => list(&|_| true, "member")?,
            ValueRule::IriList => list(&is_iri, "non-IRI member")?,
            ValueRule::LanguageTagList => {
                list(&is_string, "member that is not an xsd:string literal")?
            }
            ValueRule::Shape | ValueRule::SparqlConstraint => is_node(value),
            ValueRule::ShapeList => list(&is_node, "member that is not a shape")?,
            ValueRule::IriOrIriList => match value {
                Term::NamedNode(_) => true,
                Term::BlankNode(_) => list(&is_iri, "non-IRI member")?,
                Term::Literal(_) | Term::Triple(_) => false,
            },
            ValueRule::Path => {
                self.parse_path(value, shape, &mut FastSet::default())
                    .map_err(|e| {
                        format!(
                            "<{predicate}> on shape {shape} must be {}, got {value}: {e}",
                            rule.describe()
                        )
                    })?;
                true
            }
            ValueRule::ClosedValue => match value {
                Term::NamedNode(n) if n.as_str() == sh::BY_TYPES => {
                    return Err(format!(
                        "shape {shape} uses sh:closed sh:ByTypes, which is not evaluated by this \
                         engine; the shape is refused rather than validated as if it were open"
                    ));
                }
                other => super::node_expr::boolean_value(other).is_some(),
            },
        };
        if ok {
            Ok(())
        } else {
            Err(format!(
                "<{predicate}> on shape {shape} must be {}, got {value}",
                rule.describe()
            ))
        }
    }

    /// The members of the SHACL list `value` of `predicate` on `shape`, refusing a
    /// value that is not a well-formed SHACL list.
    fn list_members(
        &self,
        value: &Term,
        shape: &Term,
        predicate: &str,
    ) -> Result<Vec<Term>, String> {
        self.walk_rdf_list(value, shape)
            .map_err(|error| format!("<{predicate}> on shape {shape}: {error}"))
    }

    /// A target predicate's value (SHACL 1.2 Core §2.1.3).
    fn check_target_value(
        &self,
        shape: &Term,
        predicate: &str,
        value: &Term,
    ) -> Result<(), String> {
        let ok = match predicate {
            sh::TARGET_CLASS | sh::TARGET_SUBJECTS_OF | sh::TARGET_OBJECTS_OF => {
                matches!(value, Term::NamedNode(_))
            }
            // SHACL 1.2 Core §2.1.3.1: "Each value of sh:targetNode in a shape is a
            // well-formed node expression". The empty expression targets nothing;
            // a structured one is not evaluated here and is refused.
            sh::TARGET_NODE => {
                if matches!(value, Term::BlankNode(_)) && !self.is_empty_expression(value) {
                    return Err(format!(
                        "sh:targetNode on shape {shape} is the node expression {value}; a \
                         structured node-expression sh:targetNode value is not evaluated by this \
                         engine, so the shape is refused rather than targeting the blank node \
                         itself"
                    ));
                }
                true
            }
            _ => matches!(value, Term::NamedNode(_) | Term::BlankNode(_)),
        };
        if ok {
            Ok(())
        } else {
            Err(format!(
                "<{predicate}> on shape {shape} has the value {value}, which is not a valid target"
            ))
        }
    }

    /// A shape characteristic's value: `sh:path` a well-formed path, `sh:severity`
    /// a severity IRI this engine honours, `sh:message` a string literal,
    /// `sh:deactivated` an `xsd:boolean`, `sh:prefixes` a node.
    fn check_characteristic_value(
        &self,
        shape: &Term,
        predicate: &str,
        value: &Term,
    ) -> Result<(), String> {
        match predicate {
            sh::PATH => {
                self.parse_path(value, shape, &mut FastSet::default())?;
            }
            sh::SEVERITY => {
                let Term::NamedNode(level) = value else {
                    return Err(format!(
                        "sh:severity on shape {shape} must be an IRI, got {value}"
                    ));
                };
                if let Some(row) = census::classify(level.as_str())
                    && let TermClass::Unimplemented(why) = row.class
                {
                    return Err(format!(
                        "sh:severity on shape {shape} is <{}>, which is not evaluated by this \
                         engine: {why}",
                        level.as_str()
                    ));
                }
            }
            sh::MESSAGE => {
                if !is_text_literal(value, false) {
                    return Err(format!(
                        "sh:message on shape {shape} must be an xsd:string, rdf:langString or \
                         rdf:dirLangString literal, got {value}"
                    ));
                }
            }
            sh::DEACTIVATED => {
                if super::node_expr::boolean_value(value).is_none() {
                    return Err(format!(
                        "sh:deactivated on shape {shape} must be an xsd:boolean literal, got \
                         {value}; a node-expression value of sh:deactivated is not evaluated by \
                         this engine"
                    ));
                }
            }
            _ => {
                if !matches!(value, Term::NamedNode(_) | Term::BlankNode(_)) {
                    return Err(format!(
                        "<{predicate}> on shape {shape} must be an IRI or a blank node, got {value}"
                    ));
                }
            }
        }
        Ok(())
    }
}

/// The value checks SHACL 1.2 Core §5.7 states with "must" for a non-validating
/// property: `sh:name` and `sh:description` are text literals.
fn check_non_validating_value(shape: &Term, predicate: &str, value: &Term) -> Result<(), String> {
    let ok = match predicate {
        sh::NAME => is_text_literal(value, false),
        sh::DESCRIPTION => is_text_literal(value, true),
        _ => true,
    };
    if ok {
        Ok(())
    } else {
        Err(format!(
            "<{predicate}> on shape {shape} must be a literal with datatype xsd:string, \
             rdf:langString or rdf:dirLangString{}, got {value}",
            if predicate == sh::DESCRIPTION {
                " (or rdf:HTML)"
            } else {
                ""
            }
        ))
    }
}

/// Whether `value` is an `xsd:string` / `rdf:langString` / `rdf:dirLangString`
/// literal (or, when `html`, also an `rdf:HTML` one).
fn is_text_literal(value: &Term, html: bool) -> bool {
    matches!(value, Term::Literal(lit) if matches!(
        lit.datatype_str(),
        xsd::STRING | RDF_LANG_STRING | RDF_DIR_LANG_STRING
    ) || (html && lit.datatype_str() == RDF_HTML))
}

/// A class description for a diagnostic.
fn describe_class(class: TermClass) -> &'static str {
    match class {
        TermClass::ConstraintParameter { .. } => "a constraint parameter",
        TermClass::NonValidating => "a non-validating shape characteristic",
        TermClass::Target => "target vocabulary",
        TermClass::Rule => "rule vocabulary",
        TermClass::Unimplemented(_) => "a term this engine does not evaluate",
        TermClass::Structural(role) => match role {
            Role::ShapeCharacteristic => "a shape characteristic",
            Role::Path => "SHACL property-path vocabulary",
            Role::NodeExpression => "node-expression vocabulary",
            Role::Builtin => "a built-in component or function IRI",
            Role::Declaration => "declaration vocabulary",
            Role::ParameterDeclaration => "parameter-declaration vocabulary",
            Role::Prefixes => "prefix-declaration vocabulary",
            Role::Report => "validation-report vocabulary",
            Role::Graph => "graph-level vocabulary",
            Role::Vocabulary => "a SHACL class or individual",
        },
    }
}

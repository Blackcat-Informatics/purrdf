// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Shapes-graph well-formedness: every shape node, checked against the census.
//!
//! The parser reads the constraint parameters it knows off the shapes it reaches.
//! Everything else a shape node carries — an unknown `sh:` predicate, a term this
//! engine does not evaluate, a literal where SHACL requires an IRI — would
//! otherwise be walked past, and the shapes graph would load green while checking
//! less than it said. This pass checks it, once, before any shape is parsed:
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
//!   ([`crate::spec::census`]): an unknown term, a refused term, and a
//!   term that does not belong on a shape are load errors naming the term and
//!   the node; a constraint parameter's value must meet its value rule, and a
//!   parameter SHACL forbids on node shapes must not appear on one.
//!
//! The graph-level MUST SHACL places on a processor for an entailment regime it does
//! not support is enforced where the one regime this processor supports is read
//! (`Parser::parse_rule_graph`).

use ::purrdf::FastSet;

use super::shacl_instance::ShaclInstances;
use crate::data::{GraphFilter, native_quads, quads_for_pattern_ids};
use crate::model::{rdf, sh, xsd};
use crate::shapes::Parser;
use crate::spec::ValueRule;
use crate::spec::census::{self, Role, Site, TermClass};
use crate::term::{NamedNode, Term, term_id_to_native};

/// `rdf:langString`.
const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";
/// `rdf:dirLangString`.
const RDF_DIR_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString";
/// `rdf:HTML`.
const RDF_HTML: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#HTML";

/// The terms a SPARQL-based constraint node carries (SHACL 1.2 SPARQL
/// Extensions, "SPARQL-based Constraints" and "Annotation Properties"): its query
/// and the prefixes, message, severity, deactivation and result annotations it may
/// declare.
const SPARQL_EXECUTABLE_TERMS: [&str; 6] = [
    sh::SELECT,
    sh::PREFIXES,
    sh::MESSAGE,
    sh::SEVERITY,
    sh::DEACTIVATED,
    sh::RESULT_ANNOTATION,
];

/// The terms a validator of a SPARQL-based constraint component carries (SHACL 1.2
/// SPARQL Extensions, "Validators" and "Annotation Properties"): its ASK or SELECT
/// query (the one its type does not name is refused where the validator is
/// parsed), its prefixes, the message and severity it contributes, and its result
/// annotations.
const SPARQL_VALIDATOR_TERMS: [&str; 6] = [
    sh::ASK,
    sh::SELECT,
    sh::PREFIXES,
    sh::MESSAGE,
    sh::SEVERITY,
    sh::RESULT_ANNOTATION,
];

/// The terms a constraint component's declaration carries that the component
/// registry reads (SHACL 1.2 SPARQL Extensions, "Parameter Declarations" and
/// "Validators"): its parameters and its validators.
const COMPONENT_DECLARATION_TERMS: [&str; 4] = [
    sh::PARAMETER_PROPERTY,
    sh::VALIDATOR,
    sh::NODE_VALIDATOR,
    sh::PROPERTY_VALIDATOR,
];

/// The terms a `sh:SPARQLTarget` node carries (SHACL 1.2 SPARQL Extensions,
/// "SPARQL-based Targets"): its SELECT query and its prefixes.
const SPARQL_TARGET_TERMS: [&str; 2] = [sh::SELECT, sh::PREFIXES];

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
        let parameter_declarations: FastSet<Term> = self
            .quads_with(None, Some(sh::PARAMETER_PROPERTY), None)
            .into_iter()
            .map(|(_, _, object)| object)
            .collect();
        let mut instances = ShaclInstances::new(self.data);
        let shapes = self.spec_shapes();
        for shape in &shapes {
            let is_parameter = parameter_declarations.contains(shape);
            self.check_shape_node(shape, is_parameter)?;
            self.check_implicit_class_shape(shape, &mut instances)?;
        }
        let shapes: FastSet<Term> = shapes.into_iter().collect();
        self.check_sparql_executables(&shapes)
    }

    /// Check every SPARQL executable the loader reads — the SPARQL-based
    /// constraints (objects of `sh:sparql`), the validators of SPARQL-based
    /// constraint components (objects of `sh:validator`, `sh:nodeValidator` and
    /// `sh:propertyValidator`) and the `sh:SPARQLTarget`s — against the census, in
    /// canonical term order.
    ///
    /// Each reads a fixed set of terms and would walk past any other: a
    /// `sh:update` beside a constraint's `sh:select`, an `sh:ask` on a SPARQL-based
    /// constraint, a misspelled `sh:mesage`. Each of those is refused here, naming
    /// the term and the node. A non-validating term and any term outside the `sh:`
    /// and `shnex:` namespaces pass. A node that is also a shape of the shapes
    /// graph (`ex:S sh:sparql ex:S`) may also carry what a shape carries, which
    /// [`Self::check_shape_node`] has already checked.
    fn check_sparql_executables(&self, shapes: &FastSet<Term>) -> Result<(), String> {
        let mut executables: Vec<(Term, &'static str, &'static [&'static str])> = Vec::new();
        for (_, _, node) in self.quads_with(None, Some(sh::SPARQL), None) {
            executables.push((node, "SPARQL-based constraint", &SPARQL_EXECUTABLE_TERMS));
        }
        for attachment in [sh::VALIDATOR, sh::NODE_VALIDATOR, sh::PROPERTY_VALIDATOR] {
            for (_, _, node) in self.quads_with(None, Some(attachment), None) {
                executables.push((node, "SPARQL validator", &SPARQL_VALIDATOR_TERMS));
            }
        }
        for (_, _, node) in self.quads_with(None, Some(sh::TARGET), None) {
            if self.has_type(&node, sh::SPARQL_TARGET) {
                executables.push((node, "SPARQL-based target", &SPARQL_TARGET_TERMS));
            }
        }
        executables.sort_by(|a, b| crate::term::canonical_cmp(&a.0, &b.0).then(a.1.cmp(b.1)));
        executables.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);
        for (node, kind, allowed) in &executables {
            if !matches!(node, Term::NamedNode(_) | Term::BlankNode(_)) {
                continue;
            }
            let is_shape = shapes.contains(node);
            let mut predicates: Vec<NamedNode> =
                native_quads(self.data, Some(node), None, None, GraphFilter::AnyGraph)
                    .into_iter()
                    .map(|(_, predicate, _)| predicate)
                    .collect();
            predicates.sort();
            predicates.dedup();
            for predicate in &predicates {
                let p = predicate.as_str();
                if !census::is_census_namespace(p) || allowed.contains(&p) {
                    continue;
                }
                let Some(row) = census::classify(p) else {
                    return Err(format!(
                        "{kind} {node} carries <{p}>, which is not a term of SHACL 1.2, SHACL \
                         Advanced Features or SHACL-SPARQL; it is refused rather than silently \
                         ignored"
                    ));
                };
                if let TermClass::Refused(why) = row.class {
                    return Err(format!(
                        "{kind} {node} uses <{p}>, which is not evaluated by this engine: {why}"
                    ));
                }
                if row.class == TermClass::NonValidating || (is_shape && row.on_shape()) {
                    continue;
                }
                return Err(format!(
                    "{kind} {node} carries <{p}>, which is {} and not read on a {kind}{}; it is \
                     refused rather than silently ignored",
                    describe_class(row.class),
                    census::no_processing_note(p)
                ));
            }
        }
        Ok(())
    }

    /// The shapes of the shapes graph, by SHACL 1.2 Core §2.1's definition, in
    /// canonical term order.
    fn spec_shapes(&self) -> Vec<Term> {
        self.shapes_of_graph(None)
    }

    /// The shapes an EXPLICIT SHAPE TARGET can name (SHACL 1.2 Core, "Explicit
    /// shape targets": "If s is a shape in a shapes graph and n is a node in the
    /// data graph. If n has value s for sh:shape in the data graph, then n is a
    /// target for s"), in canonical term order.
    ///
    /// The shapes of [`Self::spec_shapes`] minus the PARAMETER DECLARATIONS — the
    /// objects of `sh:parameter` and the SHACL instances of `sh:Parameter` — and
    /// minus whatever is a shape only because a parameter declaration says so (the
    /// vocabulary's `sh:MemberShapeConstraintComponent-memberShape` carries
    /// `sh:node sh:NodeShape`). A parameter declaration declares a component's
    /// parameter; it is never validated, so nothing it mentions is a shape anything
    /// could be targeted at.
    pub(crate) fn targetable_shapes(&self) -> Vec<Term> {
        let mut declarations: FastSet<Term> = self
            .quads_with(None, Some(sh::PARAMETER_PROPERTY), None)
            .into_iter()
            .map(|(_, _, object)| object)
            .collect();
        let mut instances = ShaclInstances::new(self.data);
        let parameter_class = self.data.term_id_by_iri(sh::PARAMETER);
        if let Some(rdf_type) = self.data.term_id_by_iri(rdf::TYPE) {
            let typed: Vec<::purrdf::TermId> =
                quads_for_pattern_ids(self.data, None, Some(rdf_type), None, GraphFilter::AnyGraph)
                    .map(|quad| quad.s)
                    .collect();
            for node in typed {
                if instances.is_instance(node, parameter_class) {
                    declarations.insert(term_id_to_native(self.data, node));
                }
            }
        }
        self.shapes_of_graph(Some(&declarations))
    }

    /// [`Self::spec_shapes`], or — with `declarations` — the same definition read
    /// over every statement whose subject is not one of `declarations`, and with
    /// `declarations` themselves left out.
    fn shapes_of_graph(&self, declarations: Option<&FastSet<Term>>) -> Vec<Term> {
        let skip = |subject: &Term| declarations.is_some_and(|set| set.contains(subject));
        let mut shapes: FastSet<Term> = FastSet::default();
        // "s is a SHACL instance of sh:NodeShape or sh:PropertyShape" — through
        // `rdfs:subClassOf*` and `sh:ShapeClass` (see `shacl_instance`).
        let mut instances = ShaclInstances::new(self.data);
        let mut typed = ::purrdf::IdSet::default();
        if let Some(rdf_type) = self.data.term_id_by_iri(rdf::TYPE) {
            for quad in
                quads_for_pattern_ids(self.data, None, Some(rdf_type), None, GraphFilter::AnyGraph)
            {
                typed.insert(quad.s);
            }
        }
        for node in typed {
            if instances.is_node_shape(node) || instances.is_property_shape(node) {
                let shape = term_id_to_native(self.data, node);
                if !skip(&shape) {
                    shapes.insert(shape);
                }
            }
        }
        // A parameter declaration is an `sh:Parameter`, which SHACL makes a
        // subclass of `sh:PropertyShape`: the objects of `sh:parameter` are
        // property shapes whether or not they carry the type.
        if declarations.is_none() {
            for (_, _, object) in self.quads_with(None, Some(sh::PARAMETER_PROPERTY), None) {
                if matches!(object, Term::NamedNode(_) | Term::BlankNode(_)) {
                    shapes.insert(object);
                }
            }
        }
        let mut subject_predicates: Vec<&str> = crate::spec::target_predicates().collect();
        for component in crate::spec::implemented().components() {
            for p in component.params() {
                subject_predicates.push(p.path);
            }
        }
        for predicate in subject_predicates {
            for (subject, _, _) in self.quads_with(None, Some(predicate), None) {
                if !skip(&subject) {
                    shapes.insert(subject);
                }
            }
        }
        // "Each value of sh:targetWhere in a shape is a well-formed shape."
        for (subject, _, object) in self.quads_with(None, Some(sh::TARGET_WHERE), None) {
            if matches!(object, Term::NamedNode(_) | Term::BlankNode(_)) && !skip(&subject) {
                shapes.insert(object);
            }
        }
        for component in crate::spec::implemented().components() {
            for p in component.params() {
                match p.value {
                    ValueRule::Shape => {
                        for (subject, _, object) in self.quads_with(None, Some(p.path), None) {
                            if matches!(object, Term::NamedNode(_) | Term::BlankNode(_))
                                && !skip(&subject)
                            {
                                shapes.insert(object);
                            }
                        }
                    }
                    ValueRule::ShapeList => {
                        for (subject, _, object) in self.quads_with(None, Some(p.path), None) {
                            if skip(&subject) {
                                continue;
                            }
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
        let mut out: Vec<Term> = shapes.into_iter().filter(|shape| !skip(shape)).collect();
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
        // A shape may ALSO be a constraint component the registry has linked (TOSH's
        // `tosh:MemberShapeConstraintComponent` carries `sh:targetClass`), and then it
        // legitimately carries that component's declaration: the registry reads its
        // parameters and validators, so none of them is silently ignored.
        let is_component_declaration = match shape {
            Term::NamedNode(iri) => {
                self.component_registry
                    .components
                    .contains_key(iri.as_str())
                    || (crate::spec::component(iri.as_str()).is_some()
                        && self.has_type(shape, sh::CONSTRAINT_COMPONENT))
            }
            _ => false,
        };
        let mut statements: Vec<(NamedNode, Term)> =
            native_quads(self.data, Some(shape), None, None, GraphFilter::AnyGraph)
                .into_iter()
                .map(|(_, predicate, object)| (predicate, object))
                .collect();
        statements.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));
        for (predicate, object) in &statements {
            let p = predicate.as_str();
            self.check_statement_annotations(shape, predicate, object, is_parameter)?;
            if is_component_declaration && COMPONENT_DECLARATION_TERMS.contains(&p) {
                continue;
            }
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
            let Some(row) = census::classify(p) else {
                return Err(format!(
                    "shape {shape} carries <{p}>, which is not a term of SHACL 1.2, SHACL \
                     Advanced Features or SHACL-SPARQL; the shape is refused rather than \
                     silently ignoring it (a misspelled parameter checks nothing)"
                ));
            };
            let site = if is_parameter {
                Site::ParameterDeclaration
            } else {
                Site::Shape
            };
            if row.class_at(site) == TermClass::NonValidating
                && row.class != TermClass::NonValidating
            {
                // A term the specification makes documentation at this position
                // (`sh:defaultValue` on a parameter declaration): any value.
                continue;
            }
            if let TermClass::Refused(why) = row.class {
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
                    "shape {shape} carries <{p}>, which is {} and not a property of a shape{}",
                    describe_class(row.class),
                    census::no_processing_note(p)
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
                TermClass::Structural(Role::ComputedValues) => {
                    self.check_computed_values_site(shape, p, is_parameter)?;
                }
                TermClass::Structural(_) | TermClass::Rule => {}
                TermClass::Refused(_) => {}
            }
        }
        Ok(())
    }

    /// Where `sh:values` / `sh:defaultValue` may appear: on a property shape whose
    /// `sh:path` is an IRI, and nowhere else.
    ///
    /// SHACL 1.2 Core, "Property Shapes": "A property shape can only have values
    /// for sh:values and/or sh:defaultValue when its value for sh:path is a
    /// Predicate Path." A node shape has no path, so it has no value nodes these
    /// could add to ("For node shapes the value nodes are the individual focus
    /// nodes, forming a set with exactly one member"). A parameter declaration is
    /// never validated, so an `sh:values` on one would compute value nodes for
    /// nothing; its `sh:defaultValue` is documentation and never reaches here
    /// (see [`census::CensusRow::class_at`]).
    fn check_computed_values_site(
        &self,
        shape: &Term,
        predicate: &str,
        is_parameter: bool,
    ) -> Result<(), String> {
        if is_parameter {
            return Err(format!(
                "parameter declaration {shape} carries <{predicate}>; a parameter declaration \
                 is never validated, so the value nodes it would compute would be computed for \
                 nothing"
            ));
        }
        match self.first_object_of(shape, sh::PATH) {
            None => Err(format!(
                "shape {shape} has no sh:path, so it is a node shape, and carries \
                 <{predicate}>; SHACL 1.2 Core: \"A property shape can only have values for \
                 sh:values and/or sh:defaultValue when its value for sh:path is a Predicate \
                 Path\""
            )),
            Some(Term::NamedNode(_)) => Ok(()),
            Some(path) => Err(format!(
                "property shape {shape} carries <{predicate}> but its sh:path {path} is not an \
                 IRI; SHACL 1.2 Core: \"A property shape can only have values for sh:values \
                 and/or sh:defaultValue when its value for sh:path is a Predicate Path\""
            )),
        }
    }

    /// The RDF 1.2 reifiers of the shape statement `(shape, predicate, object)`.
    ///
    /// `sh:deactivated`, `sh:severity` and `sh:message` on one are the
    /// per-constraint annotations of SHACL 1.2 Core ("A triple that has a shape as
    /// subject, a parameter (such as sh:minCount) as predicate can have at most
    /// one reifier with a value for the property sh:deactivated"; likewise
    /// `sh:severity` and `sh:message` "on a reifier for a triple where the shape
    /// is the subject and one of the parameters of the constraint is the
    /// predicate"). Each value is checked here, and every disagreement between
    /// reifiers refused ([`Parser::statement_annotation`]); the parser applies
    /// them ([`super::annotations`]).
    ///
    /// Refused, because no constraint could honour them:
    ///
    /// * one of the three on a statement whose predicate is not a constraint
    ///   parameter (`sh:targetClass`, `sh:path`, `sh:name`, `rdf:type`, …) —
    ///   SHACL defines the annotation only on a constraint's triples, and it would
    ///   otherwise deactivate or re-grade nothing;
    /// * one of the three on a PARAMETER DECLARATION's statement, which declares a
    ///   component's parameter rather than a constraint that validates anything;
    /// * any other SHACL term on a reifier of a shape statement.
    ///
    /// A non-validating annotation (`sh:formalized` on an `sh:intent`) changes no
    /// answer and loads.
    fn check_statement_annotations(
        &self,
        shape: &Term,
        predicate: &NamedNode,
        object: &Term,
        is_parameter: bool,
    ) -> Result<(), String> {
        let p = predicate.as_str();
        let mut constraint_annotation: Option<String> = None;
        for (annotation, _) in self.reifier_rows(shape, p, object).into_iter().flatten() {
            let a = annotation.as_str();
            if super::annotations::CONSTRAINT_ANNOTATIONS.contains(&a) {
                constraint_annotation.get_or_insert_with(|| a.to_owned());
                continue;
            }
            if !census::is_census_namespace(a)
                || census::classify(a).is_some_and(|row| row.class == TermClass::NonValidating)
            {
                continue;
            }
            return Err(format!(
                "shape {shape} annotates its <{p}> statement with <{a}> on a reifier; only \
                 sh:deactivated, sh:severity and sh:message annotate a constraint there, so the \
                 shape is refused rather than validated without it"
            ));
        }
        let Some(annotation) = constraint_annotation else {
            return Ok(());
        };
        if is_parameter {
            return Err(format!(
                "parameter declaration {shape} annotates its <{p}> statement with <{annotation}> \
                 on a reifier; a parameter declaration declares a component's parameter, not a \
                 constraint, so there is no constraint for the annotation to apply to"
            ));
        }
        let is_constraint_parameter = self.component_registry.by_parameter_path.contains_key(p)
            || census::classify(p)
                .is_some_and(|row| matches!(row.class, TermClass::ConstraintParameter { .. }));
        if !is_constraint_parameter {
            return Err(format!(
                "shape {shape} annotates its <{p}> statement with <{annotation}> on a reifier, but \
                 <{p}> is not a constraint parameter: SHACL defines sh:deactivated, sh:severity \
                 and sh:message on a reifier only for a triple whose predicate is a parameter of a \
                 constraint, so the annotation would apply to nothing"
            ));
        }
        self.statement_annotation(shape, p, object).map(|_| ())
    }

    /// A shape's `rdf:type`: a SHACL class the census refuses is a load error.
    fn check_shape_type(&self, shape: &Term, class: &Term) -> Result<(), String> {
        let Term::NamedNode(class) = class else {
            return Ok(());
        };
        if census::is_census_namespace(class.as_str())
            && let Some(row) = census::classify(class.as_str())
            && let TermClass::Refused(why) = row.class
        {
            return Err(format!(
                "shape {shape} is typed <{}>, which is not evaluated by this engine: {why}",
                class.as_str()
            ));
        }
        Ok(())
    }

    /// SHACL 1.2 Core, "Implicit Class Targets and sh:ShapeClass": "If s is a
    /// SHACL instance of sh:NodeShape or sh:PropertyShape in an RDF graph G and s
    /// is also a SHACL instance of rdfs:Class in G and s is not an IRI then s is an
    /// ill-formed shape in G." A SHACL instance of `sh:ShapeClass` is both.
    fn check_implicit_class_shape(
        &self,
        shape: &Term,
        instances: &mut ShaclInstances<'_>,
    ) -> Result<(), String> {
        if !matches!(shape, Term::BlankNode(_)) {
            return Ok(());
        }
        let Some(node) = crate::data::resolve_id(self.data, shape) else {
            return Ok(());
        };
        if instances.has_implicit_class_target(node) {
            return Err(format!(
                "shape {shape} is a blank node that is a SHACL instance of both rdfs:Class and \
                 sh:NodeShape or sh:PropertyShape (or of sh:ShapeClass); SHACL makes such a \
                 shape ill-formed, because its implicit class target has to be named by an IRI"
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
                Term::NamedNode(n) => n.as_str() == sh::BY_TYPES,
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
            // SHACL 1.2 Core, "Node targets": "Each value of sh:targetNode in a shape
            // is a well-formed node expression" — every term is one, and a
            // structured one's well-formedness is its parse.
            sh::TARGET_NODE => true,
            // SHACL 1.2 Core, "Explicit shape targets": "Each value of sh:shape is
            // an IRI."
            sh::SHAPE => matches!(value, Term::NamedNode(_)),
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
                    && let TermClass::Refused(why) = row.class
                {
                    return Err(format!(
                        "sh:severity on shape {shape} is <{}>, which is not evaluated by this \
                         engine: {why}",
                        level.as_str()
                    ));
                }
            }
            sh::MESSAGE => {
                if !is_text_literal(value, true) {
                    return Err(format!(
                        "sh:message on shape {shape} must be an xsd:string, rdf:langString, \
                         rdf:dirLangString or rdf:HTML literal, got {value}"
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
pub(crate) fn is_text_literal(value: &Term, html: bool) -> bool {
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
        TermClass::Refused(_) => "a term this engine does not evaluate",
        TermClass::Structural(role) => match role {
            Role::ShapeCharacteristic => "a shape characteristic",
            Role::Path => "SHACL property-path vocabulary",
            Role::NodeExpression => "node-expression vocabulary",
            Role::Builtin => "a built-in component or function IRI",
            Role::Declaration => "declaration vocabulary",
            Role::ParameterDeclaration => "parameter-declaration vocabulary",
            Role::ComputedValues => "a property shape's computed value nodes",
            Role::Prefixes => "prefix-declaration vocabulary",
            Role::Report => "validation-report vocabulary",
            Role::Graph => "graph-level vocabulary",
            Role::Vocabulary => "a SHACL class or individual",
        },
    }
}

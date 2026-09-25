// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! THE CENSUS: every `sh:` and `shnex:` term the engine can meet, classified.
//!
//! A shapes-graph loader that reads the predicates it knows and walks past the
//! rest turns every term it does not know into a silent no-op: a misspelled
//! `sh:minCont`, a `sh:values` this engine does not compute, a `sh:js` constraint
//! from an extension it does not implement — each loads green and checks nothing.
//! The census closes that door. Its universe is the UNION of
//!
//! * every term the vendored SHACL 1.2 vocabularies declare (`shacl.ttl`,
//!   `shnex.ttl`); and
//! * every SHACL Advanced Features 1.0 / SHACL-SPARQL term PurRDF reads today —
//!   the `sh::` / `shnex::` constants of [`crate::model`], which include the AF
//!   node-expression spellings (`sh:union`, `sh:count`, …), `sh:SPARQLFunction`
//!   and `sh:returnType`, `sh:rule` and its rule vocabulary, and the SPARQL
//!   validators — plus the SHACL JavaScript Extensions terms, which SHACL 1.2 does
//!   not define and this engine does not evaluate.
//!
//! and every term in it has exactly one [`TermClass`]. The census tests assert
//! both halves: every declared vocabulary term and every `model.rs` constant is
//! classified, read out of the files themselves rather than out of a second list.
//!
//! # What a class means at load
//!
//! On a SHAPE node and on a NODE-EXPRESSION node the loader consults the census
//! for every `sh:` / `shnex:` predicate:
//!
//! * a term the census does not know is a load error naming the term and node;
//! * a [`TermClass::Unimplemented`] term is a load error saying what is not
//!   evaluated, instead of silently validating as if it were absent;
//! * a term that is known but does not belong on that kind of node (a rule's
//!   `sh:subject` on a shape, a path form's `sh:inversePath` on a shape) is a load
//!   error;
//! * a [`TermClass::ConstraintParameter`] value is checked against its
//!   [`ValueRule`], and an ill-typed value is a load error.
//!
//! Derived rows — every constraint parameter, component, built-in function and
//! its parameters, node-expression alias, target predicate and rule type — come
//! from the spec symbol table ([`super::table`]); only the terms the table has
//! no row for are listed here, so there is no second list of either.

use std::sync::LazyLock;

use super::ValueRule;
use super::table::{self, Implementation};
use crate::model::{sh, shnex};

/// A SHACL-namespace IRI the census names but [`crate::model`] does not, because
/// nothing but the census reads it.
macro_rules! sh_iri {
    ($local:literal) => {
        concat!("http://www.w3.org/ns/shacl#", $local)
    };
}

/// What a term is, to this engine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TermClass {
    /// A parameter of a constraint component the engine evaluates, with the value
    /// rule a shape's value for it must meet.
    ConstraintParameter {
        /// The component the parameter belongs to (the first, for a parameter two
        /// components share).
        component: &'static str,
        /// What a value of the parameter must be.
        value: ValueRule,
    },
    /// A property SHACL defines as ignored by SHACL processors (SHACL 1.2 Core
    /// §5.7, "Non-Validating Shape Characteristics"): form building, code
    /// generation, documentation.
    NonValidating,
    /// Structure: vocabulary that shapes, paths, node expressions, declarations,
    /// prefixes and reports are built from.
    Structural(Role),
    /// A target predicate or target class.
    Target,
    /// Rule vocabulary (SHACL Advanced Features / SHACL 1.2 Rules).
    Rule,
    /// A term the engine does not evaluate. Using it where the loader looks is a
    /// load error carrying this reason, never a silent no-op.
    Unimplemented(&'static str),
}

/// The kind of structure a [`TermClass::Structural`] term belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// A characteristic of a shape that is not a constraint parameter: its path,
    /// severity, message, deactivation, or SPARQL prefixes.
    ShapeCharacteristic,
    /// A SHACL property-path form (`sh:inversePath`, …).
    Path,
    /// Node-expression vocabulary: a built-in function's key or parameter, an AF
    /// spelling of one, or `sh:this`.
    NodeExpression,
    /// A built-in constraint component or node-expression function IRI.
    Builtin,
    /// Declaration vocabulary: constraint components, parameters, validators,
    /// functions, SPARQL executables and target types.
    Declaration,
    /// A term only a PARAMETER DECLARATION (an object of `sh:parameter`) carries.
    ParameterDeclaration,
    /// Prefix-declaration vocabulary (`sh:declare`, `sh:prefix`, `sh:namespace`).
    Prefixes,
    /// Validation-report vocabulary.
    Report,
    /// Graph-level vocabulary (`sh:shapesGraph`, `sh:suggestedShapesGraph`).
    Graph,
    /// A class or an individual SHACL names (`sh:NodeShape`, `sh:IRI`,
    /// `sh:Violation`, …): an object, never a predicate.
    Vocabulary,
}

/// One classified term.
#[derive(Clone, Copy, Debug)]
pub struct CensusRow {
    /// The term's IRI.
    pub iri: &'static str,
    /// What the term is.
    pub class: TermClass,
}

impl CensusRow {
    /// Whether the term may be a predicate of a SHAPE node.
    #[must_use]
    pub fn on_shape(&self) -> bool {
        match self.class {
            TermClass::ConstraintParameter { .. } | TermClass::NonValidating => true,
            TermClass::Target => self.iri != sh::SPARQL_TARGET && self.iri != sh_iri!("Target"),
            TermClass::Rule => self.iri == sh::RULE,
            TermClass::Structural(role) => role == Role::ShapeCharacteristic,
            TermClass::Unimplemented(_) => false,
        }
    }

    /// Whether the term may be a predicate of a NODE-EXPRESSION node.
    ///
    /// The node-expression vocabulary itself, plus the annotations an expression
    /// constraint's node legitimately carries beside its expression
    /// (`sh:message`, `sh:severity`, `sh:deactivated`) and `sh:prefixes` for the
    /// SPARQL-based expressions.
    #[must_use]
    pub fn on_node_expression(&self) -> bool {
        match self.class {
            TermClass::Structural(Role::NodeExpression) => true,
            TermClass::Structural(Role::ShapeCharacteristic) => self.iri != sh::PATH,
            TermClass::ConstraintParameter { .. }
            | TermClass::NonValidating
            | TermClass::Structural(_)
            | TermClass::Target
            | TermClass::Rule
            | TermClass::Unimplemented(_) => false,
        }
    }

    /// Whether the term may be a predicate of a PARAMETER DECLARATION (an object
    /// of `sh:parameter`), in addition to everything a shape may carry: SHACL
    /// makes `sh:Parameter` a subclass of `sh:PropertyShape`.
    #[must_use]
    pub fn on_parameter_declaration(&self) -> bool {
        self.on_shape()
            || matches!(
                self.class,
                TermClass::Structural(Role::ParameterDeclaration)
            )
            || self.iri == sh::DEFAULT_VALUE
    }
}

// ── The explicit rows ─────────────────────────────────────────────────────────

const fn row(iri: &'static str, class: TermClass) -> CensusRow {
    CensusRow { iri, class }
}

const fn structural(iri: &'static str, role: Role) -> CensusRow {
    row(iri, TermClass::Structural(role))
}

const fn vocabulary(iri: &'static str) -> CensusRow {
    structural(iri, Role::Vocabulary)
}

const fn unimplemented(iri: &'static str, why: &'static str) -> CensusRow {
    row(iri, TermClass::Unimplemented(why))
}

const fn non_validating(iri: &'static str) -> CensusRow {
    row(iri, TermClass::NonValidating)
}

/// Why a SHACL JavaScript Extensions term is refused.
const JS: &str = "SHACL JavaScript Extensions are not part of SHACL 1.2 and are not evaluated by \
     this engine";

/// Why a rule term the rules engine does not implement is refused.
const RULES: &str = "this SHACL 1.2 Rules feature is not implemented by this engine";

/// Every term the spec symbol table has no row for, classified by hand. Each is
/// a term SHACL 1.2 Core, Node Expressions, SPARQL Extensions or Rules, SHACL
/// Advanced Features or SHACL-JS DEFINES; PurRDF mints none of them.
static EXPLICIT: &[CensusRow] = &[
    // ── Shapes (SHACL 1.2 Core §2) ──
    vocabulary(sh_iri!("Shape")),
    vocabulary(sh::NODE_SHAPE),
    vocabulary(sh::PROPERTY_SHAPE),
    unimplemented(
        sh_iri!("ShapeClass"),
        "sh:ShapeClass implicit class targets are not evaluated",
    ),
    structural(sh::PATH, Role::ShapeCharacteristic),
    structural(sh::SEVERITY, Role::ShapeCharacteristic),
    structural(sh::MESSAGE, Role::ShapeCharacteristic),
    structural(sh::DEACTIVATED, Role::ShapeCharacteristic),
    structural(sh::PREFIXES, Role::ShapeCharacteristic),
    unimplemented(
        sh_iri!("targetWhere"),
        "sh:targetWhere targets are not evaluated",
    ),
    unimplemented(
        sh_iri!("shape"),
        "sh:shape target declarations in the data graph are not read",
    ),
    unimplemented(
        sh::VALUES,
        "sh:values computes a property shape's value nodes (SHACL 1.2 Core §2.3), and this \
         engine does not evaluate it",
    ),
    unimplemented(
        sh::DEFAULT_VALUE,
        "sh:defaultValue computes a property shape's value nodes when no other value exists \
         (SHACL 1.2 Core §2.3), and this engine does not evaluate it",
    ),
    // The IRI value of `sh:closed` (§7.9.1), never a predicate.
    vocabulary(sh::BY_TYPES),
    // ── Node kinds (§4.1.3) ──
    vocabulary(sh_iri!("NodeKind")),
    vocabulary(sh::BLANK_NODE),
    vocabulary(sh::BLANK_NODE_OR_IRI),
    vocabulary(sh::BLANK_NODE_OR_LITERAL),
    vocabulary(sh::IRI),
    vocabulary(sh::IRI_OR_LITERAL),
    vocabulary(sh::LITERAL),
    vocabulary(sh::TRIPLE_TERM),
    // ── Graphs and processor configuration ──
    vocabulary(sh_iri!("Graph")),
    vocabulary(sh_iri!("DataGraph")),
    vocabulary(sh_iri!("ShapesGraph")),
    unimplemented(
        sh_iri!("RulesGraph"),
        "sh:RulesGraph rule graphs are not executed",
    ),
    structural(sh_iri!("shapesGraph"), Role::Graph),
    structural(sh_iri!("suggestedShapesGraph"), Role::Graph),
    unimplemented(
        sh_iri!("entailment"),
        "no entailment regime is supported by validation, and SHACL requires a processor to \
         signal a failure for a regime it does not support",
    ),
    unimplemented(
        sh_iri!("RulesEntailment"),
        "the sh:RulesEntailment regime is not supported",
    ),
    // ── Validation reports (§3.6) ──
    structural(sh::VALIDATION_REPORT, Role::Report),
    structural(sh_iri!("ProcessorConfiguration"), Role::Report),
    structural(sh::CONFORMS, Role::Report),
    structural(sh_iri!("conformsToShapesGraph"), Role::Report),
    structural(sh::CONFORMANCE_DISALLOWS, Role::Report),
    structural(sh::RESULT, Role::Report),
    structural(sh_iri!("shapesGraphWellFormed"), Role::Report),
    structural(sh_iri!("usedShapesGraph"), Role::Report),
    structural(sh_iri!("usedDataGraph"), Role::Report),
    structural(sh_iri!("usedConfiguration"), Role::Report),
    structural(sh_iri!("AbstractResult"), Role::Report),
    structural(sh::VALIDATION_RESULT, Role::Report),
    structural(sh::DETAIL, Role::Report),
    structural(sh::FOCUS_NODE, Role::Report),
    structural(sh::RESULT_MESSAGE, Role::Report),
    structural(sh::RESULT_PATH, Role::Report),
    structural(sh::RESULT_SEVERITY, Role::Report),
    structural(sh_iri!("sourceConstraint"), Role::Report),
    structural(sh::SOURCE_SHAPE, Role::Report),
    structural(sh::SOURCE_CONSTRAINT_COMPONENT, Role::Report),
    structural(sh::VALUE, Role::Report),
    // ── Severities (§3.6.2.4) ──
    vocabulary(sh_iri!("Severity")),
    // ── Property paths (§2.3.1) ──
    structural(sh::INVERSE_PATH, Role::Path),
    structural(sh::ALTERNATIVE_PATH, Role::Path),
    structural(sh::ZERO_OR_MORE_PATH, Role::Path),
    structural(sh::ONE_OR_MORE_PATH, Role::Path),
    structural(sh::ZERO_OR_ONE_PATH, Role::Path),
    // ── Declarations: components, parameters, validators (SHACL-SPARQL) ──
    structural(sh_iri!("Parameterizable"), Role::Declaration),
    structural(sh::PARAMETER, Role::Declaration),
    structural(sh::PARAMETER_PROPERTY, Role::Declaration),
    structural(sh::OPTIONAL, Role::ParameterDeclaration),
    structural(sh::KEY_PARAMETER, Role::ParameterDeclaration),
    non_validating(sh_iri!("labelTemplate")),
    structural(sh::CONSTRAINT_COMPONENT, Role::Declaration),
    structural(sh::VALIDATOR, Role::Declaration),
    structural(sh::NODE_VALIDATOR, Role::Declaration),
    structural(sh::PROPERTY_VALIDATOR, Role::Declaration),
    structural(sh_iri!("Validator"), Role::Declaration),
    structural(sh::SPARQL_ASK_VALIDATOR, Role::Declaration),
    structural(sh::SPARQL_SELECT_VALIDATOR, Role::Declaration),
    structural(sh::SPARQL_CONSTRAINT, Role::Declaration),
    // ── Node-expression functions (Node Expressions §6) ──
    structural(sh::NODE_EXPRESSION_FUNCTION, Role::Declaration),
    structural(sh::NAMED_PARAMETER_EXPRESSION_FUNCTION, Role::Declaration),
    structural(sh::LIST_PARAMETER_EXPRESSION_FUNCTION, Role::Declaration),
    structural(sh_iri!("NodeExpression"), Role::Declaration),
    structural(sh::NAMED_PARAMETER_EXPRESSION, Role::Declaration),
    structural(sh::LIST_PARAMETER_EXPRESSION, Role::Declaration),
    structural(sh::BODY_EXPRESSION, Role::Declaration),
    structural(sh::THIS, Role::NodeExpression),
    unimplemented(
        sh_iri!("minus"),
        "the SHACL Advanced Features sh:minus node expression is not evaluated; SHACL 1.2 \
         Node Expressions replaces it by shnex:remove",
    ),
    // ── SPARQL executables (SHACL 1.2 SPARQL Extensions) ──
    structural(sh_iri!("SPARQLExecutable"), Role::Declaration),
    structural(sh_iri!("SPARQLAskExecutable"), Role::Declaration),
    structural(sh::ASK, Role::Declaration),
    structural(sh_iri!("SPARQLConstructExecutable"), Role::Declaration),
    structural(sh_iri!("SPARQLDescribeExecutable"), Role::Declaration),
    unimplemented(
        sh_iri!("describe"),
        "SPARQL DESCRIBE executables are not evaluated",
    ),
    structural(sh_iri!("SPARQLSelectExecutable"), Role::Declaration),
    structural(sh_iri!("SPARQLUpdateExecutable"), Role::Declaration),
    unimplemented(
        sh_iri!("update"),
        "SPARQL UPDATE executables are not evaluated",
    ),
    structural(sh_iri!("PrefixDeclaration"), Role::Prefixes),
    structural(sh::DECLARE, Role::Prefixes),
    structural(sh::PREFIX, Role::Prefixes),
    structural(sh::NAMESPACE, Role::Prefixes),
    unimplemented(
        sh_iri!("resultAnnotation"),
        "SHACL-SPARQL result annotations are not produced",
    ),
    unimplemented(
        sh_iri!("ResultAnnotation"),
        "SHACL-SPARQL result annotations are not produced",
    ),
    unimplemented(
        sh_iri!("annotationProperty"),
        "SHACL-SPARQL result annotations are not produced",
    ),
    unimplemented(
        sh_iri!("annotationValue"),
        "SHACL-SPARQL result annotations are not produced",
    ),
    unimplemented(
        sh_iri!("annotationVarName"),
        "SHACL-SPARQL result annotations are not produced",
    ),
    // ── Non-validating shape characteristics (Core §5.7) ──
    non_validating(sh_iri!("agentInstruction")),
    non_validating(sh_iri!("codeIdentifier")),
    non_validating(sh::DESCRIPTION),
    non_validating(sh_iri!("formalized")),
    non_validating(sh::GROUP),
    non_validating(sh_iri!("intent")),
    non_validating(sh::NAME),
    non_validating(sh::ORDER),
    non_validating(sh_iri!("unit")),
    vocabulary(sh_iri!("PropertyGroup")),
    // ── Rules (SHACL Advanced Features / SHACL 1.2 Rules) ──
    row(sh_iri!("Rule"), TermClass::Rule),
    row(sh::RULE, TermClass::Rule),
    row(sh::CONDITION, TermClass::Rule),
    row(sh::SUBJECT, TermClass::Rule),
    row(sh::PREDICATE, TermClass::Rule),
    row(sh::OBJECT, TermClass::Rule),
    row(sh::CONSTRUCT, TermClass::Rule),
    unimplemented(sh_iri!("expectedPredicate"), RULES),
    unimplemented(sh_iri!("layer"), RULES),
    unimplemented(sh_iri!("runOnce"), RULES),
    unimplemented(sh_iri!("RuleSet"), RULES),
    unimplemented(sh_iri!("includesRuleSet"), RULES),
    unimplemented(sh_iri!("hasRule"), RULES),
    unimplemented(sh_iri!("ruleProcessor"), RULES),
    unimplemented(sh_iri!("sourceRule"), RULES),
    unimplemented(sh_iri!("tempTriple"), RULES),
    unimplemented(sh_iri!("SPARQLRuleTemplate"), RULES),
    // ── Targets (SHACL Advanced Features) ──
    row(sh_iri!("Target"), TermClass::Target),
    row(sh::SPARQL_TARGET, TermClass::Target),
    structural(sh_iri!("TargetType"), Role::Declaration),
    structural(sh::SPARQL_TARGET_TYPE, Role::Declaration),
    // ── Functions (SHACL Advanced Features) ──
    structural(sh::FUNCTION, Role::Declaration),
    structural(sh::SPARQL_FUNCTION, Role::Declaration),
    structural(sh::RETURN_TYPE, Role::Declaration),
    // ── SHACL JavaScript Extensions (not SHACL 1.2) ──
    unimplemented(sh_iri!("js"), JS),
    unimplemented(sh_iri!("jsFunctionName"), JS),
    unimplemented(sh_iri!("jsLibrary"), JS),
    unimplemented(sh_iri!("jsLibraryURL"), JS),
    unimplemented(sh_iri!("JSConstraint"), JS),
    unimplemented(sh_iri!("JSConstraintComponent"), JS),
    unimplemented(sh_iri!("JSExecutable"), JS),
    unimplemented(sh_iri!("JSFunction"), JS),
    unimplemented(sh_iri!("JSLibrary"), JS),
    unimplemented(sh_iri!("JSRule"), JS),
    unimplemented(sh_iri!("JSTarget"), JS),
    unimplemented(sh_iri!("JSTargetType"), JS),
    unimplemented(sh_iri!("JSValidator"), JS),
];

// ── The census ────────────────────────────────────────────────────────────────

/// Every classified term: the explicit rows, then every row derived from the
/// spec symbol table for a term not already listed. One row per IRI.
fn build() -> Vec<CensusRow> {
    let mut rows: Vec<CensusRow> = EXPLICIT.to_vec();
    // Only the two namespaces the census is total over: a built-in's `rdf:first`
    // parameter is RDF vocabulary, not a SHACL term.
    let mut push = |candidate: CensusRow| {
        if is_census_namespace(candidate.iri)
            && !rows.iter().any(|existing| existing.iri == candidate.iri)
        {
            rows.push(candidate);
        }
    };
    for component in table::COMPONENTS {
        push(structural(component.iri, Role::Builtin));
        for p in component.params {
            push(row(
                p.path,
                TermClass::ConstraintParameter {
                    component: component.iri,
                    value: p.value,
                },
            ));
        }
    }
    for function in table::FUNCTIONS {
        push(structural(function.iri, Role::Builtin));
        for p in function.params {
            push(structural(p.path, Role::NodeExpression));
        }
        if let Implementation::Keyed { key, .. } = function.implementation {
            push(structural(key, Role::NodeExpression));
        }
    }
    for alias in table::KEY_ALIASES {
        push(structural(alias.iri, Role::NodeExpression));
    }
    push(structural(
        table::ARGUMENT_REFERENCE.0,
        Role::NodeExpression,
    ));
    for target in table::TARGETS {
        push(row(target.predicate, TermClass::Target));
    }
    for rule in table::RULE_TYPES {
        push(row(rule, TermClass::Rule));
    }
    for severity in table::SEVERITIES {
        push(vocabulary(severity));
    }
    rows
}

/// Every classified term, one row per IRI.
#[must_use]
pub fn census() -> &'static [CensusRow] {
    static ROWS: LazyLock<Vec<CensusRow>> = LazyLock::new(build);
    &ROWS
}

/// The census row for `iri`, if the census classifies it.
#[must_use]
pub fn classify(iri: &str) -> Option<&'static CensusRow> {
    census().iter().find(|row| row.iri == iri)
}

/// Whether `iri` is in a namespace the census is total over: `sh:` or `shnex:`.
#[must_use]
pub fn is_census_namespace(iri: &str) -> bool {
    iri.starts_with(sh::NS) || iri.starts_with(shnex::NS)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{TermClass, census, classify};
    use crate::model::sh;

    #[test]
    fn one_row_per_term() {
        let iris: BTreeSet<&str> = census().iter().map(|row| row.iri).collect();
        assert_eq!(iris.len(), census().len(), "a term is classified twice");
    }

    /// The explicit rows and the derived rows never disagree: a term the table
    /// derives is not also hand-listed (the hand-listed row would silently win).
    #[test]
    fn explicit_rows_do_not_shadow_derived_rows() {
        for explicit in super::EXPLICIT {
            if let Some(component) = super::table::COMPONENTS
                .iter()
                .find(|row| row.params.iter().any(|p| p.path == explicit.iri))
            {
                panic!(
                    "{} is a parameter of {} and also hand-classified",
                    explicit.iri,
                    component.iri()
                );
            }
        }
    }

    #[test]
    fn a_native_parameter_carries_its_value_rule() {
        let row = classify(sh::MIN_COUNT).expect("sh:minCount is classified");
        assert!(matches!(
            row.class,
            TermClass::ConstraintParameter {
                value: super::ValueRule::NonNegativeInteger,
                ..
            }
        ));
        assert!(row.on_shape());
        assert!(!row.on_node_expression());
    }
}

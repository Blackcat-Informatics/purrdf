// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SHACL-SPARQL custom constraint component registry, parsing, and evaluation.
//!
//! This module implements the machinery for user-declared constraint components
//! (`sh:ConstraintComponent`): discovering them in a shapes graph, parsing their
//! `sh:Parameter` declarations and SPARQL validators (`sh:nodeValidator`,
//! `sh:propertyValidator`, `sh:validator`), and evaluating those validators at
//! validation time. A [`ComponentRegistry`] is populated while the shapes graph is
//! parsed and is later consulted by the engine to bind component parameters and
//! run the matching ASK or SELECT query for each shape usage.

mod parameters;

use parameters::parse_parameter;
use std::sync::OnceLock;

use ::purrdf::TermValue;
use ::purrdf::{DatasetView, RdfDataset};
use ::purrdf::{FastMap, FastSet};

use crate::data::{GraphFilter, native_quads};
use crate::model::{rdf, rdfs, sh, xsd};
use crate::path;
use crate::report::{Severity, ValidationResult};
use crate::shapes::{ComponentValidator, Path, build_prefix_header};
use crate::sparql::{run_ask_with_shacl_prebinding_view, run_select_with_shacl_prebinding_view};
use crate::term::{NamedNode, Term, term_value_to_native};

/// Discriminator for a SPARQL validator's query form.
#[derive(Debug, Clone)]
pub(crate) enum ValidatorKind {
    /// An `ASK` query: a result of `true` means the focus node/value is valid.
    Ask,
    /// A `SELECT` query: each result row denotes a validation violation.
    Select,
}

/// A SPARQL validator attached to a constraint component.
#[derive(Debug, Clone)]
pub(crate) struct Validator {
    /// Whether the validator is an ASK or SELECT query.
    pub kind: ValidatorKind,
    /// Full query text with any `PREFIX` header already prepended.
    pub query_text: String,
    /// Optional human-readable message declared on the validator node.
    pub message: Option<String>,
    /// Optional severity declared on the validator node.
    pub severity: Option<Severity>,
}

/// Declaration of a single `sh:Parameter` for a constraint component.
#[derive(Debug, Clone)]
pub(crate) struct Parameter {
    /// The parameter predicate (`sh:path` of the parameter declaration).
    pub path: NamedNode,
    /// The SPARQL local name used to bind the parameter value in the validator.
    pub name: String,
    /// Whether the parameter is optional (`sh:optional true`).
    pub optional: bool,
}

/// A SHACL custom constraint component.
#[derive(Debug, Clone)]
pub(crate) struct Component {
    /// The component IRI (the `sh:ConstraintComponent` instance).
    pub id: NamedNode,
    /// Declared parameters, sorted by path IRI string for determinism.
    pub parameters: Vec<Parameter>,
    /// Node-scope validators (`sh:nodeValidator`).
    pub node_validators: Vec<Validator>,
    /// Property-scope validators (`sh:propertyValidator`).
    pub property_validators: Vec<Validator>,
    /// Generic validators (`sh:validator`).
    pub validators: Vec<Validator>,
    /// Optional human-readable message declared on the component node.
    pub message: Option<String>,
    /// Optional severity declared on the component node.
    pub severity: Option<Severity>,
}

/// Registry of custom constraint components keyed by component IRI string.
///
/// `by_parameter_path` maps a declared parameter predicate IRI string to the
/// owning component IRI (`NamedNode`), allowing the engine to recognize custom
/// constraint predicates while parsing shapes.
#[derive(Debug, Default, Clone)]
pub(crate) struct ComponentRegistry {
    /// Parameter predicate IRI string → owning component IRI.
    pub by_parameter_path: FastMap<String, NamedNode>,
    /// Component IRI string → component definition.
    pub components: FastMap<String, Component>,
}

impl ComponentRegistry {
    /// Parse all `sh:ConstraintComponent` (and subclass) declarations from the
    /// shapes graph into a registry.
    ///
    /// Discovery walks every `rdf:type` triple and keeps the subject when its
    /// class is `sh:ConstraintComponent` or a subclass thereof. Each discovered
    /// component's `sh:parameter` declarations and `sh:nodeValidator` /
    /// `sh:propertyValidator` / `sh:validator` SPARQL validators are parsed and
    /// validated (query must parse to the declared form and satisfy SHACL-SPARQL
    /// pre-binding restrictions). Any malformed component is a hard error.
    ///
    /// # Errors
    ///
    /// Returns `Err(String)` when a component, parameter, or validator is
    /// malformed or when a validator query violates the pre-binding restrictions.
    pub(crate) fn parse(
        data: &RdfDataset,
        doc_prefixes: &[(String, String)],
    ) -> Result<Self, String> {
        let rdf_type = Term::NamedNode(NamedNode::from(rdf::TYPE));
        let mut component_iris: Vec<String> = Vec::new();
        let mut seen: FastSet<String> = FastSet::default();
        let mut subclass_memo: FastMap<(String, String), bool> = FastMap::default();

        for (subject, _pred, object) in
            native_quads(data, None, Some(&rdf_type), None, GraphFilter::AnyGraph)
        {
            let Term::NamedNode(class) = object else {
                continue;
            };
            if !is_subclass_of(
                data,
                class.as_str(),
                sh::CONSTRAINT_COMPONENT,
                &mut subclass_memo,
            ) {
                continue;
            }
            let Term::NamedNode(component) = subject else {
                continue;
            };
            // Built-in components already have native parsing and evaluation.
            // Importing their RDF declarations must not add a second dispatch.
            if is_native_component(component.as_str()) {
                continue;
            }
            if seen.insert(component.as_str().to_owned()) {
                component_iris.push(component.as_str().to_owned());
            }
        }
        component_iris.sort();

        let mut registry = Self::default();
        for component_iri in component_iris {
            let component_term = Term::NamedNode(NamedNode::from(component_iri.as_str()));
            let component = parse_component(
                data,
                doc_prefixes,
                &component_term,
                &component_iri,
                &mut subclass_memo,
            )?;
            for param in &component.parameters {
                registry
                    .by_parameter_path
                    .insert(param.path.as_str().to_owned(), component.id.clone());
            }
            let id = component.id.as_str().to_owned();
            registry.components.insert(id, component);
        }
        Ok(registry)
    }
}

/// Components executed by the native SHACL engines, even when their vocabulary
/// declarations occur in the input dataset. Custom IRIs in the SHACL namespace
/// remain discoverable; this list identifies implementations, not namespaces.
fn is_native_component(iri: &str) -> bool {
    matches!(
        iri,
        "http://www.w3.org/ns/shacl#PropertyConstraintComponent"
            | sh::SPARQL_CONSTRAINT_COMPONENT
            | sh::EXPRESSION_CONSTRAINT_COMPONENT
            | sh::NODE_BY_EXPRESSION_CONSTRAINT_COMPONENT
            | sh::MIN_COUNT_CONSTRAINT_COMPONENT
            | sh::MAX_COUNT_CONSTRAINT_COMPONENT
            | sh::CLASS_CONSTRAINT_COMPONENT
            | sh::DATATYPE_CONSTRAINT_COMPONENT
            | sh::NODE_KIND_CONSTRAINT_COMPONENT
            | sh::IN_CONSTRAINT_COMPONENT
            | sh::HAS_VALUE_CONSTRAINT_COMPONENT
            | sh::PATTERN_CONSTRAINT_COMPONENT
            | sh::MIN_LENGTH_CONSTRAINT_COMPONENT
            | sh::UNIQUE_LANG_CONSTRAINT_COMPONENT
            | sh::MIN_INCLUSIVE_CONSTRAINT_COMPONENT
            | sh::MAX_INCLUSIVE_CONSTRAINT_COMPONENT
            | sh::MIN_EXCLUSIVE_CONSTRAINT_COMPONENT
            | sh::MAX_EXCLUSIVE_CONSTRAINT_COMPONENT
            | sh::AND_CONSTRAINT_COMPONENT
            | sh::OR_CONSTRAINT_COMPONENT
            | sh::XONE_CONSTRAINT_COMPONENT
            | sh::NODE_CONSTRAINT_COMPONENT
            | sh::REIFIER_SHAPE_CONSTRAINT_COMPONENT
            | sh::MAX_LENGTH_CONSTRAINT_COMPONENT
            | sh::NOT_CONSTRAINT_COMPONENT
            | sh::LANGUAGE_IN_CONSTRAINT_COMPONENT
            | sh::CLOSED_CONSTRAINT_COMPONENT
            | sh::EQUALS_CONSTRAINT_COMPONENT
            | sh::DISJOINT_CONSTRAINT_COMPONENT
            | sh::LESS_THAN_CONSTRAINT_COMPONENT
            | sh::LESS_THAN_OR_EQUALS_CONSTRAINT_COMPONENT
            | sh::QUALIFIED_MIN_COUNT_CONSTRAINT_COMPONENT
            | sh::QUALIFIED_MAX_COUNT_CONSTRAINT_COMPONENT
    )
}

/// Map an `sh:severity` object term to a [`Severity`]: the three built-in
/// `sh:` severities map to their variants, any OTHER IRI is preserved verbatim
/// (SHACL allows custom severity IRIs), and a non-IRI object yields `None`.
pub(crate) fn severity_from_term(t: &Term) -> Option<Severity> {
    match t {
        Term::NamedNode(n) => {
            Some(Severity::from_iri(n.as_str()).unwrap_or_else(|| Severity::Other(n.clone())))
        }
        _ => None,
    }
}

// ── Custom component validator evaluation ────────────────────────────────────

/// Substitute SHACL message templates of the form `{?varName}` and `{$varName}`
/// with the string rendering of the first matching binding. Unbound variables are
/// left unchanged.
///
/// SHACL-SPARQL §5.3.3 defines this for every SPARQL-based constraint, not only
/// for custom components, so [`crate::sparql::eval_sparql_constraint_view`] calls
/// it too — hence `pub(crate)` rather than private to this module.
pub(crate) fn substitute_message_templates(msg: &str, bindings: &[(String, Term)]) -> String {
    static TEMPLATE_RE: OnceLock<regex::Regex> = OnceLock::new();
    let re = TEMPLATE_RE.get_or_init(|| {
        regex::Regex::new(r"\{([$?])([A-Za-z_][A-Za-z0-9_]*)\}")
            .expect("static template regex is valid")
    });
    re.replace_all(msg, |caps: &regex::Captures<'_>| {
        let name = &caps[2];
        match bindings.iter().find(|(n, _)| n == name) {
            Some((_, term)) => match term {
                Term::Literal(lit) => lit.value().to_owned(),
                Term::NamedNode(n) => n.as_str().to_owned(),
                other => other.to_string(),
            },
            None => caps[0].to_owned(),
        }
    })
    .into_owned()
}

/// Evaluate an ASK validator for a custom constraint component.
///
/// Each value node is substituted as `$value` / `?value` alongside `$this` and the
/// parameter bindings. A result of `true` means conforming; `false` emits one
/// [`ValidationResult`].
#[allow(clippy::too_many_arguments)] // Signature mirrors the SHACL-SPARQL parameter set.
pub(crate) fn eval_ask_validator<D: DatasetView + Sync + crate::sparql::FocusGraphSource>(
    dataset: &D,
    focus: &Term,
    value_nodes: &[Term],
    validator: &ComponentValidator,
    bindings: &[(String, Term)],
    component: &NamedNode,
    source_shape: &Term,
    path: Option<&Path>,
    severity: &Severity,
    message: Option<&String>,
    shapes_graph_iri: Option<&str>,
    current_shape: Option<&Term>,
) -> Result<Vec<ValidationResult>, String> {
    let ComponentValidator::Ask { ask } = validator else {
        return Err("expected ASK validator, got SELECT".to_owned());
    };
    let mut results = Vec::with_capacity(value_nodes.len());
    let mut subs: Vec<(String, TermValue)> = Vec::with_capacity(2 + bindings.len());
    for v in value_nodes {
        subs.clear();
        subs.push(("this".to_owned(), focus.to_term_value()));
        subs.push(("value".to_owned(), v.to_term_value()));
        for (name, value) in bindings {
            subs.push((name.clone(), value.to_term_value()));
        }
        let conforms = run_ask_with_shacl_prebinding_view(
            dataset,
            ask,
            &subs,
            shapes_graph_iri,
            current_shape,
        )?;
        if !conforms {
            let mut template_bindings: Vec<(String, Term)> = bindings.to_vec();
            template_bindings.push(("value".to_owned(), v.clone()));
            results.push(ValidationResult {
                focus_node: focus.clone(),
                result_path: path.map(path::path_to_term),
                path_structure: path.filter(|p| !matches!(p, Path::Predicate(_))).cloned(),
                value: Some(v.clone()),
                source_constraint_component: component.clone(),
                source_shape: source_shape.clone(),
                severity: severity.clone(),
                message: message.map(|m| substitute_message_templates(m, &template_bindings)),
                source_box_roles: vec![],
                path_box_roles: vec![],
                result_box_roles: vec![],
                attributions: vec![],
            });
        }
    }
    Ok(results)
}

/// Evaluate a SELECT validator for a custom constraint component.
///
/// `$this` and the parameter bindings are substituted before evaluation. Each
/// result row maps to a [`ValidationResult`]; the `?this`, `?path`, and `?value`
/// columns override the default focus node, path, and value when present and
/// bound. Row bindings take precedence over parameter bindings for message
/// template substitution.
#[allow(clippy::too_many_arguments)] // Signature mirrors the SHACL-SPARQL parameter set.
pub(crate) fn eval_select_validator<D: DatasetView + Sync + crate::sparql::FocusGraphSource>(
    dataset: &D,
    focus: &Term,
    validator: &ComponentValidator,
    bindings: &[(String, Term)],
    component: &NamedNode,
    source_shape: &Term,
    path: Option<&Path>,
    severity: &Severity,
    message: Option<&String>,
    shapes_graph_iri: Option<&str>,
    current_shape: Option<&Term>,
) -> Result<Vec<ValidationResult>, String> {
    let ComponentValidator::Select { select } = validator else {
        return Err("expected SELECT validator, got ASK".to_owned());
    };
    let mut subs: Vec<(String, TermValue)> = Vec::with_capacity(1 + bindings.len());
    subs.push(("this".to_owned(), focus.to_term_value()));
    for (name, value) in bindings {
        subs.push((name.clone(), value.to_term_value()));
    }
    let query = crate::constraints::substitute_path_placeholder(select, path);
    let (variables, rows) = run_select_with_shacl_prebinding_view(
        dataset,
        &query,
        &subs,
        shapes_graph_iri,
        current_shape,
    )?;

    let this_index = variables.iter().position(|v| v == "this");
    let path_index = variables.iter().position(|v| v == "path");
    let value_index = variables.iter().position(|v| v == "value");

    let path_term = path.map(path::path_to_term);
    let path_structure = path.filter(|p| !matches!(p, Path::Predicate(_))).cloned();

    let mut results = Vec::with_capacity(rows.len());
    let mut row_bindings: Vec<(String, Term)> = Vec::with_capacity(variables.len());
    let mut template_bindings: Vec<(String, Term)> =
        Vec::with_capacity(variables.len() + bindings.len());
    for row in &rows {
        row_bindings.clear();
        row_bindings.extend(variables.iter().zip(row.iter()).filter_map(|(var, cell)| {
            cell.as_ref()
                .map(|tv| (var.clone(), term_value_to_native(tv)))
        }));

        let focus_node = this_index
            .and_then(|i| row.get(i))
            .and_then(Option::as_ref)
            .map_or_else(|| focus.clone(), term_value_to_native);

        let (result_path, result_path_structure) = if let Some(i) = path_index {
            if let Some(Some(tv)) = row.get(i) {
                (Some(term_value_to_native(tv)), None)
            } else {
                (path_term.clone(), path_structure.clone())
            }
        } else {
            (path_term.clone(), path_structure.clone())
        };

        let value = value_index
            .and_then(|i| row.get(i))
            .and_then(Option::as_ref)
            .map(term_value_to_native);

        template_bindings.clear();
        template_bindings.extend_from_slice(&row_bindings);
        for (name, value) in bindings {
            if !template_bindings.iter().any(|(n, _)| n == name) {
                template_bindings.push((name.clone(), value.clone()));
            }
        }

        results.push(ValidationResult {
            focus_node,
            result_path,
            path_structure: result_path_structure,
            value,
            source_constraint_component: component.clone(),
            source_shape: source_shape.clone(),
            severity: severity.clone(),
            message: message.map(|m| substitute_message_templates(m, &template_bindings)),
            source_box_roles: vec![],
            path_box_roles: vec![],
            result_box_roles: vec![],
            attributions: vec![],
        });
    }
    Ok(results)
}

// ── Internal helpers ─────────────────────────────────────────────────────────
///
/// Extract the SPARQL local name for an IRI: the substring after the last `/`,
/// `#`, or `:` delimiter. This is used to derive the variable name bound to a
/// declared component parameter.
///
/// ```ignore
/// use crate::components::sparql_local_name;
///
/// assert_eq!(sparql_local_name("http://example.org/ns#requiredParam"), "requiredParam");
/// assert_eq!(sparql_local_name("http://example.org/ns/requiredParam"), "requiredParam");
/// assert_eq!(sparql_local_name("ex:requiredParam"), "requiredParam");
/// ```
#[must_use]
pub(crate) fn sparql_local_name(iri: &str) -> String {
    let mut idx = iri.rfind('/').map_or(0, |i| i + 1);
    if let Some(i) = iri.rfind('#') {
        idx = idx.max(i + 1);
    }
    if let Some(i) = iri.rfind(':') {
        idx = idx.max(i + 1);
    }
    iri[idx..].to_owned()
}

// ── Internal helpers ─────────────────────────────────────────────────────────

/// Return all objects for `(subject, predicate, ?)`.
fn objects_of(data: &RdfDataset, subject: &Term, predicate: &str) -> Vec<Term> {
    let Some(subject_id) = crate::data::resolve_id(data, subject) else {
        return Vec::new();
    };
    let Some(predicate_id) = data.term_id_by_iri(predicate) else {
        return Vec::new();
    };
    let mut seen = ::purrdf::IdSet::default();
    crate::data::quads_for_pattern_ids(
        data,
        Some(subject_id),
        Some(predicate_id),
        None,
        GraphFilter::AnyGraph,
    )
    .filter(|quad| seen.insert(quad.o))
    .map(|quad| crate::term::term_id_to_native(data, quad.o))
    .collect()
}

/// Return the first object for `(subject, predicate, ?)`, if any.
fn first_object_of(data: &RdfDataset, subject: &Term, predicate: &str) -> Option<Term> {
    objects_of(data, subject, predicate).into_iter().next()
}

/// Whether `class_iri` is `target_iri` or a subclass thereof under
/// `rdfs:subClassOf` (reflexive transitive closure).
///
/// A memo table avoids repeated superclass walks; `false` is inserted before
/// recursion to break `rdfs:subClassOf` cycles.
fn is_subclass_of(
    data: &RdfDataset,
    class_iri: &str,
    target_iri: &str,
    memo: &mut FastMap<(String, String), bool>,
) -> bool {
    if class_iri == target_iri {
        return true;
    }
    let key = (class_iri.to_owned(), target_iri.to_owned());
    if let Some(&result) = memo.get(&key) {
        return result;
    }
    memo.insert(key.clone(), false);
    let class_term = Term::NamedNode(NamedNode::from(class_iri));
    let sub_class_of = Term::NamedNode(NamedNode::from(rdfs::SUB_CLASS_OF));
    let mut result = false;
    for (_subject, _pred, object) in native_quads(
        data,
        Some(&class_term),
        Some(&sub_class_of),
        None,
        GraphFilter::AnyGraph,
    ) {
        let Term::NamedNode(super_class) = object else {
            continue;
        };
        if is_subclass_of(data, super_class.as_str(), target_iri, memo) {
            result = true;
            break;
        }
    }
    memo.insert(key, result);
    result
}

/// Determine the validator kind from its `rdf:type` declarations, respecting
/// subclasses of `sh:SPARQLAskValidator` and `sh:SPARQLSelectValidator`.
fn validator_kind(
    data: &RdfDataset,
    validator: &Term,
    memo: &mut FastMap<(String, String), bool>,
) -> Result<ValidatorKind, String> {
    let mut is_ask = false;
    let mut is_select = false;
    for q in objects_of(data, validator, rdf::TYPE) {
        let Term::NamedNode(class) = q else {
            continue;
        };
        if is_subclass_of(data, class.as_str(), sh::SPARQL_ASK_VALIDATOR, memo) {
            is_ask = true;
        }
        if is_subclass_of(data, class.as_str(), sh::SPARQL_SELECT_VALIDATOR, memo) {
            is_select = true;
        }
    }
    if is_ask && is_select {
        return Err(format!(
            "validator {validator} is typed as both ASK and SELECT"
        ));
    }
    if is_ask {
        return Ok(ValidatorKind::Ask);
    }
    if is_select {
        return Ok(ValidatorKind::Select);
    }
    Err(format!(
        "validator {validator} must be typed as sh:SPARQLAskValidator or \
         sh:SPARQLSelectValidator (or a subclass)"
    ))
}

/// Whether `name` is a legal SPARQL VARNAME and not one of the reserved names
/// banned for SHACL-SPARQL parameter bindings.
fn is_valid_varname(name: &str) -> bool {
    const BANNED: &[&str] = &["this", "path", "PATH", "value"];
    if BANNED.contains(&name) {
        return false;
    }
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Parse a single SPARQL validator node attached to a component.
fn parse_validator(
    data: &RdfDataset,
    doc_prefixes: &[(String, String)],
    component: &Term,
    validator: &Term,
    param_names: &[String],
    subclass_memo: &mut FastMap<(String, String), bool>,
) -> Result<Validator, String> {
    let component_iri = match component {
        Term::NamedNode(n) => n.as_str(),
        _ => return Err(format!("component {component} is not a named node")),
    };
    let kind = validator_kind(data, validator, subclass_memo)
        .map_err(|e| format!("component {component_iri} validator {validator}: {e}"))?;

    let query_pred = match kind {
        ValidatorKind::Ask => sh::ASK,
        ValidatorKind::Select => sh::SELECT,
    };
    let raw_queries = objects_of(data, validator, query_pred);
    let raw_query = match raw_queries.as_slice() {
        [Term::Literal(literal)] if literal.datatype_str() == xsd::STRING => literal.value(),
        _ => {
            return Err(format!(
                "component {component_iri} validator {validator} must have exactly one {} xsd:string literal",
                match kind {
                    ValidatorKind::Ask => "sh:ask",
                    ValidatorKind::Select => "sh:select",
                }
            ));
        }
    };
    let query_text = format!(
        "{}{raw_query}",
        build_prefix_header(data, doc_prefixes, &[component, validator])
    );

    let query = match purrdf_sparql_algebra::SparqlParser::new().parse_query(&query_text) {
        Ok(q) => q,
        Err(e) => {
            return Err(format!(
                "component {component_iri} validator {validator} has an unparsable query: {e}"
            ));
        }
    };

    let got_form = match &query {
        purrdf_sparql_algebra::Query::Ask { .. } => "ASK",
        purrdf_sparql_algebra::Query::Select { .. } => "SELECT",
        _ => "non-ASK/SELECT",
    };
    let expected_form = match kind {
        ValidatorKind::Ask => "ASK",
        ValidatorKind::Select => "SELECT",
    };
    if got_form != expected_form {
        return Err(format!(
            "component {component_iri} validator {validator} is declared as {kind:?} but the \
             query text parses to a {got_form} query"
        ));
    }

    let mut prebound: Vec<&str> = vec!["this"];
    if matches!(kind, ValidatorKind::Ask) {
        prebound.push("value");
    }
    prebound.extend(param_names.iter().map(String::as_str));
    let prebinding_result = match kind {
        ValidatorKind::Ask => crate::prebinding::check_ask(&query, &prebound),
        ValidatorKind::Select => crate::prebinding::check_select(&query, &prebound),
    };
    prebinding_result.map_err(|e| {
        format!(
            "component {component_iri} validator {validator} violates pre-binding restrictions: \
             {e}"
        )
    })?;

    let mut messages: Vec<String> = objects_of(data, validator, sh::MESSAGE)
        .into_iter()
        .filter_map(|t| match t {
            Term::Literal(lit) => Some(lit.value().to_owned()),
            _ => None,
        })
        .collect();
    messages.sort();
    let message = messages.into_iter().next();
    let severity =
        first_object_of(data, validator, sh::SEVERITY).and_then(|t| severity_from_term(&t));

    Ok(Validator {
        kind,
        query_text,
        message,
        severity,
    })
}

/// Parse a single constraint component node and its parameters / validators.
fn parse_component(
    data: &RdfDataset,
    doc_prefixes: &[(String, String)],
    component: &Term,
    component_iri: &str,
    subclass_memo: &mut FastMap<(String, String), bool>,
) -> Result<Component, String> {
    let param_nodes: Vec<Term> = objects_of(data, component, sh::PARAMETER_PROPERTY);
    let mut parameters = Vec::with_capacity(param_nodes.len());
    for param_node in param_nodes {
        parameters.push(parse_parameter(data, &param_node, component_iri)?);
    }
    parameters.sort_by(|a, b| a.path.as_str().cmp(b.path.as_str()));
    if parameters.iter().all(|parameter| parameter.optional) {
        return Err(format!(
            "component {component_iri} must declare at least one non-optional parameter"
        ));
    }
    let mut names = FastSet::default();
    for parameter in &parameters {
        if !names.insert(parameter.name.as_str()) {
            return Err(format!(
                "component {component_iri} declares duplicate parameter name ?{}",
                parameter.name,
            ));
        }
    }
    let param_names: Vec<String> = parameters.iter().map(|p| p.name.clone()).collect();

    let mut node_validator_nodes: Vec<Term> = objects_of(data, component, sh::NODE_VALIDATOR);
    crate::term::sort_terms_canonical(&mut node_validator_nodes);
    let node_validators = node_validator_nodes
        .into_iter()
        .map(|v| {
            parse_validator(
                data,
                doc_prefixes,
                component,
                &v,
                &param_names,
                subclass_memo,
            )
        })
        .collect::<Result<Vec<Validator>, _>>()?;
    let mut property_validator_nodes: Vec<Term> =
        objects_of(data, component, sh::PROPERTY_VALIDATOR);
    crate::term::sort_terms_canonical(&mut property_validator_nodes);
    let property_validators = property_validator_nodes
        .into_iter()
        .map(|v| {
            parse_validator(
                data,
                doc_prefixes,
                component,
                &v,
                &param_names,
                subclass_memo,
            )
        })
        .collect::<Result<Vec<Validator>, _>>()?;
    let mut validator_nodes: Vec<Term> = objects_of(data, component, sh::VALIDATOR);
    crate::term::sort_terms_canonical(&mut validator_nodes);
    let validators = validator_nodes
        .into_iter()
        .map(|v| {
            parse_validator(
                data,
                doc_prefixes,
                component,
                &v,
                &param_names,
                subclass_memo,
            )
        })
        .collect::<Result<Vec<Validator>, _>>()?;

    for (attachment, parsed, expect_ask) in [
        (sh::NODE_VALIDATOR, &node_validators, false),
        (sh::PROPERTY_VALIDATOR, &property_validators, false),
        (sh::VALIDATOR, &validators, true),
    ] {
        if parsed
            .iter()
            .any(|validator| matches!(validator.kind, ValidatorKind::Ask) != expect_ask)
        {
            return Err(format!(
                "component {component_iri} {attachment} requires {} validators",
                if expect_ask { "ASK" } else { "SELECT" },
            ));
        }
    }

    let mut component_messages: Vec<String> = objects_of(data, component, sh::MESSAGE)
        .into_iter()
        .filter_map(|t| match t {
            Term::Literal(lit) => Some(lit.value().to_owned()),
            _ => None,
        })
        .collect();
    component_messages.sort();
    let message = component_messages.into_iter().next();
    let severity =
        first_object_of(data, component, sh::SEVERITY).and_then(|t| severity_from_term(&t));

    Ok(Component {
        id: NamedNode::from(component_iri),
        parameters,
        node_validators,
        property_validators,
        validators,
        message,
        severity,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::term::Literal;
    use crate::text_ingest::extract_prefixes;

    fn load_registry(ttl: &str, base_iri: &str) -> ComponentRegistry {
        let prefixes = extract_prefixes(ttl);
        let dataset: Arc<::purrdf::RdfDataset> =
            ::purrdf::parse_dataset(ttl.as_bytes(), "text/turtle", Some(base_iri))
                .expect("fixture parses");
        ComponentRegistry::parse(dataset.as_ref(), &prefixes).expect("registry parses")
    }

    #[test]
    fn sparql_local_name_extracts_suffix() {
        assert_eq!(
            sparql_local_name("http://example.org/ns#requiredParam"),
            "requiredParam"
        );
        assert_eq!(
            sparql_local_name("http://example.org/ns/requiredParam"),
            "requiredParam"
        );
        assert_eq!(sparql_local_name("ex:requiredParam"), "requiredParam");
        assert_eq!(sparql_local_name("requiredParam"), "requiredParam");
    }

    #[test]
    fn component_registry_construction_round_trips() {
        let mut registry = ComponentRegistry::default();
        let component = Component {
            id: NamedNode::from("http://example.org/ns#ExampleComponent"),
            parameters: vec![Parameter {
                path: NamedNode::from("http://example.org/ns#param"),
                name: "param".to_owned(),
                optional: false,
            }],
            node_validators: vec![Validator {
                kind: ValidatorKind::Ask,
                query_text: "ASK { ?this a ex:Thing }".to_owned(),
                message: None,
                severity: None,
            }],
            property_validators: vec![],
            validators: vec![],
            message: None,
            severity: None,
        };
        let id = component.id.as_str().to_owned();
        let component_iri = component.id.clone();
        let param_path = component.parameters[0].path.as_str().to_owned();
        registry.components.insert(id, component);
        registry.by_parameter_path.insert(param_path, component_iri);
        assert_eq!(registry.components.len(), 1);
        assert_eq!(registry.by_parameter_path.len(), 1);
    }

    #[test]
    fn parses_optional_001_component() {
        let ttl = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../vectors/shacl/sparql/component/optional-001.ttl"
        ))
        .expect("fixture exists");
        let registry = load_registry(
            &ttl,
            "http://datashapes.org/sh/tests/sparql/component/optional-001.test",
        );

        let component = registry
            .components
            .get("http://datashapes.org/sh/tests/sparql/component/optional-001.test#TestConstraintComponent")
            .expect("TestConstraintComponent present");
        assert_eq!(component.parameters.len(), 2);
        assert_eq!(component.parameters[0].name, "optionalParam");
        assert!(component.parameters[0].optional);
        assert_eq!(component.parameters[1].name, "requiredParam");
        assert!(!component.parameters[1].optional);
        let validator = component.validators.first().expect("validator present");
        assert!(matches!(validator.kind, ValidatorKind::Ask));
        assert!(validator.query_text.contains("PREFIX ex:"));
    }

    #[test]
    fn parses_property_validator_select_001_component() {
        let ttl = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../vectors/shacl/sparql/component/propertyValidator-select-001.ttl"
        ))
        .expect("fixture exists");
        let registry = load_registry(
            &ttl,
            "http://datashapes.org/sh/tests/sparql/component/propertyValidator-select-001.test",
        );

        let component = registry
            .components
            .get("http://datashapes.org/sh/tests/sparql/component/propertyValidator-select-001.test#LanguageConstraintComponentUsingSELECT")
            .expect("LanguageConstraintComponentUsingSELECT present");
        assert_eq!(component.parameters.len(), 1);
        assert_eq!(component.parameters[0].name, "lang");
        assert!(!component.parameters[0].optional);
        let validator = component
            .property_validators
            .first()
            .expect("propertyValidator present");
        assert!(matches!(validator.kind, ValidatorKind::Select));
        assert!(validator.query_text.contains("PREFIX ex:"));
    }

    #[test]
    fn parses_validator_001_component() {
        let ttl = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../vectors/shacl/sparql/component/validator-001.ttl"
        ))
        .expect("fixture exists");
        let registry = load_registry(
            &ttl,
            "http://datashapes.org/sh/tests/sparql/component/validator-001.test",
        );

        let component = registry
            .components
            .get("http://datashapes.org/sh/tests/sparql/component/validator-001.test#TestConstraintComponent")
            .expect("TestConstraintComponent present");
        assert_eq!(component.parameters.len(), 2);
        assert_eq!(component.parameters[0].name, "test1");
        assert_eq!(component.parameters[1].name, "test2");
        let validator = component.validators.first().expect("validator present");
        assert!(matches!(validator.kind, ValidatorKind::Ask));
        assert!(validator.query_text.contains("CONCAT"));
    }

    // ── Component validator evaluation (W3C fixtures) ──────────────────────────

    fn lit(s: &str) -> Term {
        Term::Literal(Literal::new_simple_literal(s))
    }

    fn validate_fixture(ttl: &str, base_iri: &str) -> crate::report::ValidationReport {
        let prefixes = extract_prefixes(ttl);
        let dataset: Arc<::purrdf::RdfDataset> =
            ::purrdf::parse_dataset(ttl.as_bytes(), "text/turtle", Some(base_iri))
                .expect("fixture parses");
        let shapes =
            crate::shapes::from_dataset_with_prefixes(&dataset, &prefixes).expect("shapes parse");
        crate::engine::validate_dataset(&dataset, &shapes).expect("validation evaluates")
    }

    #[test]
    fn repeated_single_parameter_components_conjoin_every_value() {
        let ttl = r#"
            @prefix ex: <http://example.org/> .
            @prefix sh: <http://www.w3.org/ns/shacl#> .
            ex:Required a sh:ConstraintComponent ;
                sh:parameter [ sh:path ex:required ] ;
                sh:validator [ a sh:SPARQLAskValidator ;
                    sh:ask "ASK { $this <http://example.org/p> $required }" ] .
            ex:Shape a sh:NodeShape ; sh:targetNode ex:focus ;
                ex:required ex:a, ex:b, ex:c .
            ex:focus ex:p ex:a .
        "#;
        let report = validate_fixture(ttl, "http://example.org/");
        assert!(!report.conforms);
        assert_eq!(report.results.len(), 2, "both absent requirements must run");
    }

    #[test]
    fn imported_core_component_does_not_reject_repeated_property_shapes() {
        use std::fmt::Write as _;
        let mut ttl = String::from(
            r"
            @prefix ex: <http://example.org/> .
            @prefix sh: <http://www.w3.org/ns/shacl#> .
            sh:PropertyConstraintComponent a sh:ConstraintComponent ;
                sh:parameter [ sh:path sh:property ] .
            ex:Shape a sh:NodeShape ; sh:targetNode ex:focus .
        ",
        );
        for index in 0..15 {
            writeln!(ttl, "ex:Shape sh:property ex:property{index} .\nex:property{index} sh:path ex:p{index} ; sh:minCount 1 .").unwrap();
        }
        let report = validate_fixture(&ttl, "http://example.org/");
        assert!(!report.conforms);
        assert_eq!(
            report.results.len(),
            15,
            "every native property constraint must run"
        );
    }

    #[test]
    fn multiple_parameter_components_still_refuse_multiple_values() {
        let ttl = r#"
            @prefix ex: <http://example.org/> .
            @prefix sh: <http://www.w3.org/ns/shacl#> .
            ex:Required a sh:ConstraintComponent ;
                sh:parameter [ sh:path ex:required ], [ sh:path ex:other ] ;
                sh:validator [ a sh:SPARQLAskValidator ; sh:ask "ASK { }" ] .
            ex:Shape a sh:NodeShape ; sh:targetNode ex:focus ;
                ex:required ex:a, ex:b ; ex:other ex:c .
        "#;
        let error = crate::engine::parse_shapes(ttl, None).unwrap_err();
        assert!(error.contains("only one is allowed"), "{error}");
    }

    #[test]
    fn eval_ask_validator_001() {
        let ttl = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../vectors/shacl/sparql/component/validator-001.ttl"
        ))
        .expect("fixture exists");
        let report = validate_fixture(
            &ttl,
            "http://datashapes.org/sh/tests/sparql/component/validator-001.test",
        );
        assert!(!report.conforms);
        assert_eq!(report.results.len(), 1, "exactly one non-conforming target");
        assert_eq!(report.results[0].focus_node, lit("Hallo Welt"));
        assert_eq!(report.results[0].value, Some(lit("Hallo Welt")));
        assert_eq!(
            report.results[0].source_constraint_component.as_str(),
            "http://datashapes.org/sh/tests/sparql/component/validator-001.test#TestConstraintComponent"
        );
    }

    #[test]
    fn eval_ask_validator_optional_001() {
        let ttl = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../vectors/shacl/sparql/component/optional-001.ttl"
        ))
        .expect("fixture exists");
        let report = validate_fixture(
            &ttl,
            "http://datashapes.org/sh/tests/sparql/component/optional-001.test",
        );
        assert!(!report.conforms);
        assert_eq!(report.results.len(), 4, "four violating focus/value pairs");
        let focus_values: Vec<(Option<String>, Option<String>)> = report
            .results
            .iter()
            .map(|r| (Some(r.focus_value()), r.value.as_ref().map(Term::to_string)))
            .collect();
        assert!(focus_values.contains(&(Some("One".to_owned()), Some("\"One\"".to_owned()))));
        assert!(focus_values.contains(&(Some("Three".to_owned()), Some("\"Three\"".to_owned()))));
        assert!(focus_values.contains(&(Some("Two".to_owned()), Some("\"Two\"".to_owned()))));
    }

    #[test]
    fn eval_select_property_validator_001() {
        let ttl = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../vectors/shacl/sparql/component/propertyValidator-select-001.ttl"
        ))
        .expect("fixture exists");
        let report = validate_fixture(
            &ttl,
            "http://datashapes.org/sh/tests/sparql/component/propertyValidator-select-001.test",
        );
        assert!(!report.conforms);
        assert_eq!(report.results.len(), 2, "two violating property values");
        let paths: Vec<&str> = report
            .results
            .iter()
            .map(|r| match r.result_path.as_ref().expect("path present") {
                Term::NamedNode(n) => n.as_str(),
                other => panic!("expected named-node path, got {other}"),
            })
            .collect();
        assert!(paths.contains(
            &"http://datashapes.org/sh/tests/sparql/component/propertyValidator-select-001.test#englishLabel"));
        assert!(paths.contains(
            &"http://datashapes.org/sh/tests/sparql/component/propertyValidator-select-001.test#germanLabel"));
        for r in &report.results {
            assert!(
                r.message
                    .as_ref()
                    .expect("message present")
                    .contains("Values are literals with language"),
                "message template should be substituted"
            );
        }
    }
}

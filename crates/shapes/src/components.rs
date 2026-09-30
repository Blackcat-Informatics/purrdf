// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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
use purrdf_sparql_eval::Prebinding;

use crate::data::{GraphFilter, native_quads, objects_of};
use crate::error::{IllFormedDeclaration, PrebindingViolation, RuleViolation as Violation};
use crate::model::{rdf, rdfs, sh, xsd};
use crate::path;
use crate::report::{Severity, ValidationResult};
use crate::shapes::prefixes::PrefixResolver;
use crate::shapes::{ComponentValidator, Path};
use crate::sparql::{run_ask_with_shacl_prebinding_view, run_select_with_shacl_prebinding_view};
use crate::term::{Literal, NamedNode, Term, term_value_to_native};
use crate::validator_alternatives::{AlternativeValidator, ValidatorLanguage};

/// Discriminator for a SPARQL validator's query form.
#[derive(Debug, Clone, Copy)]
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
    /// The `sh:message` values declared on the validator node.
    pub messages: Vec<Literal>,
    /// Optional severity declared on the validator node.
    pub severity: Option<Severity>,
    /// The result annotations declared on the validator node.
    pub annotations: Vec<crate::shapes::ResultAnnotation>,
    /// The validator node itself.
    pub node: Term,
    /// The attachment it is declared under (`sh:nodeValidator`, `sh:propertyValidator`
    /// or `sh:validator`).
    pub attachment: &'static str,
    /// The pre-binding restriction its query violates, when it violates one. A
    /// violation is not a syntax error of the declaration: SHACL 1.2 SPARQL Extensions,
    /// Appendix A requires a failure only for a query "executed with pre-bound
    /// variables", so it is refused where a shape selects the validator (see
    /// `Parser::parse_constraints`) and listed by `lint` otherwise.
    pub prebinding: Option<String>,
}

impl Validator {
    /// The declaration this validator is, as refusals and `lint` render it.
    pub(crate) fn declaration(&self, component: &str) -> String {
        custom_validator_declaration(&self.node, component, self.attachment)
    }
}

/// How a validator of the custom component `component` declared under `attachment` is
/// named in a refusal or a `lint` entry.
fn custom_validator_declaration(node: &Term, component: &str, attachment: &str) -> String {
    format!("validator {node} of the constraint component <{component}>, via <{attachment}>")
}

/// How a validator of the built-in component `component` declared under `attachment` is
/// named in a refusal or a `lint` entry.
fn builtin_validator_declaration(node: &Term, component: &str, attachment: &str) -> String {
    format!("validator {node} of the built-in component <{component}>, via <{attachment}>")
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
    /// The `sh:message` values declared on the component node.
    pub messages: Vec<Literal>,
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
    /// Every validator declared for a built-in component — an alternative its native
    /// implementation supersedes — sorted. See [`crate::validator_alternatives`].
    pub alternatives: Vec<AlternativeValidator>,
    /// Every syntax-rule violation of a component declaration, of its parameters or of
    /// its validators, built-in and custom alike. Collected rather than raised one at a
    /// time, so the refusal names every violation in the graph (see
    /// [`crate::error::IllFormedShapesGraph`]).
    pub ill_formed: Vec<IllFormedDeclaration>,
    /// The pre-binding violations of the validators declared for built-in components.
    /// The native implementation supersedes every such validator, so none of them ever
    /// executes and none refuses anything: `lint` lists them as unexecuted.
    pub alternative_prebinding: Vec<PrebindingViolation>,
}

impl ComponentRegistry {
    /// Parse all `sh:ConstraintComponent` (and subclass) declarations from the
    /// shapes graph into a registry.
    ///
    /// Discovery walks every `rdf:type` triple and keeps the subject when its
    /// class is `sh:ConstraintComponent` or a subclass thereof. Each discovered
    /// component's `sh:parameter` declarations and `sh:nodeValidator` /
    /// `sh:propertyValidator` / `sh:validator` SPARQL validators are parsed and
    /// checked against the syntax rules of SHACL 1.2 SPARQL Extensions. Every violation,
    /// on a built-in or a custom component, used by a shape or not, is collected into
    /// [`Self::ill_formed`]; the caller refuses the shapes graph with all of them (see
    /// [`Self::parse`]). A validator query that violates a pre-binding restriction is
    /// NOT ill-formed: the violation is recorded on the validator
    /// ([`Validator::prebinding`]) or, for a built-in's, in
    /// [`Self::alternative_prebinding`], and judged where the query would execute.
    ///
    /// # Errors
    ///
    /// Returns `Err(String)` for a refusal that is not a syntax-rule violation of a
    /// declaration: a component declaration naming a built-in function, a declaration
    /// of a built-in component that contradicts its signature, a malformed
    /// `sh:message`.
    ///
    /// A value of an attachment that is not a SPARQL validator of the attachment's query
    /// form — a SHACL-JS `sh:JSValidator` included — is a syntax-rule violation like any
    /// other, collected into [`Self::ill_formed`]. So is one declared by a subject the
    /// registry does not read as a component: the class rules name the attachment, not
    /// its subject.
    pub(crate) fn parse_collecting(
        data: &RdfDataset,
        prefixes: &PrefixResolver,
    ) -> Result<Self, String> {
        let rdf_type = Term::NamedNode(NamedNode::from(rdf::TYPE));
        let mut component_iris: Vec<String> = Vec::new();
        let mut seen: FastSet<String> = FastSet::default();
        let mut subclass_memo: FastMap<(String, String), bool> = FastMap::default();
        let mut builtin_alternatives: Vec<AlternativeValidator> = Vec::new();
        let mut ill_formed: Vec<IllFormedDeclaration> = Vec::new();
        let mut alternative_prebinding: Vec<PrebindingViolation> = Vec::new();

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
            if !seen.insert(component.as_str().to_owned()) {
                continue;
            }
            // The linker (`crate::spec`): a declaration of a component the spec
            // symbol table knows is a SIGNATURE, bound against the table rather than
            // registered as a custom component. A native component's declaration
            // binds and registers nothing; the validators it declares are
            // alternative implementations the native one supersedes, checked for
            // well-formedness, recorded, and never run; a body, a query or a
            // semantic `sh:` statement on it is refused; a declaration of a
            // built-in FUNCTION IRI is a kind mismatch. Every spec component is
            // evaluated natively, so none is registered as a custom component.
            crate::spec::refuse_component_declared_function(component.as_str())?;
            if let Some(row) = crate::spec::component(component.as_str()) {
                crate::spec::bind_spec_component(data, row)?;
                let component_term = Term::NamedNode(component.clone());
                // An alternative is never run, but it must still be a well-formed
                // validator: "The value of sh:ask must be a valid SPARQL ASK query"
                // (likewise sh:select), under the parameter names the built-in's
                // signature pre-binds. The syntax rules hold whether or not anything
                // runs the validator, so a violation is collected like a custom
                // component's. A pre-binding violation is not a syntax violation, and
                // an alternative never executes, so it is recorded for `lint` and
                // refuses nothing. The parse resolves no function, so a query calling
                // a function this engine does not have is well-formed and loads.
                let param_names: Vec<String> = row
                    .params
                    .iter()
                    .map(|param| sparql_local_name(param.path))
                    .collect();
                for (attachment, validator, kind) in
                    declared_validators(data, &component_term, &mut subclass_memo)
                {
                    let declaration =
                        builtin_validator_declaration(&validator, component.as_str(), attachment);
                    let language = match kind {
                        Ok(kind) => {
                            match parse_validator(
                                data,
                                prefixes,
                                &component_term,
                                &validator,
                                attachment,
                                &param_names,
                                kind,
                            ) {
                                Ok(parsed) => {
                                    if let Some(message) = parsed.prebinding {
                                        alternative_prebinding.push(PrebindingViolation::new(
                                            format!(
                                                "{declaration}, which never executes: the \
                                                 native implementation supersedes it"
                                            ),
                                            message,
                                        ));
                                    }
                                }
                                Err((rule, message)) => ill_formed.push(IllFormedDeclaration::new(
                                    declaration,
                                    rule,
                                    message,
                                )),
                            }
                            match kind {
                                ValidatorKind::Ask => ValidatorLanguage::SparqlAsk,
                                ValidatorKind::Select => ValidatorLanguage::SparqlSelect,
                            }
                        }
                        Err((rule, message)) => {
                            ill_formed.push(IllFormedDeclaration::new(declaration, rule, message));
                            continue;
                        }
                    };
                    builtin_alternatives.push(AlternativeValidator {
                        component: component.as_str().to_owned(),
                        attachment: attachment.to_owned(),
                        validator,
                        language,
                    });
                }
                continue;
            }
            component_iris.push(component.as_str().to_owned());
        }
        component_iris.sort();

        let mut registry = Self {
            alternatives: builtin_alternatives,
            alternative_prebinding,
            ..Self::default()
        };
        for component_iri in component_iris {
            let component_term = Term::NamedNode(NamedNode::from(component_iri.as_str()));
            let component = parse_component(
                data,
                prefixes,
                &component_term,
                &component_iri,
                &mut subclass_memo,
                &mut ill_formed,
            )?;
            for param in &component.parameters {
                registry
                    .by_parameter_path
                    .insert(param.path.as_str().to_owned(), component.id.clone());
            }
            let id = component.id.as_str().to_owned();
            registry.components.insert(id, component);
        }
        // The class rules constrain every value of an attachment, whatever declares it:
        // a subject read above as a component was judged there, and every other subject
        // of `sh:validator`, `sh:nodeValidator` or `sh:propertyValidator` is judged here,
        // so no ill-formed validator loads because its owner is not a component.
        let mut owners: Vec<Term> = Vec::new();
        for attachment in [sh::NODE_VALIDATOR, sh::PROPERTY_VALIDATOR, sh::VALIDATOR] {
            let predicate = Term::NamedNode(NamedNode::from(attachment));
            for (subject, _, _) in
                native_quads(data, None, Some(&predicate), None, GraphFilter::AnyGraph)
            {
                if !matches!(&subject, Term::NamedNode(iri) if seen.contains(iri.as_str())) {
                    owners.push(subject);
                }
            }
        }
        crate::term::sort_terms_canonical(&mut owners);
        owners.dedup();
        for owner in owners {
            for attachment in [sh::NODE_VALIDATOR, sh::PROPERTY_VALIDATOR, sh::VALIDATOR] {
                let mut nodes = objects_of(data, &owner, attachment);
                crate::term::sort_terms_canonical(&mut nodes);
                for node in nodes {
                    if let Err((rule, message)) = attachment_class(
                        data,
                        &owner.to_string(),
                        attachment,
                        &node,
                        &mut subclass_memo,
                    ) {
                        ill_formed.push(IllFormedDeclaration::new(
                            format!("validator {node} of {owner}, via <{attachment}>"),
                            rule,
                            message,
                        ));
                    }
                }
            }
        }
        registry.alternatives.sort();
        registry.alternative_prebinding.sort();
        registry.alternative_prebinding.dedup();
        registry.ill_formed = ill_formed;
        Ok(registry)
    }

    /// [`Self::parse_collecting`], refusing the registry when any declaration in it
    /// violates a syntax rule — for a caller that reads the registry alone. A parse of
    /// the whole shapes graph instead joins [`Self::ill_formed`] with the violations of
    /// the rest of its declarations before refusing (see `Parser::parse_with_expressions`).
    ///
    /// # Errors
    ///
    /// [`Self::parse_collecting`]'s own, and every syntax-rule violation, as the
    /// rendered [`crate::error::IllFormedShapesGraph`].
    pub(crate) fn parse(data: &RdfDataset, prefixes: &PrefixResolver) -> Result<Self, String> {
        let registry = Self::parse_collecting(data, prefixes)?;
        match crate::error::IllFormedShapesGraph::from_violations(registry.ill_formed.clone()) {
            Some(refusal) => Err(refusal.to_string()),
            None => Ok(registry),
        }
    }
}

/// Map an `sh:severity` object term to a [`Severity`]: the five built-in
/// `sh:` severities map to their variants, any OTHER IRI is preserved verbatim
/// (SHACL allows custom severity IRIs), and a non-IRI object yields `None`.
pub(crate) fn severity_from_term(t: &Term) -> Option<Severity> {
    match t {
        Term::NamedNode(n) => Some(Severity::from_iri_open(n.as_str())),
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

/// Render every message of a result against `bindings` (SHACL-SPARQL §5.3.3),
/// each keeping its datatype, language tag and base direction, in canonical order.
pub(crate) fn render_message_templates(
    messages: &[Literal],
    bindings: &[(String, Term)],
) -> Vec<Literal> {
    crate::report::canonical_messages(
        messages
            .iter()
            .map(|message| {
                message.with_value(substitute_message_templates(message.value(), bindings))
            })
            .collect(),
    )
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
    focus_id: Option<D::Id>,
    value_nodes: &[Term],
    validator: &ComponentValidator,
    bindings: &[(String, Term)],
    component: &NamedNode,
    source_shape: &Term,
    path: Option<&Path>,
    severity: &Severity,
    messages: &[Literal],
    annotations: &[crate::shapes::ResultAnnotation],
    shapes_graph_iri: Option<&str>,
    current_shape: Option<&Term>,
) -> Result<Vec<ValidationResult>, String> {
    let ComponentValidator::Ask { ask } = validator else {
        return Err("expected ASK validator, got SELECT".to_owned());
    };
    let mut results = Vec::with_capacity(value_nodes.len());
    // Every pre-binding but `$value` is a constant of this component invocation: the
    // focus node, the parameter bindings, and the shape context. So is every NAME,
    // `$value`'s included — the names are shape text, not data — and so is the ASK
    // TEXT. The name list is therefore built once here and the value-node loop below
    // writes the terms straight into a prepared handle's slots, instead of re-hashing
    // the whole ASK text to probe the plan cache and re-interning every name per
    // value node.
    //
    // This worker's CACHED handle, not a local one, even though the loop below is
    // serial: this function is called once per focus node and loops over that focus
    // node's value nodes, so a local handle would pay a fresh preparation per focus
    // node to save a probe on the two or three runs inside it — a preparation the
    // cache amortizes over the whole focus set instead.
    //
    // # The repeated-name fallback, and why it is not optional
    //
    // A component declares its own `sh:parameter`s. `this`, `path`, `PATH` and
    // `value` are refused as parameter names at shapes LOAD, so those four never
    // arrive here — but `shapesGraph` and `currentShape` are not on that list, and
    // SHACL pre-binds both around every validator. A component declaring a parameter
    // whose local name is either presents this function with that name TWICE, which a
    // handle cannot express: it has no single slot to bind, and `prepare_execution`
    // refuses it. The `&str` door SUPPORTS that case with a defined answer (two seeds
    // binding one variable to different terms are incompatible, so it keeps a
    // per-variable path and yields the empty solution). So a repeated name falls back
    // to the `&str` door UNCHANGED rather than becoming an error: refusing a query
    // that has an answer would be the mirror of silently dropping one.
    //
    // The fallback's own pre-binding list is built only ON that path. Building both
    // eagerly would materialize every parameter's `TermValue` twice per focus node,
    // which costs more than the handle saves.
    const VALUE_SLOT: usize = 1;
    let mut names: Vec<&str> = Vec::with_capacity(4 + bindings.len());
    names.push("this");
    names.push("value");
    names.extend(bindings.iter().map(|(name, _)| name.as_str()));
    crate::sparql::push_shape_context_names(&mut names, shapes_graph_iri, current_shape);
    let prepared = crate::sparql::parameters_are_distinct(&names);
    let mut subs: Vec<Prebinding<'_>> = if prepared {
        Vec::new()
    } else {
        let mut subs = Vec::with_capacity(4 + bindings.len());
        subs.push(Prebinding {
            variable: "this",
            value: focus.to_term_value(),
        });
        subs.push(Prebinding {
            variable: "value",
            value: TermValue::Iri(String::new()),
        });
        for (name, value) in bindings {
            subs.push(Prebinding {
                variable: name.as_str(),
                value: value.to_term_value(),
            });
        }
        crate::sparql::push_shape_context(&mut subs, shapes_graph_iri, current_shape);
        subs
    };
    // The violating branch's template buffer, hoisted for the same reason: its
    // parameter half does not vary, so it is filled once and only the trailing
    // `value` entry is replaced.
    let mut template_bindings: Vec<(String, Term)> = if !messages.is_empty() {
        let mut buffer = Vec::with_capacity(bindings.len() + 1);
        buffer.extend_from_slice(bindings);
        buffer.push(("value".to_owned(), focus.clone()));
        buffer
    } else {
        Vec::new()
    };
    // One reporting step, called from whichever door computed the verdict, so the two
    // doors cannot grow two different notions of what a violation looks like.
    let mut report = |v: &Term, results: &mut Vec<ValidationResult>| {
        let messages = if messages.is_empty() {
            Vec::new()
        } else {
            let slot = template_bindings
                .last_mut()
                .expect("the template buffer is non-empty whenever a message is present");
            slot.1 = v.clone();
            render_message_templates(messages, &template_bindings)
        };
        // SHACL 1.2 SPARQL Extensions, "Validation with SPARQL-based Constraint
        // Components": an ASK validator's solution for a failing value node "consists
        // of the bindings ($this, focus node) and ($value, v)", and the result is
        // produced from it as from a SELECT solution — so its annotations read
        // `this`, `value` and, pre-bound beside them, the component's parameters.
        let annotations = crate::result_annotations::annotate(annotations, |name| match name {
            "this" => Some(focus.clone()),
            "value" => Some(v.clone()),
            _ => bindings
                .iter()
                .find(|(parameter, _)| parameter == name)
                .map(|(_, value)| value.clone()),
        });
        results.push(ValidationResult {
            focus_node: focus.clone(),
            result_path: path.map(path::path_to_term),
            path_structure: path.filter(|p| !matches!(p, Path::Predicate(_))).cloned(),
            value: Some(v.clone()),
            source_constraint_component: component.clone(),
            source_shape: source_shape.clone(),
            severity: severity.clone(),
            messages,
            source_box_roles: vec![],
            path_box_roles: vec![],
            result_box_roles: vec![],
            attributions: vec![],
            details: vec![],
            annotations,
        });
    };

    // The prepared handle is checked out ONCE for the whole value-node set: `$this`,
    // the declared parameters and the shape context are constants of this invocation,
    // so they are bound once and only `$value` is rewritten per run. Re-binding them
    // per value node would re-materialize every one of their `TermValue`s — the exact
    // per-value-node cost the hoisted `subs` list was introduced to remove, and which
    // a bind-per-run door would have reintroduced.
    if prepared {
        crate::sparql::with_cached_execution(
            ask,
            &names,
            purrdf_sparql_eval::ShaclPrebinding::Applied,
            |execution| {
                // Every slot but `$value`, once. `with_cached_execution` cleared them all
                // at checkout, so any slot this forgets is `None` and the engine refuses
                // the run rather than answering with whatever a previous focus node left
                // there.
                crate::sparql::bind_focus(execution, 0, dataset, focus, focus_id)?;
                let mut slot = VALUE_SLOT + 1;
                for (_, value) in bindings {
                    execution.bind(slot, value.to_term_value())?;
                    slot += 1;
                }
                crate::sparql::bind_shape_context(
                    execution,
                    slot,
                    shapes_graph_iri,
                    current_shape,
                )?;
                for v in value_nodes {
                    // The one varying slot, written on every pass.
                    execution.bind(VALUE_SLOT, v.to_term_value())?;
                    if !crate::sparql::run_bound_ask_with_shacl_prebinding_view(dataset, execution)?
                    {
                        report(v, &mut results);
                    }
                }
                Ok(())
            },
        )?;
    } else {
        for v in value_nodes {
            subs[VALUE_SLOT].value = v.to_term_value();
            if !run_ask_with_shacl_prebinding_view(dataset, ask, &subs)? {
                report(v, &mut results);
            }
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
    focus_id: Option<D::Id>,
    validator: &ComponentValidator,
    bindings: &[(String, Term)],
    component: &NamedNode,
    source_shape: &Term,
    path: Option<&Path>,
    severity: &Severity,
    messages: &[Literal],
    annotations: &[crate::shapes::ResultAnnotation],
    shapes_graph_iri: Option<&str>,
    current_shape: Option<&Term>,
) -> Result<Vec<ValidationResult>, String> {
    let ComponentValidator::Select { select } = validator else {
        return Err("expected SELECT validator, got ASK".to_owned());
    };
    // The parameter NAMES for the prepared door: `$this`, each declared parameter,
    // then the shape context, in the order the values are bound below. A component
    // may declare a parameter whose local name is `currentShape` or `shapesGraph`,
    // which a handle cannot express — see `eval_ask_validator` for why that falls back
    // to the `&str` door unchanged rather than becoming an error.
    let mut names: Vec<&str> = Vec::with_capacity(3 + bindings.len());
    names.push("this");
    names.extend(bindings.iter().map(|(name, _)| name.as_str()));
    crate::sparql::push_shape_context_names(&mut names, shapes_graph_iri, current_shape);
    let query = crate::constraints::substitute_path_placeholder(select, path);

    let path_term = path.map(path::path_to_term);
    let path_structure = path.filter(|p| !matches!(p, Path::Predicate(_))).cloned();

    let project = |solutions: &purrdf_sparql_eval::InternedSolutions<'_, '_, D>| {
        let this_index = solutions.column("this");
        let path_index = solutions.column("path");
        let value_index = solutions.column("value");
        let annotation_columns =
            crate::sparql::annotation_columns(annotations, |name| solutions.column(name));

        let mut results = Vec::with_capacity(solutions.len());
        // §5.3.3 message templating is the ONLY reader of the row's other columns,
        // so the buffer is allocated — and the columns are converted — only when
        // there is a message to render. A validator with none reads exactly the
        // three columns it maps onto the result.
        let mut template_bindings: Vec<(String, Term)> = if !messages.is_empty() {
            Vec::with_capacity(solutions.variables().len() + bindings.len())
        } else {
            Vec::new()
        };
        for row in solutions.rows() {
            let focus_node = this_index
                .and_then(|i| solutions.cell(row, i))
                .as_ref()
                .map_or_else(|| focus.clone(), term_value_to_native);

            let (result_path, result_path_structure) =
                match path_index.and_then(|i| solutions.cell(row, i)) {
                    Some(value) => (Some(term_value_to_native(&value)), None),
                    None => (path_term.clone(), path_structure.clone()),
                };

            // SHACL 1.2 SPARQL Extensions, "Validation with SPARQL-based Constraint
            // Components": "The production rules for the validation results are
            // identical to those for SPARQL-based constraints", whose `sh:value` is
            // "The binding for the variable value", else "The value node". A node
            // shape's one value node is its focus node.
            let value = value_index
                .and_then(|i| solutions.cell(row, i))
                .as_ref()
                .map(term_value_to_native)
                .or_else(|| path.is_none().then(|| focus.clone()));

            let messages = if messages.is_empty() {
                Vec::new()
            } else {
                // The row's own bindings first, so a projected variable outranks a
                // parameter of the same name — the precedence this has always had.
                template_bindings.clear();
                for (index, var) in solutions.variables().iter().enumerate() {
                    if let Some(value) = solutions.cell(row, index) {
                        template_bindings
                            .push((var.as_str().to_owned(), term_value_to_native(&value)));
                    }
                }
                for (name, value) in bindings {
                    if !template_bindings.iter().any(|(n, _)| n == name) {
                        template_bindings.push((name.clone(), value.clone()));
                    }
                }
                render_message_templates(messages, &template_bindings)
            };
            // SHACL 1.2 SPARQL Extensions, "Annotation Properties": the solution's
            // binding of each annotation's variable — the pre-bound `$this` and
            // parameters included, as for message templates — or else its defaults.
            let annotations = crate::result_annotations::annotate(annotations, |name| {
                annotation_columns
                    .iter()
                    .find(|(variable, _)| *variable == name)
                    .and_then(|(_, column)| column.and_then(|i| solutions.cell(row, i)))
                    .as_ref()
                    .map(term_value_to_native)
                    .or_else(|| (name == "this").then(|| focus.clone()))
                    .or_else(|| {
                        bindings
                            .iter()
                            .find(|(parameter, _)| parameter == name)
                            .map(|(_, value)| value.clone())
                    })
            });

            results.push(ValidationResult {
                focus_node,
                result_path,
                path_structure: result_path_structure,
                value,
                source_constraint_component: component.clone(),
                source_shape: source_shape.clone(),
                severity: severity.clone(),
                messages,
                source_box_roles: vec![],
                path_box_roles: vec![],
                result_box_roles: vec![],
                attributions: vec![],
                details: vec![],
                annotations,
            });
        }
        Ok(results)
    };

    // One run per FOCUS NODE, from a loop `rayon` fans across workers, so no single
    // handle can span the focus set: this uses the per-worker cached one. A repeated
    // parameter name falls back to the `&str` door unchanged — see
    // `eval_ask_validator`.
    if crate::sparql::parameters_are_distinct(&names) {
        return crate::sparql::run_cached_select_with_shacl_prebinding_view(
            dataset,
            &query,
            &names,
            |execution| {
                crate::sparql::bind_focus(execution, 0, dataset, focus, focus_id)?;
                let mut slot = 1;
                for (_, value) in bindings {
                    execution.bind(slot, value.to_term_value())?;
                    slot += 1;
                }
                crate::sparql::bind_shape_context(
                    execution,
                    slot,
                    shapes_graph_iri,
                    current_shape,
                )?;
                Ok(())
            },
            project,
        );
    }
    let mut subs: Vec<Prebinding<'_>> = Vec::with_capacity(3 + bindings.len());
    subs.push(Prebinding {
        variable: "this",
        value: focus.to_term_value(),
    });
    for (name, value) in bindings {
        subs.push(Prebinding {
            variable: name.as_str(),
            value: value.to_term_value(),
        });
    }
    crate::sparql::push_shape_context(&mut subs, shapes_graph_iri, current_shape);
    run_select_with_shacl_prebinding_view(dataset, &query, &subs, project)
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

/// The query form `validator`'s `rdf:type` declarations make it, respecting subclasses
/// of `sh:SPARQLAskValidator` and `sh:SPARQLSelectValidator`.
///
/// # Errors
///
/// A validator typed as both ASK and SELECT, and one typed as neither — a SHACL-JS
/// `sh:JSValidator` among them, which is named as such: SHACL 1.2 SPARQL Extensions
/// admits only ASK-based validators as values of `sh:validator` and SELECT-based ones as
/// values of `sh:nodeValidator` and `sh:propertyValidator`.
fn validator_kind(
    data: &RdfDataset,
    validator: &Term,
    memo: &mut FastMap<(String, String), bool>,
) -> Result<ValidatorKind, String> {
    let mut is_ask = false;
    let mut is_select = false;
    let mut is_js = false;
    for q in objects_of(data, validator, rdf::TYPE) {
        let Term::NamedNode(class) = q else {
            continue;
        };
        is_ask |= is_subclass_of(data, class.as_str(), sh::SPARQL_ASK_VALIDATOR, memo);
        is_select |= is_subclass_of(data, class.as_str(), sh::SPARQL_SELECT_VALIDATOR, memo);
        is_js |= is_subclass_of(data, class.as_str(), sh::JS_VALIDATOR, memo);
    }
    match (is_ask, is_select) {
        (true, true) => Err(format!(
            "validator {validator} is typed as both ASK and SELECT"
        )),
        (true, false) => Ok(ValidatorKind::Ask),
        (false, true) => Ok(ValidatorKind::Select),
        (false, false) if is_js => Err(format!(
            "validator {validator} is a sh:JSValidator of the SHACL JavaScript Extensions, \
             which is not a SPARQL validator: it must be typed as sh:SPARQLAskValidator or \
             sh:SPARQLSelectValidator (or a subclass)"
        )),
        (false, false) => Err(format!(
            "validator {validator} must be typed as sh:SPARQLAskValidator or \
             sh:SPARQLSelectValidator (or a subclass)"
        )),
    }
}

/// The syntax rule that makes the values of `attachment` validators of one form:
/// `nodeValidator-class`, `propertyValidator-class` or `validator-class` (SHACL 1.2
/// SPARQL Extensions, "Summary of Syntax Rules").
fn class_rule(attachment: &str) -> &'static str {
    match attachment {
        sh::NODE_VALIDATOR => "nodeValidator-class",
        sh::PROPERTY_VALIDATOR => "propertyValidator-class",
        _ => "validator-class",
    }
}

/// Every validator `component` declares, as `(attachment, validator, kind)` in
/// attachment order and canonical term order within one.
///
/// A kind is a [`Violation`] of the attachment's class rule for a validator
/// [`validator_kind`] refuses, and for a SPARQL validator whose query form is not the
/// one its attachment takes: "The values of sh:nodeValidator must be SELECT-based
/// validators", "The values of sh:propertyValidator must be SELECT-based validators",
/// "The values of sh:validator must be ASK-based validators" (SHACL 1.2 SPARQL
/// Extensions). A SHACL-JS `sh:JSValidator` is judged by the same rules: it is neither
/// ASK- nor SELECT-based, so it is a violation under every attachment.
fn declared_validators(
    data: &RdfDataset,
    component: &Term,
    memo: &mut FastMap<(String, String), bool>,
) -> Vec<(&'static str, Term, Result<ValidatorKind, Violation>)> {
    let mut out = Vec::new();
    for attachment in [sh::NODE_VALIDATOR, sh::PROPERTY_VALIDATOR, sh::VALIDATOR] {
        let mut nodes = objects_of(data, component, attachment);
        crate::term::sort_terms_canonical(&mut nodes);
        for node in nodes {
            let kind = attachment_class(
                data,
                &format!("component {component}"),
                attachment,
                &node,
                memo,
            );
            out.push((attachment, node, kind));
        }
    }
    out
}

/// `node`'s query form, judged as a value of `attachment` declared by `owner` (rendered
/// for the message): the [`Violation`] of the attachment's class rule when
/// [`validator_kind`] refuses it or its form is not the one the attachment takes.
fn attachment_class(
    data: &RdfDataset,
    owner: &str,
    attachment: &'static str,
    node: &Term,
    memo: &mut FastMap<(String, String), bool>,
) -> Result<ValidatorKind, Violation> {
    let expects_ask = attachment == sh::VALIDATOR;
    let form = validator_kind(data, node, memo).map_err(|e| {
        (
            Some(class_rule(attachment)),
            format!("{owner} validator {node}: {e}"),
        )
    })?;
    if matches!(form, ValidatorKind::Ask) == expects_ask {
        return Ok(form);
    }
    Err((
        Some(class_rule(attachment)),
        format!(
            "{owner} {attachment} requires {} validators, and {node} is {form:?}",
            if expects_ask { "ASK" } else { "SELECT" },
        ),
    ))
}

/// Whether `name` is a SPARQL `VARNAME` and not one of the reserved names banned
/// for SHACL-SPARQL parameter bindings.
///
/// `VARNAME` is the grammar's production, Unicode included — `ex:größe` binds
/// `?größe` and `ex:2d` binds `?2d` — so the answer is the SPARQL lexer's own.
fn is_valid_varname(name: &str) -> bool {
    !is_reserved_parameter_name(name) && purrdf_sparql_algebra::lexer::is_varname(name)
}

/// Whether `name` is one SHACL 1.2 SPARQL Extensions reserves: "Parameter names must not
/// be one of the following: this, path, PATH, value." (`parameter-name-not-in`).
fn is_reserved_parameter_name(name: &str) -> bool {
    const RESERVED: &[&str] = &["this", "path", "PATH", "value"];
    RESERVED.contains(&name)
}

/// Parse a single SPARQL validator node, of query form `kind`, declared under
/// `attachment` of a component.
///
/// # Errors
///
/// The syntax-rule [`Violation`] of the declaration. A pre-binding violation is not
/// one: it is returned on the parsed validator ([`Validator::prebinding`]).
fn parse_validator(
    data: &RdfDataset,
    prefixes: &PrefixResolver,
    component: &Term,
    validator: &Term,
    attachment: &'static str,
    param_names: &[String],
    kind: ValidatorKind,
) -> Result<Validator, Violation> {
    let component_iri = match component {
        Term::NamedNode(n) => n.as_str(),
        _ => return Err((None, format!("component {component} is not a named node"))),
    };
    let (query_rule, count_rule, datatype_rule) = match kind {
        ValidatorKind::Ask => ("ask-sparql", "ask-count", "ask-datatype"),
        ValidatorKind::Select => (
            "select-query-valid",
            "SPARQLSelectValidator-select-count",
            "select-query-valid",
        ),
    };

    let query_pred = match kind {
        ValidatorKind::Ask => sh::ASK,
        ValidatorKind::Select => sh::SELECT,
    };
    // A validator typed for one query form and carrying the other's query would
    // have that query silently ignored.
    let other_pred = match kind {
        ValidatorKind::Ask => sh::SELECT,
        ValidatorKind::Select => sh::ASK,
    };
    if !objects_of(data, validator, other_pred).is_empty() {
        // No numbered rule forbids the second query; it is refused because a
        // validator of this form would silently never run it.
        return Err((
            None,
            format!(
                "component {component_iri} validator {validator} is declared as {kind:?} but \
                 also carries <{other_pred}>, which a {kind:?} validator never runs"
            ),
        ));
    }
    let raw_queries = objects_of(data, validator, query_pred);
    let raw_query = match raw_queries.as_slice() {
        [Term::Literal(literal)] if literal.datatype_str() == xsd::STRING => literal.value(),
        values => {
            return Err((
                Some(if values.len() == 1 {
                    datatype_rule
                } else {
                    count_rule
                }),
                format!(
                    "component {component_iri} validator {validator} must have exactly one {} \
                     xsd:string literal",
                    match kind {
                        ValidatorKind::Ask => "sh:ask",
                        ValidatorKind::Select => "sh:select",
                    }
                ),
            ));
        }
    };
    let header = prefixes
        .header(data, &[component, validator])
        .map_err(crate::shapes::prefixes::split_syntax_rule)?;
    let query_text = format!("{header}{raw_query}");

    let query = match purrdf_sparql_algebra::SparqlParser::new().parse_query(&query_text) {
        Ok(q) => q,
        Err(e) => {
            return Err((
                Some(query_rule),
                format!(
                    "component {component_iri} validator {validator} has an unparsable query: {e}"
                ),
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
        return Err((
            Some(query_rule),
            format!(
                "component {component_iri} validator {validator} is declared as {kind:?} but \
                 the query text parses to a {got_form} query"
            ),
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
    let prebinding = prebinding_result.err().map(|e| {
        format!(
            "component {component_iri} validator {validator} violates pre-binding restrictions: \
             {e}"
        )
    });

    let messages = declared_messages(data, validator).map_err(|message| (None, message))?;
    let severity =
        first_object_of(data, validator, sh::SEVERITY).and_then(|t| severity_from_term(&t));
    // SHACL 1.2 SPARQL Extensions: result annotations are declared "at the subject
    // of the sh:select or sh:ask triple", which is this validator node.
    let annotations = crate::result_annotations::parse_with_rule(data, validator)?;

    Ok(Validator {
        kind,
        query_text,
        messages,
        severity,
        annotations,
        node: validator.clone(),
        attachment,
        prebinding,
    })
}

/// Parse a single constraint component node and its parameters / validators.
fn parse_component(
    data: &RdfDataset,
    prefixes: &PrefixResolver,
    component: &Term,
    component_iri: &str,
    subclass_memo: &mut FastMap<(String, String), bool>,
    ill_formed: &mut Vec<IllFormedDeclaration>,
) -> Result<Component, String> {
    let component_declaration = format!("the constraint component <{component_iri}>");
    let param_nodes: Vec<Term> = objects_of(data, component, sh::PARAMETER_PROPERTY);
    let mut parameters = Vec::with_capacity(param_nodes.len());
    let mut parameters_ill_formed = false;
    for param_node in param_nodes {
        match parse_parameter(data, &param_node, component_iri) {
            Ok(parameter) => parameters.push(parameter),
            Err((rule, message)) => {
                parameters_ill_formed = true;
                ill_formed.push(IllFormedDeclaration::new(
                    format!("parameter {param_node} of the constraint component <{component_iri}>"),
                    Some(rule),
                    message,
                ));
            }
        }
    }
    parameters.sort_by(|a, b| a.path.as_str().cmp(b.path.as_str()));
    // A parameter that did not parse may be the non-optional one, so the rule is judged
    // only over a complete parameter list.
    if !parameters_ill_formed && parameters.iter().all(|parameter| parameter.optional) {
        ill_formed.push(IllFormedDeclaration::new(
            component_declaration.clone(),
            Some("ConstraintComponent-parameter"),
            format!("component {component_iri} must declare at least one non-optional parameter"),
        ));
    }
    let mut names = FastSet::default();
    for parameter in &parameters {
        if !names.insert(parameter.name.as_str()) {
            ill_formed.push(IllFormedDeclaration::new(
                component_declaration.clone(),
                Some("parameter-name-unique"),
                format!(
                    "component {component_iri} declares duplicate parameter name ?{}",
                    parameter.name,
                ),
            ));
        }
    }
    let param_names: Vec<String> = parameters.iter().map(|p| p.name.clone()).collect();

    let mut node_validators = Vec::new();
    let mut property_validators = Vec::new();
    let mut validators = Vec::new();
    for (attachment, node, kind) in declared_validators(data, component, subclass_memo) {
        let declaration = custom_validator_declaration(&node, component_iri, attachment);
        let kind = match kind {
            Ok(kind) => kind,
            Err((rule, message)) => {
                ill_formed.push(IllFormedDeclaration::new(declaration, rule, message));
                continue;
            }
        };
        match parse_validator(
            data,
            prefixes,
            component,
            &node,
            attachment,
            &param_names,
            kind,
        ) {
            Ok(parsed) => match attachment {
                sh::NODE_VALIDATOR => node_validators.push(parsed),
                sh::PROPERTY_VALIDATOR => property_validators.push(parsed),
                _ => validators.push(parsed),
            },
            Err((rule, message)) => {
                ill_formed.push(IllFormedDeclaration::new(declaration, rule, message));
            }
        }
    }

    let messages = declared_messages(data, component)?;
    let severity =
        first_object_of(data, component, sh::SEVERITY).and_then(|t| severity_from_term(&t));

    Ok(Component {
        id: NamedNode::from(component_iri),
        parameters,
        node_validators,
        property_validators,
        validators,
        messages,
        severity,
    })
}

/// Every `sh:message` of a validator or component node, as literals in canonical
/// order. A value that is not an `xsd:string`, `rdf:langString`,
/// `rdf:dirLangString` or `rdf:HTML` literal is refused rather than skipped.
fn declared_messages(data: &RdfDataset, node: &Term) -> Result<Vec<Literal>, String> {
    let mut messages = Vec::new();
    for value in objects_of(data, node, sh::MESSAGE) {
        match &value {
            Term::Literal(lit) if crate::shapes::parser_is_text_literal(&value) => {
                messages.push(lit.clone());
            }
            other => {
                return Err(format!(
                    "sh:message on {node} must be an xsd:string, rdf:langString, \
                     rdf:dirLangString or rdf:HTML literal, got {other}"
                ));
            }
        }
    }
    Ok(crate::report::canonical_messages(messages))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::term::Literal;
    use crate::text_ingest::parse_turtle_document;

    fn load_registry(ttl: &str, base_iri: &str) -> ComponentRegistry {
        let document = parse_turtle_document(ttl, Some(base_iri)).expect("fixture parses");
        let prefixes = PrefixResolver::new(&document.prefixes);
        ComponentRegistry::parse(document.dataset.as_ref(), &prefixes).expect("registry parses")
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
                messages: vec![],
                severity: None,
                annotations: vec![],
                node: Term::NamedNode(NamedNode::from("http://example.org/ns#validator")),
                attachment: sh::NODE_VALIDATOR,
                prebinding: None,
            }],
            property_validators: vec![],
            validators: vec![],
            messages: vec![],
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
        validate_fixture_with_imports(ttl, base_iri, &crate::imports::ShapesImports::new())
            .expect("shapes parse")
    }

    fn validate_fixture_with_imports(
        ttl: &str,
        base_iri: &str,
        imports: &crate::imports::ShapesImports,
    ) -> Result<crate::report::ValidationReport, crate::error::ShapesError> {
        let document = parse_turtle_document(ttl, Some(base_iri)).expect("fixture parses");
        let dataset = document.dataset;
        let shapes = crate::shapes::from_dataset_with_base(
            &dataset,
            Some(base_iri),
            &document.prefixes,
            None,
            None,
            imports,
        )?;
        Ok(crate::engine::validate_dataset(&dataset, &shapes).expect("validation evaluates"))
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
        let error = crate::engine::parse_shapes(ttl, None)
            .unwrap_err()
            .to_string();
        assert!(error.contains("only one is allowed"), "{error}");
    }

    /// The W3C `validator-001` mechanism — a custom component with two parameters
    /// whose ASK validator flags every value that is not their concatenation, typed
    /// through SUBCLASSES of `sh:ConstraintComponent` and `sh:SPARQLAskValidator` —
    /// on an `example.org` fixture whose subclass axioms live in an IMPORTED
    /// document, written for this test. With the document supplied, exactly the
    /// non-conforming target `"Hallo Welt"` is reported; without it, the load
    /// refuses the unresolved import by name.
    #[test]
    fn eval_ask_validator_001() {
        const VOCABULARY: &str = "http://example.org/validator-vocabulary";
        let shapes = format!(
            r#"
            @prefix ex: <http://example.org/ns#> .
            @prefix owl: <http://www.w3.org/2002/07/owl#> .
            @prefix sh: <http://www.w3.org/ns/shacl#> .
            @prefix xsd: <http://www.w3.org/2001/XMLSchema#> .
            <http://example.org/shapes> owl:imports <{VOCABULARY}> .
            ex:TestConstraintComponent a ex:ConstraintComponent ;
                sh:parameter ex:TestParameter1, ex:TestParameter2 ;
                sh:validator [
                    a ex:SPARQLAskValidator ;
                    sh:ask "ASK {{ FILTER (?value = CONCAT($test1, $test2)) }}" ;
                ] .
            ex:TestParameter1 a sh:Parameter ; sh:path ex:test1 ; sh:datatype xsd:string .
            ex:TestParameter2 a sh:Parameter ; sh:path ex:test2 ; sh:datatype xsd:string .
            ex:TestShape a sh:NodeShape ;
                ex:test1 "Hello " ;
                ex:test2 "World" ;
                sh:targetNode "Hallo Welt", "Hello World" .
            "#
        );
        let vocabulary = format!(
            r"
            @prefix ex: <http://example.org/ns#> .
            @prefix owl: <http://www.w3.org/2002/07/owl#> .
            @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
            @prefix sh: <http://www.w3.org/ns/shacl#> .
            <{VOCABULARY}> a owl:Ontology .
            ex:ConstraintComponent rdfs:subClassOf sh:ConstraintComponent .
            ex:SPARQLAskValidator rdfs:subClassOf sh:SPARQLAskValidator .
            "
        );
        let mut imports = crate::imports::ShapesImports::new();
        imports
            .insert_turtle(VOCABULARY, &vocabulary)
            .expect("the imported document parses");
        let report = validate_fixture_with_imports(&shapes, "http://example.org/shapes", &imports)
            .expect("the imported document resolves the import");
        assert!(!report.conforms);
        assert_eq!(report.results.len(), 1, "exactly one non-conforming target");
        assert_eq!(report.results[0].focus_node, lit("Hallo Welt"));
        assert_eq!(report.results[0].value, Some(lit("Hallo Welt")));
        assert_eq!(
            report.results[0].source_constraint_component.as_str(),
            "http://example.org/ns#TestConstraintComponent"
        );

        let refused = validate_fixture_with_imports(
            &shapes,
            "http://example.org/shapes",
            &crate::imports::ShapesImports::new(),
        )
        .expect_err("an import no document resolves is refused");
        assert!(
            matches!(
                refused.as_imports(),
                Some(crate::imports::ShapesImportError::Unresolved { iris })
                    if iris == &[VOCABULARY.to_owned()]
            ),
            "refused for exactly the unresolved import: {refused}"
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
                r.messages
                    .first()
                    .map(Literal::value)
                    .expect("message present")
                    .contains("Values are literals with language"),
                "message template should be substituted"
            );
        }
    }
}

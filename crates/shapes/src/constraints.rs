// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SHACL Core constraint implementations.
//!
//! Evaluates all non-SPARQL SHACL Core constraint components plus the
//! recursive shape evaluator.  PyO3-free.

use std::collections::HashSet;

use crate::data::{GraphFilter, ShaclDataGraph};
use crate::model::{purrdf, rdf, sh};
use crate::path;
use crate::report::ValidationResult;
use crate::shapes::{Constraint, NodeKindValue, Path, PropertyShape, Shape};
use crate::term::{NamedNode, Term, Triple};

// ── Public surface ─────────────────────────────────────────────────────────────

/// Validate a single focus node against a shape, returning all `ValidationResult`s.
///
/// Any result ⇒ non-conformance (regardless of severity).  Recurses for
/// `sh:and`, `sh:or`, `sh:xone`, and `sh:node` constraints.
///
/// A `deactivated` shape produces no results.
pub fn validate_shape<G: ShaclDataGraph>(
    store: &G,
    focus: &Term,
    shape: &Shape,
) -> Vec<ValidationResult> {
    if shape.deactivated {
        return vec![];
    }

    let mut results: Vec<ValidationResult> = Vec::new();

    // --- Node-level constraints (value nodes = [focus], no path) ---
    let node_value_nodes = std::slice::from_ref(focus);
    for constraint in &shape.constraints {
        let mut rs = eval_constraint(store, focus, node_value_nodes, constraint, None, shape);
        for r in &mut rs {
            r.apply_box_roles(&shape.box_roles, &[]);
        }
        results.extend(rs);
    }

    // --- Property shapes ---
    for ps in &shape.property_shapes {
        results.extend(eval_property_shape(store, focus, ps, shape));
    }

    // --- sh:closed (node-shape-level; needs the sibling property shapes) ---
    // `eval_closed` stamps each result's box roles itself — the source roles plus
    // the OFFENDING PREDICATE's path roles — so closed-world violations carry the
    // same predicate attribution that property-shape results do (#700 Gap B).
    for constraint in &shape.constraints {
        if let Constraint::Closed { ignored } = constraint {
            results.extend(eval_closed(store, focus, shape, ignored));
        }
    }

    results
}

/// Evaluate `sh:closed` against a focus node (SHACL §4.8.1).
///
/// The permitted predicate set is the union of:
/// - every simple-predicate `sh:path` of the shape's property shapes (an inverse
///   path constrains incoming, not outgoing, triples and so does not permit an
///   outgoing predicate);
/// - the `sh:ignoredProperties` list; and
/// - `rdf:type` (always allowed).
///
/// One result per focus-node outgoing triple whose predicate is not permitted.
fn eval_closed<G: ShaclDataGraph>(
    store: &G,
    focus: &Term,
    shape: &Shape,
    ignored: &[NamedNode],
) -> Vec<ValidationResult> {
    let mut permitted: HashSet<String> = HashSet::new();
    permitted.insert(rdf::TYPE.to_owned());
    for ps in &shape.property_shapes {
        if let Path::Predicate(predicate) = &ps.path {
            permitted.insert(predicate.as_str().to_owned());
        }
    }
    for ign in ignored {
        permitted.insert(ign.as_str().to_owned());
    }

    let mut results = Vec::new();
    let quads = store.quads_for_pattern(Some(focus), None, None, GraphFilter::AnyGraph);
    for quad in quads {
        let predicate = quad.predicate;
        if permitted.contains(predicate.as_str()) {
            continue;
        }
        // Resolve the offending predicate's graph-box roles (the same resolution
        // property shapes use for their path) so closed-world results are not left
        // with empty path attribution.
        let path_roles = path_box_roles(store, &Path::Predicate(predicate.clone()));
        let mut result = ValidationResult {
            focus_node: focus.clone(),
            result_path: Some(Term::NamedNode(predicate.clone())),
            value: Some(quad.object),
            source_constraint_component: NamedNode::from(sh::CLOSED_CONSTRAINT_COMPONENT),
            source_shape: shape.id.clone(),
            severity: shape.severity,
            message: shape.message.clone(),
            source_box_roles: vec![],
            path_box_roles: vec![],
            result_box_roles: vec![],
            attributions: vec![],
        };
        result.apply_box_roles(&shape.box_roles, &path_roles);
        results.push(result);
    }
    results
}

/// Returns `true` iff the focus node produces zero validation results against
/// the shape (i.e., it fully conforms).
pub fn conforms<G: ShaclDataGraph>(store: &G, focus: &Term, shape: &Shape) -> bool {
    validate_shape(store, focus, shape).is_empty()
}

// ── Property shape evaluator ───────────────────────────────────────────────────

fn eval_property_shape<G: ShaclDataGraph>(
    store: &G,
    focus: &Term,
    ps: &PropertyShape,
    parent_shape: &Shape,
) -> Vec<ValidationResult> {
    let value_nodes = path::eval(store, focus, &ps.path);
    let path_term = path::path_to_term(&ps.path);
    let source_roles = merge_box_roles(&parent_shape.box_roles, &ps.box_roles);
    let path_roles = path_box_roles(store, &ps.path);

    // Build a synthetic shape wrapping the property shape so result
    // metadata (source_shape, severity, message) can come from the PS.
    let ps_as_shape = Shape {
        id: parent_shape.id.clone(),
        targets: vec![],
        constraints: ps.constraints.clone(),
        property_shapes: vec![],
        severity: ps.severity,
        message: ps.message.clone(),
        deactivated: false,
        box_roles: source_roles.clone(),
    };

    let mut results = Vec::new();
    for constraint in &ps.constraints {
        let mut rs = eval_constraint(
            store,
            focus,
            &value_nodes,
            constraint,
            Some(&ps.path),
            &ps_as_shape,
        );
        // Stamp the property-shape path and focus onto every result, but PRESERVE
        // a path the constraint itself bound — a `sh:sparql` query may project
        // `?path` (→ result_path, SHACL-AF §3.4.2.2), which is more specific than
        // the shape's declared path and must not be clobbered.
        for r in &mut rs {
            if r.result_path.is_none() {
                r.result_path = Some(path_term.clone());
            }
            r.focus_node = focus.clone();
            r.apply_box_roles(&source_roles, &path_roles);
        }
        results.extend(rs);
    }
    results.extend(eval_reifier_shapes(ReifierEvalContext {
        store,
        focus,
        value_nodes: &value_nodes,
        ps,
        ps_as_shape: &ps_as_shape,
        source_roles: &source_roles,
        path_roles: &path_roles,
        path_term: &path_term,
    }));
    results
}

struct ReifierEvalContext<'a, G: ShaclDataGraph> {
    store: &'a G,
    focus: &'a Term,
    value_nodes: &'a [Term],
    ps: &'a PropertyShape,
    ps_as_shape: &'a Shape,
    source_roles: &'a [NamedNode],
    path_roles: &'a [NamedNode],
    path_term: &'a Term,
}

// Manual impls (not derives): a `derive(Copy)` would demand `G: Copy`, but the
// context only holds `&G` — every field is a reference, so the struct is Copy
// for ANY data-graph type.
impl<G: ShaclDataGraph> Clone for ReifierEvalContext<'_, G> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<G: ShaclDataGraph> Copy for ReifierEvalContext<'_, G> {}

fn eval_reifier_shapes<G: ShaclDataGraph>(ctx: ReifierEvalContext<'_, G>) -> Vec<ValidationResult> {
    let ReifierEvalContext {
        store,
        focus,
        value_nodes,
        ps,
        ps_as_shape,
        source_roles,
        path_roles,
        path_term,
    } = ctx;
    if ps.reifier_shapes.is_empty() && !ps.reification_required {
        return vec![];
    }
    let Path::Predicate(predicate) = &ps.path else {
        return vec![];
    };

    let source_roles = with_cbox_role(source_roles);
    let mut results = Vec::new();
    for value in value_nodes {
        let Some(triple_term) = triple_term(focus, predicate, value) else {
            continue;
        };
        let reifiers = reifiers_for(store, &triple_term);
        if reifiers.is_empty() && ps.reification_required {
            let mut result = ValidationResult {
                focus_node: focus.clone(),
                result_path: Some(path_term.clone()),
                value: Some(triple_term.clone()),
                source_constraint_component: NamedNode::from(
                    sh::REIFIER_SHAPE_CONSTRAINT_COMPONENT,
                ),
                source_shape: ps_as_shape.id.clone(),
                severity: ps.severity,
                message: ps.message.clone(),
                source_box_roles: vec![],
                path_box_roles: vec![],
                result_box_roles: vec![],
                attributions: vec![],
            };
            result.apply_box_roles(&source_roles, path_roles);
            results.push(result);
            continue;
        }

        for reifier in &reifiers {
            for reifier_shape in &ps.reifier_shapes {
                let inner_results = validate_shape(store, reifier, reifier_shape);
                if inner_results.is_empty() {
                    continue;
                }
                for inner in inner_results {
                    let mut result = ValidationResult {
                        focus_node: focus.clone(),
                        result_path: Some(path_term.clone()),
                        value: Some(triple_term.clone()),
                        source_constraint_component: NamedNode::from(
                            sh::REIFIER_SHAPE_CONSTRAINT_COMPONENT,
                        ),
                        source_shape: inner.source_shape.clone(),
                        severity: inner.severity,
                        message: inner
                            .message
                            .clone()
                            .or_else(|| reifier_shape.message.clone())
                            .or_else(|| ps.message.clone()),
                        source_box_roles: vec![],
                        path_box_roles: vec![],
                        result_box_roles: vec![],
                        attributions: vec![],
                    };
                    let inner_source_roles =
                        merge_box_roles(&source_roles, &inner.source_box_roles);
                    result.apply_box_roles(&inner_source_roles, path_roles);
                    results.push(result);
                }
            }
        }
    }
    results
}

fn triple_term(focus: &Term, predicate: &NamedNode, value: &Term) -> Option<Term> {
    // A quoted triple's subject must be an IRI or blank node.
    if !focus.is_subject() {
        return None;
    }
    Some(Term::Triple(Box::new(Triple::new(
        focus.clone(),
        predicate.clone(),
        value.clone(),
    ))))
}

fn reifiers_for<G: ShaclDataGraph>(store: &G, triple_term: &Term) -> Vec<Term> {
    let reifies = Term::NamedNode(NamedNode::from(rdf::REIFIES));
    let reifiers_set: HashSet<Term> = store
        .quads_for_pattern(
            None,
            Some(&reifies),
            Some(triple_term),
            GraphFilter::DefaultGraph,
        )
        .into_iter()
        .map(|q| q.subject)
        .collect();
    let mut reifiers: Vec<Term> = reifiers_set.into_iter().collect();
    reifiers.sort_by_key(Term::to_string);
    reifiers
}

fn path_box_roles<G: ShaclDataGraph>(store: &G, path: &Path) -> Vec<NamedNode> {
    let predicate = match path {
        Path::Predicate(predicate) => predicate,
        Path::Inverse(inner) => match inner.as_ref() {
            Path::Predicate(predicate) => predicate,
            Path::Inverse(_) => return vec![],
        },
    };
    let predicate_term = Term::NamedNode(predicate.clone());
    let box_role = Term::NamedNode(NamedNode::from(purrdf::GRAPH_BOX_ROLE));
    let mut roles: Vec<NamedNode> = store
        .quads_for_pattern(
            Some(&predicate_term),
            Some(&box_role),
            None,
            GraphFilter::DefaultGraph,
        )
        .into_iter()
        .filter_map(|q| match q.object {
            Term::NamedNode(node) => Some(node),
            _ => None,
        })
        .collect();
    roles.sort_unstable();
    roles.dedup();
    roles
}

fn with_cbox_role(source_roles: &[NamedNode]) -> Vec<NamedNode> {
    merge_box_roles(source_roles, &[NamedNode::from(purrdf::BOX_CBOX)])
}

fn merge_box_roles(left: &[NamedNode], right: &[NamedNode]) -> Vec<NamedNode> {
    let mut roles = left.to_vec();
    roles.extend_from_slice(right);
    roles.sort_unstable();
    roles.dedup();
    roles
}

// ── Per-constraint evaluator ───────────────────────────────────────────────────

/// Evaluate a single constraint against the provided value node set.
///
/// `focus_node` is the SHACL focus node (subject) — always the real focus, never
/// a path value.  For node-level constraints `focus_node == value_nodes[0]`; for
/// property shapes `focus_node` is the subject while `value_nodes` are the path
/// objects.  `sh:sparql`'s `$this` must bind to `focus_node` in both contexts
/// (SHACL-AF spec: `$this` = focus node, not value node).
///
/// `path` is `None` for node-level constraints, `Some` for property shapes.
fn eval_constraint<G: ShaclDataGraph>(
    store: &G,
    focus_node: &Term,
    value_nodes: &[Term],
    constraint: &Constraint,
    path: Option<&Path>,
    shape: &Shape,
) -> Vec<ValidationResult> {
    let result_path = path.map(path::path_to_term);
    let severity = shape.severity;
    let message = shape.message.clone();
    let source_shape = shape.id.clone();

    macro_rules! result {
        ($component:expr, $value:expr) => {
            ValidationResult {
                focus_node: value_nodes
                    .first()
                    .cloned()
                    .unwrap_or_else(|| source_shape.clone()),
                result_path: result_path.clone(),
                value: $value,
                source_constraint_component: NamedNode::from($component),
                source_shape: source_shape.clone(),
                severity,
                message: message.clone(),
                source_box_roles: vec![],
                path_box_roles: vec![],
                result_box_roles: vec![],
                attributions: vec![],
            }
        };
        ($component:expr, $focus:expr, $value:expr) => {
            ValidationResult {
                focus_node: $focus,
                result_path: result_path.clone(),
                value: $value,
                source_constraint_component: NamedNode::from($component),
                source_shape: source_shape.clone(),
                severity,
                message: message.clone(),
                source_box_roles: vec![],
                path_box_roles: vec![],
                result_box_roles: vec![],
                attributions: vec![],
            }
        };
    }

    match constraint {
        // ── Count constraints (operate on the SET) ─────────────────────────────
        Constraint::MinCount(n) => {
            let count = value_nodes.len() as u64;
            if count < *n {
                vec![result!(sh::MIN_COUNT_CONSTRAINT_COMPONENT, None)]
            } else {
                vec![]
            }
        }
        Constraint::MaxCount(n) => {
            let count = value_nodes.len() as u64;
            if count > *n {
                vec![result!(sh::MAX_COUNT_CONSTRAINT_COMPONENT, None)]
            } else {
                vec![]
            }
        }

        // ── Class (per value node; honors asserted rdfs:subClassOf, §4.2.5) ────
        Constraint::Class(class_iri) => {
            // Hoist the BFS closure computation once, outside the per-value loop.
            // Previously called inside the loop: O(N×M) → now O(M) + O(N).
            let closure = crate::engine::subclass_closure(store, class_iri);
            let mut results = Vec::new();
            let focus = value_nodes
                .first()
                .cloned()
                .unwrap_or_else(|| source_shape.clone());
            for value in value_nodes {
                let violates = match value {
                    Term::Literal(_) => true,
                    _ => !is_shacl_instance(store, value, &closure),
                };
                if violates {
                    results.push(ValidationResult {
                        focus_node: focus.clone(),
                        result_path: result_path.clone(),
                        value: Some(value.clone()),
                        source_constraint_component: NamedNode::from(
                            sh::CLASS_CONSTRAINT_COMPONENT,
                        ),
                        source_shape: source_shape.clone(),
                        severity,
                        message: message.clone(),
                        source_box_roles: vec![],
                        path_box_roles: vec![],
                        result_box_roles: vec![],
                        attributions: vec![],
                    });
                }
            }
            results
        }

        // ── Datatype (per value node) ──────────────────────────────────────────
        Constraint::Datatype(dt_iri) => {
            let mut results = Vec::new();
            let focus = value_nodes
                .first()
                .cloned()
                .unwrap_or_else(|| source_shape.clone());
            for value in value_nodes {
                if !check_datatype(value, dt_iri) {
                    results.push(ValidationResult {
                        focus_node: focus.clone(),
                        result_path: result_path.clone(),
                        value: Some(value.clone()),
                        source_constraint_component: NamedNode::from(
                            sh::DATATYPE_CONSTRAINT_COMPONENT,
                        ),
                        source_shape: source_shape.clone(),
                        severity,
                        message: message.clone(),
                        source_box_roles: vec![],
                        path_box_roles: vec![],
                        result_box_roles: vec![],
                        attributions: vec![],
                    });
                }
            }
            results
        }

        // ── NodeKind (per value node) ──────────────────────────────────────────
        Constraint::NodeKind(kind) => {
            let mut results = Vec::new();
            let focus = value_nodes
                .first()
                .cloned()
                .unwrap_or_else(|| source_shape.clone());
            for value in value_nodes {
                if !check_node_kind(value, kind) {
                    results.push(ValidationResult {
                        focus_node: focus.clone(),
                        result_path: result_path.clone(),
                        value: Some(value.clone()),
                        source_constraint_component: NamedNode::from(
                            sh::NODE_KIND_CONSTRAINT_COMPONENT,
                        ),
                        source_shape: source_shape.clone(),
                        severity,
                        message: message.clone(),
                        source_box_roles: vec![],
                        path_box_roles: vec![],
                        result_box_roles: vec![],
                        attributions: vec![],
                    });
                }
            }
            results
        }

        // ── In (per value node) ────────────────────────────────────────────────
        Constraint::In(allowed) => {
            let mut results = Vec::new();
            let focus = value_nodes
                .first()
                .cloned()
                .unwrap_or_else(|| source_shape.clone());
            for value in value_nodes {
                if !allowed.iter().any(|a| terms_equal(a, value)) {
                    results.push(ValidationResult {
                        focus_node: focus.clone(),
                        result_path: result_path.clone(),
                        value: Some(value.clone()),
                        source_constraint_component: NamedNode::from(sh::IN_CONSTRAINT_COMPONENT),
                        source_shape: source_shape.clone(),
                        severity,
                        message: message.clone(),
                        source_box_roles: vec![],
                        path_box_roles: vec![],
                        result_box_roles: vec![],
                        attributions: vec![],
                    });
                }
            }
            results
        }

        // ── HasValue (on the SET, one result if missing) ───────────────────────
        Constraint::HasValue(required) => {
            let found = value_nodes.iter().any(|v| terms_equal(v, required));
            if !found {
                let focus = value_nodes
                    .first()
                    .cloned()
                    .unwrap_or_else(|| source_shape.clone());
                vec![ValidationResult {
                    focus_node: focus,
                    result_path,
                    value: None,
                    source_constraint_component: NamedNode::from(
                        sh::HAS_VALUE_CONSTRAINT_COMPONENT,
                    ),
                    source_shape,
                    severity,
                    message,
                    source_box_roles: vec![],
                    path_box_roles: vec![],
                    result_box_roles: vec![],
                    attributions: vec![],
                }]
            } else {
                vec![]
            }
        }

        // ── Pattern (per value node) ───────────────────────────────────────────
        Constraint::Pattern {
            regex,
            flags,
            compiled,
        } => {
            // Compile at most once per Constraint instance (across all focus
            // nodes and value nodes) using the OnceLock cache.  Behaviour is
            // identical to the per-call path: Err ⇒ violation on every value.
            let compiled: &Result<regex::Regex, String> =
                compiled.get_or_init(|| build_regex(regex, flags.as_deref()));
            let mut results = Vec::new();
            let focus = value_nodes
                .first()
                .cloned()
                .unwrap_or_else(|| source_shape.clone());
            for value in value_nodes {
                let lexical = match value {
                    Term::Literal(lit) => Some(lit.value().to_owned()),
                    Term::NamedNode(nn) => Some(nn.as_str().to_owned()),
                    _ => None,
                };
                let violates = match (compiled, &lexical) {
                    (Err(_), _) => true,   // bad regex → violation on every value node
                    (Ok(_), None) => true, // blank node → violation
                    (Ok(re), Some(lex)) => !re.is_match(lex),
                };
                if violates {
                    results.push(ValidationResult {
                        focus_node: focus.clone(),
                        result_path: result_path.clone(),
                        value: Some(value.clone()),
                        source_constraint_component: NamedNode::from(
                            sh::PATTERN_CONSTRAINT_COMPONENT,
                        ),
                        source_shape: source_shape.clone(),
                        severity,
                        message: message.clone(),
                        source_box_roles: vec![],
                        path_box_roles: vec![],
                        result_box_roles: vec![],
                        attributions: vec![],
                    });
                }
            }
            results
        }

        // ── MinLength (per value node) ─────────────────────────────────────────
        Constraint::MinLength(n) => {
            let mut results = Vec::new();
            let focus = value_nodes
                .first()
                .cloned()
                .unwrap_or_else(|| source_shape.clone());
            for value in value_nodes {
                let len_opt = lexical_length(value);
                let violates = match len_opt {
                    None => true, // blank node
                    Some(len) => (len as u64) < *n,
                };
                if violates {
                    results.push(ValidationResult {
                        focus_node: focus.clone(),
                        result_path: result_path.clone(),
                        value: Some(value.clone()),
                        source_constraint_component: NamedNode::from(
                            sh::MIN_LENGTH_CONSTRAINT_COMPONENT,
                        ),
                        source_shape: source_shape.clone(),
                        severity,
                        message: message.clone(),
                        source_box_roles: vec![],
                        path_box_roles: vec![],
                        result_box_roles: vec![],
                        attributions: vec![],
                    });
                }
            }
            results
        }

        // ── MaxLength (per value node) ─────────────────────────────────────────
        Constraint::MaxLength(n) => {
            let mut results = Vec::new();
            let focus = value_nodes
                .first()
                .cloned()
                .unwrap_or_else(|| source_shape.clone());
            for value in value_nodes {
                let len_opt = lexical_length(value);
                let violates = match len_opt {
                    None => true, // blank node
                    Some(len) => (len as u64) > *n,
                };
                if violates {
                    results.push(ValidationResult {
                        focus_node: focus.clone(),
                        result_path: result_path.clone(),
                        value: Some(value.clone()),
                        source_constraint_component: NamedNode::from(
                            sh::MAX_LENGTH_CONSTRAINT_COMPONENT,
                        ),
                        source_shape: source_shape.clone(),
                        severity,
                        message: message.clone(),
                        source_box_roles: vec![],
                        path_box_roles: vec![],
                        result_box_roles: vec![],
                        attributions: vec![],
                    });
                }
            }
            results
        }

        // ── LanguageIn (per value node) ────────────────────────────────────────
        Constraint::LanguageIn(tags) => {
            let mut results = Vec::new();
            let focus = value_nodes
                .first()
                .cloned()
                .unwrap_or_else(|| source_shape.clone());
            for value in value_nodes {
                if !language_matches_any(value, tags) {
                    results.push(ValidationResult {
                        focus_node: focus.clone(),
                        result_path: result_path.clone(),
                        value: Some(value.clone()),
                        source_constraint_component: NamedNode::from(
                            sh::LANGUAGE_IN_CONSTRAINT_COMPONENT,
                        ),
                        source_shape: source_shape.clone(),
                        severity,
                        message: message.clone(),
                        source_box_roles: vec![],
                        path_box_roles: vec![],
                        result_box_roles: vec![],
                        attributions: vec![],
                    });
                }
            }
            results
        }

        // ── Not (per value node, recursive) ────────────────────────────────────
        Constraint::Not(inner_shape) => {
            let mut results = Vec::new();
            let focus = value_nodes
                .first()
                .cloned()
                .unwrap_or_else(|| source_shape.clone());
            for value in value_nodes {
                // Violation iff the value node DOES conform to the negated shape.
                if conforms(store, value, inner_shape) {
                    results.push(ValidationResult {
                        focus_node: focus.clone(),
                        result_path: result_path.clone(),
                        value: Some(value.clone()),
                        source_constraint_component: NamedNode::from(sh::NOT_CONSTRAINT_COMPONENT),
                        source_shape: source_shape.clone(),
                        severity,
                        message: message.clone(),
                        source_box_roles: vec![],
                        path_box_roles: vec![],
                        result_box_roles: vec![],
                        attributions: vec![],
                    });
                }
            }
            results
        }

        // ── Closed (node-shape-level; evaluated in validate_shape) ─────────────
        // The closed-world check needs the SET of permitted predicates, derived
        // from the sibling property shapes — data `eval_constraint` does not
        // receive. It is evaluated directly in `validate_shape`; here it is a
        // no-op so the match stays exhaustive.
        Constraint::Closed { .. } => vec![],

        // ── UniqueLang (on the SET) ────────────────────────────────────────────
        Constraint::UniqueLang(true) => {
            let mut seen_langs: std::collections::HashMap<String, usize> =
                std::collections::HashMap::new();
            for value in value_nodes {
                if let Term::Literal(lit) = value {
                    if let Some(lang) = lit.language() {
                        *seen_langs.entry(lang.to_lowercase()).or_insert(0) += 1;
                    }
                }
            }
            let focus = value_nodes
                .first()
                .cloned()
                .unwrap_or_else(|| source_shape.clone());
            let mut results = Vec::new();
            for (lang, count) in &seen_langs {
                if *count > 1 {
                    results.push(ValidationResult {
                        focus_node: focus.clone(),
                        result_path: result_path.clone(),
                        value: None,
                        source_constraint_component: NamedNode::from(
                            sh::UNIQUE_LANG_CONSTRAINT_COMPONENT,
                        ),
                        source_shape: source_shape.clone(),
                        severity,
                        message: message
                            .clone()
                            .or_else(|| Some(format!("duplicate language tag: {lang}"))),
                        source_box_roles: vec![],
                        path_box_roles: vec![],
                        result_box_roles: vec![],
                        attributions: vec![],
                    });
                }
            }
            results
        }
        Constraint::UniqueLang(false) => vec![],

        // ── MinInclusive / MaxInclusive (per value node) ───────────────────────
        Constraint::MinInclusive(bound) => {
            let bound_num = numeric_value(bound);
            let mut results = Vec::new();
            let focus = value_nodes
                .first()
                .cloned()
                .unwrap_or_else(|| source_shape.clone());
            for value in value_nodes {
                let violates = match (numeric_value(value), &bound_num) {
                    (Some(v), Some(b)) => v < *b,
                    _ => true,
                };
                if violates {
                    results.push(ValidationResult {
                        focus_node: focus.clone(),
                        result_path: result_path.clone(),
                        value: Some(value.clone()),
                        source_constraint_component: NamedNode::from(
                            sh::MIN_INCLUSIVE_CONSTRAINT_COMPONENT,
                        ),
                        source_shape: source_shape.clone(),
                        severity,
                        message: message.clone(),
                        source_box_roles: vec![],
                        path_box_roles: vec![],
                        result_box_roles: vec![],
                        attributions: vec![],
                    });
                }
            }
            results
        }
        Constraint::MaxInclusive(bound) => {
            let bound_num = numeric_value(bound);
            let mut results = Vec::new();
            let focus = value_nodes
                .first()
                .cloned()
                .unwrap_or_else(|| source_shape.clone());
            for value in value_nodes {
                let violates = match (numeric_value(value), &bound_num) {
                    (Some(v), Some(b)) => v > *b,
                    _ => true,
                };
                if violates {
                    results.push(ValidationResult {
                        focus_node: focus.clone(),
                        result_path: result_path.clone(),
                        value: Some(value.clone()),
                        source_constraint_component: NamedNode::from(
                            sh::MAX_INCLUSIVE_CONSTRAINT_COMPONENT,
                        ),
                        source_shape: source_shape.clone(),
                        severity,
                        message: message.clone(),
                        source_box_roles: vec![],
                        path_box_roles: vec![],
                        result_box_roles: vec![],
                        attributions: vec![],
                    });
                }
            }
            results
        }

        // ── MinExclusive / MaxExclusive (per value node) ───────────────────────
        Constraint::MinExclusive(bound) => {
            let bound_num = numeric_value(bound);
            let mut results = Vec::new();
            let focus = value_nodes
                .first()
                .cloned()
                .unwrap_or_else(|| source_shape.clone());
            for value in value_nodes {
                let violates = match (numeric_value(value), &bound_num) {
                    (Some(v), Some(b)) => v <= *b,
                    _ => true,
                };
                if violates {
                    results.push(ValidationResult {
                        focus_node: focus.clone(),
                        result_path: result_path.clone(),
                        value: Some(value.clone()),
                        source_constraint_component: NamedNode::from(
                            sh::MIN_EXCLUSIVE_CONSTRAINT_COMPONENT,
                        ),
                        source_shape: source_shape.clone(),
                        severity,
                        message: message.clone(),
                        source_box_roles: vec![],
                        path_box_roles: vec![],
                        result_box_roles: vec![],
                        attributions: vec![],
                    });
                }
            }
            results
        }
        Constraint::MaxExclusive(bound) => {
            let bound_num = numeric_value(bound);
            let mut results = Vec::new();
            let focus = value_nodes
                .first()
                .cloned()
                .unwrap_or_else(|| source_shape.clone());
            for value in value_nodes {
                let violates = match (numeric_value(value), &bound_num) {
                    (Some(v), Some(b)) => v >= *b,
                    _ => true,
                };
                if violates {
                    results.push(ValidationResult {
                        focus_node: focus.clone(),
                        result_path: result_path.clone(),
                        value: Some(value.clone()),
                        source_constraint_component: NamedNode::from(
                            sh::MAX_EXCLUSIVE_CONSTRAINT_COMPONENT,
                        ),
                        source_shape: source_shape.clone(),
                        severity,
                        message: message.clone(),
                        source_box_roles: vec![],
                        path_box_roles: vec![],
                        result_box_roles: vec![],
                        attributions: vec![],
                    });
                }
            }
            results
        }

        // ── And (per value node, recursive) ───────────────────────────────────
        Constraint::And(members) => {
            let mut results = Vec::new();
            let focus = value_nodes
                .first()
                .cloned()
                .unwrap_or_else(|| source_shape.clone());
            for value in value_nodes {
                let all_conform = members.iter().all(|m| conforms(store, value, m));
                if !all_conform {
                    results.push(ValidationResult {
                        focus_node: focus.clone(),
                        result_path: result_path.clone(),
                        value: Some(value.clone()),
                        source_constraint_component: NamedNode::from(sh::AND_CONSTRAINT_COMPONENT),
                        source_shape: source_shape.clone(),
                        severity,
                        message: message.clone(),
                        source_box_roles: vec![],
                        path_box_roles: vec![],
                        result_box_roles: vec![],
                        attributions: vec![],
                    });
                }
            }
            results
        }

        // ── Or (per value node, recursive) ────────────────────────────────────
        Constraint::Or(members) => {
            let mut results = Vec::new();
            let focus = value_nodes
                .first()
                .cloned()
                .unwrap_or_else(|| source_shape.clone());
            for value in value_nodes {
                let any_conforms = members.iter().any(|m| conforms(store, value, m));
                if !any_conforms {
                    results.push(ValidationResult {
                        focus_node: focus.clone(),
                        result_path: result_path.clone(),
                        value: Some(value.clone()),
                        source_constraint_component: NamedNode::from(sh::OR_CONSTRAINT_COMPONENT),
                        source_shape: source_shape.clone(),
                        severity,
                        message: message.clone(),
                        source_box_roles: vec![],
                        path_box_roles: vec![],
                        result_box_roles: vec![],
                        attributions: vec![],
                    });
                }
            }
            results
        }

        // ── Xone (per value node, recursive) ──────────────────────────────────
        Constraint::Xone(members) => {
            let mut results = Vec::new();
            let focus = value_nodes
                .first()
                .cloned()
                .unwrap_or_else(|| source_shape.clone());
            for value in value_nodes {
                let count = members.iter().filter(|m| conforms(store, value, m)).count();
                if count != 1 {
                    results.push(ValidationResult {
                        focus_node: focus.clone(),
                        result_path: result_path.clone(),
                        value: Some(value.clone()),
                        source_constraint_component: NamedNode::from(sh::XONE_CONSTRAINT_COMPONENT),
                        source_shape: source_shape.clone(),
                        severity,
                        message: message.clone(),
                        source_box_roles: vec![],
                        path_box_roles: vec![],
                        result_box_roles: vec![],
                        attributions: vec![],
                    });
                }
            }
            results
        }

        // ── Node (per value node, recursive) ──────────────────────────────────
        Constraint::Node(inner_shape) => {
            let mut results = Vec::new();
            let focus = value_nodes
                .first()
                .cloned()
                .unwrap_or_else(|| source_shape.clone());
            for value in value_nodes {
                if !conforms(store, value, inner_shape) {
                    results.push(ValidationResult {
                        focus_node: focus.clone(),
                        result_path: result_path.clone(),
                        value: Some(value.clone()),
                        source_constraint_component: NamedNode::from(sh::NODE_CONSTRAINT_COMPONENT),
                        source_shape: source_shape.clone(),
                        severity,
                        message: message.clone(),
                        source_box_roles: vec![],
                        path_box_roles: vec![],
                        result_box_roles: vec![],
                        attributions: vec![],
                    });
                }
            }
            results
        }

        // ── Sparql (SHACL-AF — $this always binds to the focus node, never to a
        //           path value node.  SHACL-AF spec §3.4: for sh:sparql on a
        //           property shape, $this is still the focus subject; the path
        //           objects are NOT auto-bound.)
        //
        // The constraint blank node may carry its own sh:message / sh:severity;
        // those override the shape-level defaults at eval time.
        // SELECT-form is enforced at shape-load (shapes.rs rejects non-SELECT), so the only Err here is an impossible-by-construction runtime failure; .expect() documents that invariant.
        // The native SPARQL engine runs the validated query text over the dataset,
        // substituting $this for this focus node (SparqlRequest.substitutions).
        Constraint::Sparql {
            select,
            message: cmsg,
            severity: csev,
        } => {
            let sev = csev.unwrap_or(severity);
            let msg = cmsg.clone().or_else(|| message.clone());
            crate::sparql::eval_sparql_constraint(
                &store.sparql_dataset(),
                focus_node,
                select,
                &NamedNode::from(sh::SPARQL_CONSTRAINT_COMPONENT),
                &source_shape,
                sev,
                msg.as_ref(),
            )
            .expect("sh:sparql query execution failed (parseability checked at parse time)")
        }
    }
}

// ── Helper functions ───────────────────────────────────────────────────────────

/// Whether `value` is a SHACL instance of a class, given a precomputed subclass
/// closure (SHACL §4.2.5).
///
/// `closure` must contain the class IRI itself plus every transitive subclass
/// derived from asserted `rdfs:subClassOf` edges (as returned by
/// [`crate::engine::subclass_closure`]).  The caller hoists the closure
/// computation once before the per-value-node loop to avoid O(N×M) BFS cost.
fn is_shacl_instance<G: ShaclDataGraph>(store: &G, value: &Term, closure: &HashSet<Term>) -> bool {
    if !matches!(value, Term::NamedNode(_) | Term::BlankNode(_)) {
        return false;
    }
    let rdf_type = Term::NamedNode(NamedNode::from(rdf::TYPE));
    store
        .quads_for_pattern(Some(value), Some(&rdf_type), None, GraphFilter::AnyGraph)
        .into_iter()
        .any(|q| closure.contains(&q.object))
}

/// `xsd:integer` lexical space: optional sign then one-or-more ASCII digits.
/// Unbounded — no native-int overflow.
fn is_xsd_integer_lexical(s: &str) -> bool {
    let s = s.trim();
    let digits = s.strip_prefix(['+', '-']).unwrap_or(s);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

/// `xsd:decimal` lexical space: optional sign then digits with an optional
/// single '.' — NO exponent. At least one digit must be present.
fn is_xsd_decimal_lexical(s: &str) -> bool {
    let s = s.trim();
    let body = s.strip_prefix(['+', '-']).unwrap_or(s);
    if body.is_empty() {
        return false;
    }
    let mut seen_dot = false;
    let mut seen_digit = false;
    for b in body.bytes() {
        match b {
            b'0'..=b'9' => seen_digit = true,
            b'.' if !seen_dot => seen_dot = true,
            _ => return false, // rejects 'e'/'E' (scientific notation) and any other char
        }
    }
    seen_digit
}

/// `xsd:double`/`xsd:float` lexical space: the three special values exactly
/// (INF, -INF, NaN — case-sensitive per XSD), or a mantissa (decimal lexical)
/// with an optional [eE][+-]?digits exponent.
fn is_xsd_double_lexical(s: &str) -> bool {
    let s = s.trim();
    if matches!(s, "INF" | "-INF" | "NaN") {
        return true;
    }
    // Split optional exponent.
    let (mantissa, exponent) = match s.split_once(['e', 'E']) {
        Some((m, e)) => (m, Some(e)),
        None => (s, None),
    };
    if !is_xsd_decimal_lexical(mantissa) {
        return false;
    }
    match exponent {
        None => true,
        Some(exp) => {
            let digits = exp.strip_prefix(['+', '-']).unwrap_or(exp);
            !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
        }
    }
}

/// Check that a `Term` satisfies `sh:datatype` requirements.
///
/// - Must be a `Literal` whose datatype matches `dt_iri`.
/// - On an exact datatype-IRI match, additionally validates the lexical form for
///   common XSD types (xsd:integer unbounded, xsd:decimal no scientific notation,
///   xsd:double/float, xsd:boolean).
/// - Oxigraph canonicalizes XSD derived integer types to `xsd:integer` in the
///   store at load time (e.g. `"1"^^xsd:nonNegativeInteger` becomes
///   `"1"^^xsd:integer`), which would break the exact-IRI match. When the shape
///   requires such a derived type and the stored literal carries the canonical
///   base, accept iff the lexical value lies in the derived type's value space.
///   This matches pySHACL's spec-correct result. See #598.
fn check_datatype(value: &Term, dt_iri: &NamedNode) -> bool {
    let Term::Literal(lit) = value else {
        return false;
    };
    let stored_dt = lit.datatype();
    let lex = lit.value();
    if stored_dt.as_str() == dt_iri.as_str() {
        // Exact datatype-IRI match. For the primitive types validate the lexical
        // form; for a DERIVED integer type the literal is faithfully typed (the
        // native value space does not normalize it to xsd:integer the way oxigraph
        // did), so additionally validate the VALUE space — a `"-2"^^nonNegativeInteger`
        // is lexically an integer but out of the derived type's range and must
        // violate. `XSD_INTEGER` is the canonical fold target the value-space check
        // keys on, so check the stored value against xsd:integer first.
        if is_derived_integer_type(dt_iri.as_str()) {
            return derived_integer_matches(XSD_INTEGER, dt_iri.as_str(), lex);
        }
        return xsd_lexical_valid(dt_iri.as_str(), lex);
    }
    derived_integer_matches(stored_dt.as_str(), dt_iri.as_str(), lex)
}

/// The XSD integer-derived datatype IRIs whose VALUE space is narrower than
/// `xsd:integer` (so an exact datatype match still requires a range check).
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";

fn is_derived_integer_type(dt: &str) -> bool {
    matches!(
        dt,
        "http://www.w3.org/2001/XMLSchema#nonNegativeInteger"
            | "http://www.w3.org/2001/XMLSchema#positiveInteger"
            | "http://www.w3.org/2001/XMLSchema#nonPositiveInteger"
            | "http://www.w3.org/2001/XMLSchema#negativeInteger"
            | "http://www.w3.org/2001/XMLSchema#long"
            | "http://www.w3.org/2001/XMLSchema#int"
            | "http://www.w3.org/2001/XMLSchema#short"
            | "http://www.w3.org/2001/XMLSchema#byte"
            | "http://www.w3.org/2001/XMLSchema#unsignedLong"
            | "http://www.w3.org/2001/XMLSchema#unsignedInt"
            | "http://www.w3.org/2001/XMLSchema#unsignedShort"
            | "http://www.w3.org/2001/XMLSchema#unsignedByte"
    )
}

/// Lexical-form validity for an exact datatype-IRI match. Unknown datatypes are
/// accepted (no lexical facet enforced).
fn xsd_lexical_valid(dt: &str, lex: &str) -> bool {
    match dt {
        "http://www.w3.org/2001/XMLSchema#integer" => is_xsd_integer_lexical(lex),
        "http://www.w3.org/2001/XMLSchema#decimal" => is_xsd_decimal_lexical(lex),
        "http://www.w3.org/2001/XMLSchema#double" => is_xsd_double_lexical(lex),
        "http://www.w3.org/2001/XMLSchema#float" => is_xsd_double_lexical(lex),
        "http://www.w3.org/2001/XMLSchema#boolean" => {
            matches!(lex.trim(), "true" | "false" | "1" | "0")
        }
        _ => true,
    }
}

/// Whether a literal that oxigraph stored as the canonical base type satisfies a
/// shape's required XSD *derived* integer type, by validating the lexical value
/// against the derived type's value space. Every XSD integer-derived type
/// canonicalizes to `xsd:integer` in oxigraph; only that base is considered here.
/// See #598.
fn derived_integer_matches(stored_dt: &str, required_dt: &str, lex: &str) -> bool {
    const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
    if stored_dt != XSD_INTEGER || !is_xsd_integer_lexical(lex) {
        return false;
    }
    let trimmed = lex.trim();
    // For sign-constrained but unbounded types, fall back to a lexical sign check
    // when the magnitude exceeds i128 (astronomically large; never in practice).
    let value = trimmed.parse::<i128>().ok();
    let is_negative = || value.map_or_else(|| trimmed.starts_with('-'), |n| n < 0);
    let is_positive = || value.map_or_else(|| !trimmed.starts_with('-'), |n| n > 0);
    let is_zero = || value == Some(0);
    match required_dt {
        "http://www.w3.org/2001/XMLSchema#nonNegativeInteger" => !is_negative(),
        "http://www.w3.org/2001/XMLSchema#positiveInteger" => is_positive(),
        "http://www.w3.org/2001/XMLSchema#nonPositiveInteger" => is_negative() || is_zero(),
        "http://www.w3.org/2001/XMLSchema#negativeInteger" => is_negative(),
        "http://www.w3.org/2001/XMLSchema#long" => trimmed.parse::<i64>().is_ok(),
        "http://www.w3.org/2001/XMLSchema#int" => trimmed.parse::<i32>().is_ok(),
        "http://www.w3.org/2001/XMLSchema#short" => trimmed.parse::<i16>().is_ok(),
        "http://www.w3.org/2001/XMLSchema#byte" => trimmed.parse::<i8>().is_ok(),
        "http://www.w3.org/2001/XMLSchema#unsignedLong" => trimmed.parse::<u64>().is_ok(),
        "http://www.w3.org/2001/XMLSchema#unsignedInt" => trimmed.parse::<u32>().is_ok(),
        "http://www.w3.org/2001/XMLSchema#unsignedShort" => trimmed.parse::<u16>().is_ok(),
        "http://www.w3.org/2001/XMLSchema#unsignedByte" => trimmed.parse::<u8>().is_ok(),
        _ => false,
    }
}

/// Check that a `Term` satisfies `sh:nodeKind`.
fn check_node_kind(value: &Term, kind: &NodeKindValue) -> bool {
    matches!(
        (value, kind),
        (
            Term::NamedNode(_),
            NodeKindValue::Iri | NodeKindValue::BlankNodeOrIri | NodeKindValue::IriOrLiteral
        ) | (
            Term::BlankNode(_),
            NodeKindValue::BlankNode
                | NodeKindValue::BlankNodeOrIri
                | NodeKindValue::BlankNodeOrLiteral
        ) | (
            Term::Literal(_),
            NodeKindValue::Literal
                | NodeKindValue::BlankNodeOrLiteral
                | NodeKindValue::IriOrLiteral
        )
    )
}

/// Return the character count of the lexical form of `value`, or `None` for
/// blank nodes (which violate `sh:minLength`).
fn lexical_length(value: &Term) -> Option<usize> {
    match value {
        Term::Literal(lit) => Some(lit.value().chars().count()),
        Term::NamedNode(nn) => Some(nn.as_str().chars().count()),
        _ => None,
    }
}

/// Whether a value node's language tag matches any entry in an `sh:languageIn`
/// list, using SHACL basic-filtering / prefix semantics (RFC 4647 §3.3.1).
///
/// A value tag matches an entry iff, comparing case-insensitively, it equals the
/// entry or extends it at a subtag boundary (e.g. `"en"` matches `"en"` and
/// `"en-US"`, but not `"eng"`). A non-language-tagged literal (or any non-literal)
/// never matches, so it always violates the constraint.
fn language_matches_any(value: &Term, tags: &[String]) -> bool {
    let Term::Literal(lit) = value else {
        return false;
    };
    let Some(lang) = lit.language() else {
        return false;
    };
    // RFC 4647 basic filtering, case-insensitive, allocation-free: compare ASCII
    // slices in place rather than lowercasing `lang` and each `entry` per call.
    tags.iter().any(|entry| {
        if lang.eq_ignore_ascii_case(entry) {
            return true;
        }
        lang.len() > entry.len()
            && lang.as_bytes()[entry.len()] == b'-'
            && lang[..entry.len()].eq_ignore_ascii_case(entry)
    })
}

/// Parse a numeric value (xsd:integer, xsd:decimal, xsd:double) as `f64`.
fn numeric_value(term: &Term) -> Option<f64> {
    const XSD_NS: &str = "http://www.w3.org/2001/XMLSchema#";
    let Term::Literal(lit) = term else {
        return None;
    };
    // The full XSD numeric lattice: the primitives plus EVERY derived integer
    // datatype. The set must match the rest of the engine (see
    // `instance.rs::numeric_or_bool_scalar`); the previous list omitted the
    // derived/unsigned integers (e.g. `xsd:nonNegativeInteger`), so a faithful
    // `"1"^^xsd:nonNegativeInteger` value read as non-numeric and spuriously
    // violated every `sh:minInclusive`/`sh:maxInclusive` facet. (The omission was
    // masked while data round-tripped through oxigraph's NT serializer, which
    // value-space-normalized such literals to `xsd:integer`; the oxigraph-free
    // path, #906, is the faithful one and exposes the gap.)
    let local = lit.datatype_str().strip_prefix(XSD_NS)?;
    if matches!(
        local,
        "integer"
            | "decimal"
            | "double"
            | "float"
            | "long"
            | "int"
            | "short"
            | "byte"
            | "nonNegativeInteger"
            | "positiveInteger"
            | "nonPositiveInteger"
            | "negativeInteger"
            | "unsignedLong"
            | "unsignedInt"
            | "unsignedShort"
            | "unsignedByte"
    ) {
        lit.value().trim().parse::<f64>().ok()
    } else {
        None
    }
}

/// Term equality: two terms are equal iff their string representations match
/// (oxigraph's `PartialEq` does the right thing for typed literals).
fn terms_equal(a: &Term, b: &Term) -> bool {
    a == b
}

/// Build a compiled `Regex` from a pattern string and optional `sh:flags` string.
///
/// Supported flags (XPath 2.0 subset with Rust `regex` semantics):
/// - `i` — case-insensitive
/// - `s` — dot-all (`.` matches newlines)
/// - `m` — multi-line (`^`/`$` match line boundaries)
/// - `x` — ignore unescaped whitespace in pattern
///
/// **Hard-fail discipline**: any flag character outside `{i, s, m, x}` — including
/// `q` (the XPath literal-match flag) — is a hard error. Silently ignoring `q`
/// would change matching semantics in ways the caller cannot detect. Consistent
/// with this crate's policy of hard-failing on any unmodelled SHACL feature, an
/// unsupported flag returns `Err` immediately.
///
/// **Deviation from XPath 2.0**: patterns are evaluated with Rust `regex` crate
/// semantics, not XPath 2.0 regex semantics. Behaviour diverges on features such
/// as Unicode category escapes (`\p{…}`) and backreferences.
fn build_regex(pattern: &str, flags: Option<&str>) -> Result<regex::Regex, String> {
    let mut builder = regex::RegexBuilder::new(pattern);
    if let Some(f) = flags {
        for c in f.chars() {
            match c {
                'i' => {
                    builder.case_insensitive(true);
                }
                's' => {
                    builder.dot_matches_new_line(true);
                }
                'm' => {
                    builder.multi_line(true);
                }
                'x' => {
                    builder.ignore_whitespace(true);
                }
                _ => {
                    return Err(format!(
                        "unsupported sh:flags character {c:?} in sh:pattern \
                         (supported: i, s, m, x)"
                    ));
                }
            }
        }
    }
    builder.build().map_err(|e| e.to_string())
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::sync::{Arc, OnceLock};

    use super::*;
    use crate::data::IrDataGraph;
    use crate::report::Severity;
    use crate::term::{Literal, NamedNode};

    const EX: &str = "http://example.org/ns#";
    const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
    const RDF: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#";

    fn nn(iri: &str) -> Term {
        Term::NamedNode(NamedNode::new_unchecked(iri))
    }

    fn ex(local: &str) -> Term {
        nn(&format!("{EX}{local}"))
    }

    fn xsd_lit(value: &str, dt: &str) -> Term {
        Term::Literal(Literal::new_typed_literal(
            value,
            NamedNode::new_unchecked(format!("{XSD}{dt}")),
        ))
    }

    #[test]
    fn numeric_value_covers_all_derived_integer_datatypes() {
        // Regression (#906): `numeric_value` must read EVERY xsd numeric-derived
        // datatype, not just the primitives. The omission of the derived/unsigned
        // integers (e.g. xsd:nonNegativeInteger) made a faithful
        // `"1"^^xsd:nonNegativeInteger` value read as non-numeric and spuriously
        // violate sh:minInclusive/sh:maxInclusive — masked only while data
        // round-tripped through oxigraph's value-space-normalizing NT serializer.
        for dt in [
            "integer",
            "decimal",
            "double",
            "float",
            "long",
            "int",
            "short",
            "byte",
            "nonNegativeInteger",
            "positiveInteger",
            "nonPositiveInteger",
            "negativeInteger",
            "unsignedLong",
            "unsignedInt",
            "unsignedShort",
            "unsignedByte",
        ] {
            // nonPositive/negative datatypes accept a non-positive lexical; use "0"
            // for those, "1" otherwise — both must parse to a numeric value.
            let lexical = if dt.contains("nonPositive") || dt.starts_with("negative") {
                "0"
            } else {
                "1"
            };
            assert!(
                numeric_value(&xsd_lit(lexical, dt)).is_some(),
                "xsd:{dt} must be read as numeric"
            );
        }
        // A non-numeric typed literal stays non-numeric.
        assert!(numeric_value(&xsd_lit("x", "string")).is_none());
        // A plain IRI is never numeric.
        assert!(numeric_value(&ex("thing")).is_none());
    }

    fn shape_with(id: &str, constraints: Vec<Constraint>) -> Shape {
        Shape {
            id: ex(id),
            targets: vec![],
            constraints,
            property_shapes: vec![],
            severity: Severity::Violation,
            message: None,
            deactivated: false,
            box_roles: vec![],
        }
    }

    fn prop_shape(id: &str, path_iri: &str, constraints: Vec<Constraint>) -> Shape {
        use crate::shapes::Path;
        Shape {
            id: ex(id),
            targets: vec![],
            constraints: vec![],
            property_shapes: vec![PropertyShape {
                path: Path::Predicate(NamedNode::new_unchecked(path_iri)),
                constraints,
                reifier_shapes: vec![],
                reification_required: false,
                severity: Severity::Violation,
                message: None,
                box_roles: vec![],
            }],
            severity: Severity::Violation,
            message: None,
            deactivated: false,
            box_roles: vec![],
        }
    }

    fn load_store(ttl: &str) -> IrDataGraph {
        let dataset = crate::text_ingest::parse_turtle_to_dataset(ttl).expect("Turtle parse");
        // Apply the same SHACL projection `validate_dataset` uses, so RDF-1.2
        // reifier bindings are materialized as `rdf:reifies` quads the engine's
        // reifier-shape lookup can find (the IR keeps reifiers in a side table).
        let projected =
            crate::engine::shacl_dataset_from_dataset(&dataset).expect("SHACL projection");
        IrDataGraph::new(projected)
    }

    fn component_iri(results: &[ValidationResult]) -> Vec<String> {
        results
            .iter()
            .map(|r| r.source_constraint_component.as_str().to_owned())
            .collect()
    }

    fn role_iris(roles: &[NamedNode]) -> Vec<&str> {
        roles
            .iter()
            .map(super::super::term::NamedNode::as_str)
            .collect()
    }

    fn named_role(role: &str) -> NamedNode {
        NamedNode::from(role)
    }

    // ── minCount ───────────────────────────────────────────────────────────────

    #[test]
    fn min_count_pass() {
        let store = load_store("@prefix ex: <http://example.org/ns#> . ex:a ex:p ex:b .");
        let shape = prop_shape("S", &format!("{EX}p"), vec![Constraint::MinCount(1)]);
        let results = validate_shape(&store, &ex("a"), &shape);
        assert!(results.is_empty(), "should pass with 1 value");
    }

    #[test]
    fn min_count_fail() {
        let store = load_store("@prefix ex: <http://example.org/ns#> . ex:a a ex:Thing .");
        let shape = prop_shape("S", &format!("{EX}p"), vec![Constraint::MinCount(1)]);
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("MinCount"));
    }

    #[test]
    fn property_shape_box_roles_augment_parent_roles() {
        use crate::model::purrdf;
        use crate::shapes::Path;

        let store = load_store(&format!(
            "@prefix ex: <{EX}> .\n\
             @prefix purrdf: <https://blackcatinformatics.ca/purrdf/> .\n\
             ex:p purrdf:graphBoxRole purrdf:boxRBox .\n\
             ex:a a ex:Thing .\n"
        ));
        let shape = Shape {
            id: ex("S"),
            targets: vec![],
            constraints: vec![],
            property_shapes: vec![PropertyShape {
                path: Path::Predicate(NamedNode::new_unchecked(format!("{EX}p"))),
                constraints: vec![Constraint::MinCount(1)],
                reifier_shapes: vec![],
                reification_required: false,
                severity: Severity::Violation,
                message: None,
                box_roles: vec![named_role(purrdf::BOX_CONFIG_BOX)],
            }],
            severity: Severity::Violation,
            message: None,
            deactivated: false,
            box_roles: vec![named_role(purrdf::BOX_TBOX)],
        };

        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        let source_roles = role_iris(&results[0].source_box_roles);
        assert!(source_roles.contains(&purrdf::BOX_TBOX));
        assert!(source_roles.contains(&purrdf::BOX_CONFIG_BOX));
        assert_eq!(role_iris(&results[0].path_box_roles), [purrdf::BOX_RBOX]);
        let result_roles = role_iris(&results[0].result_box_roles);
        assert!(result_roles.contains(&purrdf::BOX_TBOX));
        assert!(result_roles.contains(&purrdf::BOX_CONFIG_BOX));
        assert!(result_roles.contains(&purrdf::BOX_RBOX));
    }

    #[test]
    fn reifier_shape_box_roles_preserve_inner_roles() {
        use crate::model::purrdf;
        use crate::shapes::Path;

        let store = load_store(&format!(
            "@prefix ex: <{EX}> .\n\
             @prefix rdf: <{RDF}> .\n\
             ex:a ex:p ex:b .\n\
             ex:reifier rdf:reifies <<( ex:a ex:p ex:b )>> .\n"
        ));
        let reifier_shape = Shape {
            id: ex("ReifierShape"),
            targets: vec![],
            constraints: vec![Constraint::Class(NamedNode::new_unchecked(format!(
                "{EX}RequiredReifierClass"
            )))],
            property_shapes: vec![],
            severity: Severity::Violation,
            message: None,
            deactivated: false,
            box_roles: vec![named_role(purrdf::BOX_CONFIG_BOX)],
        };
        let shape = Shape {
            id: ex("S"),
            targets: vec![],
            constraints: vec![],
            property_shapes: vec![PropertyShape {
                path: Path::Predicate(NamedNode::new_unchecked(format!("{EX}p"))),
                constraints: vec![],
                reifier_shapes: vec![reifier_shape],
                reification_required: false,
                severity: Severity::Violation,
                message: None,
                box_roles: vec![],
            }],
            severity: Severity::Violation,
            message: None,
            deactivated: false,
            box_roles: vec![named_role(purrdf::BOX_TBOX)],
        };

        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("ReifierShapeConstraintComponent"));
        let source_roles = role_iris(&results[0].source_box_roles);
        assert!(source_roles.contains(&purrdf::BOX_TBOX));
        assert!(source_roles.contains(&purrdf::BOX_CBOX));
        assert!(source_roles.contains(&purrdf::BOX_CONFIG_BOX));
        let result_roles = role_iris(&results[0].result_box_roles);
        assert!(result_roles.contains(&purrdf::BOX_TBOX));
        assert!(result_roles.contains(&purrdf::BOX_CBOX));
        assert!(result_roles.contains(&purrdf::BOX_CONFIG_BOX));
    }

    // ── maxCount ───────────────────────────────────────────────────────────────

    #[test]
    fn max_count_pass() {
        let store = load_store("@prefix ex: <http://example.org/ns#> . ex:a ex:p ex:b .");
        let shape = prop_shape("S", &format!("{EX}p"), vec![Constraint::MaxCount(1)]);
        let results = validate_shape(&store, &ex("a"), &shape);
        assert!(results.is_empty());
    }

    #[test]
    fn max_count_fail() {
        let store = load_store("@prefix ex: <http://example.org/ns#> . ex:a ex:p ex:b, ex:c .");
        let shape = prop_shape("S", &format!("{EX}p"), vec![Constraint::MaxCount(1)]);
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("MaxCount"));
    }

    // ── class ──────────────────────────────────────────────────────────────────

    #[test]
    fn class_pass() {
        let store = load_store(
            "@prefix ex: <http://example.org/ns#> . @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> . ex:a ex:p ex:b . ex:b rdf:type ex:Foo .",
        );
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::Class(NamedNode::new_unchecked(format!(
                "{EX}Foo"
            )))],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert!(results.is_empty());
    }

    #[test]
    fn class_fail_no_direct_type() {
        // ex:b is typed ex:SubFoo, and there is NO asserted ex:SubFoo
        // rdfs:subClassOf ex:Foo triple in the data — so b is not a SHACL
        // instance of ex:Foo and the constraint fails. (We honor asserted
        // subClassOf, but invent none: no reasoner runs.)
        let store = load_store(
            "@prefix ex: <http://example.org/ns#> . @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> . ex:a ex:p ex:b . ex:b rdf:type ex:SubFoo .",
        );
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::Class(NamedNode::new_unchecked(format!(
                "{EX}Foo"
            )))],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("Class"));
    }

    #[test]
    fn class_pass_asserted_subclass() {
        // ex:b is typed ex:SubFoo and the data ASSERTS ex:SubFoo rdfs:subClassOf
        // ex:Foo, so b is a SHACL instance of ex:Foo (SHACL §4.2.5) and the
        // sh:class ex:Foo constraint conforms — matching pySHACL. See #599.
        let store = load_store(
            "@prefix ex: <http://example.org/ns#> . @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> . @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> . ex:a ex:p ex:b . ex:b rdf:type ex:SubFoo . ex:SubFoo rdfs:subClassOf ex:Foo .",
        );
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::Class(NamedNode::new_unchecked(format!(
                "{EX}Foo"
            )))],
        );
        assert!(
            validate_shape(&store, &ex("a"), &shape).is_empty(),
            "asserted subClassOf must make ex:b a SHACL instance of ex:Foo"
        );
    }

    #[test]
    fn class_pass_transitive_subclass() {
        // Transitive: ex:b a ex:C, ex:C ⊑ ex:B, ex:B ⊑ ex:A → b is an A-instance.
        let store = load_store(
            "@prefix ex: <http://example.org/ns#> . @prefix rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#> . @prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> . ex:a ex:p ex:b . ex:b rdf:type ex:C . ex:C rdfs:subClassOf ex:B . ex:B rdfs:subClassOf ex:A .",
        );
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::Class(NamedNode::new_unchecked(format!(
                "{EX}A"
            )))],
        );
        assert!(
            validate_shape(&store, &ex("a"), &shape).is_empty(),
            "transitive asserted subClassOf must be honored"
        );
    }

    // ── datatype ───────────────────────────────────────────────────────────────

    #[test]
    fn datatype_pass() {
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:age \"42\"^^<{XSD}integer> ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}age"),
            vec![Constraint::Datatype(NamedNode::new_unchecked(format!(
                "{XSD}integer"
            )))],
        );
        assert!(validate_shape(&store, &ex("a"), &shape).is_empty());
    }

    #[test]
    fn datatype_fail_wrong_type() {
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:age \"hello\"^^<{XSD}string> ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}age"),
            vec![Constraint::Datatype(NamedNode::new_unchecked(format!(
                "{XSD}integer"
            )))],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("Datatype"));
    }

    #[test]
    fn datatype_fail_lexically_invalid() {
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:n \"notanumber\"^^<{XSD}integer> ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}n"),
            vec![Constraint::Datatype(NamedNode::new_unchecked(format!(
                "{XSD}integer"
            )))],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("Datatype"));
    }

    // ── datatype derived-integer (oxigraph canonicalization, #598) ──────────────

    #[test]
    fn datatype_derived_nonneg_integer_pass() {
        // Oxigraph stores "5"^^xsd:nonNegativeInteger as "5"^^xsd:integer, but a
        // shape requiring xsd:nonNegativeInteger must still accept it (value 5 is
        // in range) — matching pySHACL. Pre-fix this produced a false violation.
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:n \"5\"^^<{XSD}nonNegativeInteger> ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}n"),
            vec![Constraint::Datatype(NamedNode::new_unchecked(format!(
                "{XSD}nonNegativeInteger"
            )))],
        );
        assert!(
            validate_shape(&store, &ex("a"), &shape).is_empty(),
            "in-range derived-integer value must conform under canonicalization"
        );
    }

    #[test]
    fn derived_integer_value_space() {
        let int = "http://www.w3.org/2001/XMLSchema#integer";
        let nn = "http://www.w3.org/2001/XMLSchema#nonNegativeInteger";
        let pos = "http://www.w3.org/2001/XMLSchema#positiveInteger";
        let neg = "http://www.w3.org/2001/XMLSchema#negativeInteger";
        let byte = "http://www.w3.org/2001/XMLSchema#byte";
        // nonNegativeInteger: >= 0
        assert!(derived_integer_matches(int, nn, "5"));
        assert!(derived_integer_matches(int, nn, "0"));
        assert!(!derived_integer_matches(int, nn, "-3"));
        // positiveInteger: > 0 (zero excluded)
        assert!(derived_integer_matches(int, pos, "1"));
        assert!(!derived_integer_matches(int, pos, "0"));
        // negativeInteger: < 0
        assert!(derived_integer_matches(int, neg, "-2"));
        assert!(!derived_integer_matches(int, neg, "0"));
        // byte: -128..=127
        assert!(derived_integer_matches(int, byte, "127"));
        assert!(!derived_integer_matches(int, byte, "128"));
        // only the xsd:integer base is the canonical fold target; a non-integer
        // stored type or a non-numeric lexical form never matches a derived type.
        assert!(!derived_integer_matches(
            "http://www.w3.org/2001/XMLSchema#string",
            nn,
            "5"
        ));
        assert!(!derived_integer_matches(int, nn, "x"));
    }

    // ── nodeKind ───────────────────────────────────────────────────────────────

    #[test]
    fn node_kind_iri_pass() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:p ex:b ."));
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::NodeKind(NodeKindValue::Iri)],
        );
        assert!(validate_shape(&store, &ex("a"), &shape).is_empty());
    }

    #[test]
    fn node_kind_iri_fail_literal() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:p \"hello\" ."));
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::NodeKind(NodeKindValue::Iri)],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("NodeKind"));
    }

    // ── in ─────────────────────────────────────────────────────────────────────

    #[test]
    fn in_pass() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:color \"red\" ."));
        let shape = prop_shape(
            "S",
            &format!("{EX}color"),
            vec![Constraint::In(vec![
                Term::Literal(Literal::new_simple_literal("red")),
                Term::Literal(Literal::new_simple_literal("green")),
            ])],
        );
        assert!(validate_shape(&store, &ex("a"), &shape).is_empty());
    }

    #[test]
    fn in_fail() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:color \"blue\" ."));
        let shape = prop_shape(
            "S",
            &format!("{EX}color"),
            vec![Constraint::In(vec![
                Term::Literal(Literal::new_simple_literal("red")),
                Term::Literal(Literal::new_simple_literal("green")),
            ])],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("In"));
    }

    // ── hasValue ───────────────────────────────────────────────────────────────

    #[test]
    fn has_value_pass() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:p ex:b, ex:c ."));
        let shape = prop_shape("S", &format!("{EX}p"), vec![Constraint::HasValue(ex("b"))]);
        assert!(validate_shape(&store, &ex("a"), &shape).is_empty());
    }

    #[test]
    fn has_value_fail() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:p ex:c ."));
        let shape = prop_shape("S", &format!("{EX}p"), vec![Constraint::HasValue(ex("b"))]);
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("HasValue"));
    }

    // ── pattern ────────────────────────────────────────────────────────────────

    #[test]
    fn pattern_pass() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:code \"ABC\" ."));
        let shape = prop_shape(
            "S",
            &format!("{EX}code"),
            vec![Constraint::Pattern {
                regex: "^[A-Z]+$".to_owned(),
                flags: None,
                compiled: Arc::new(OnceLock::new()),
            }],
        );
        assert!(validate_shape(&store, &ex("a"), &shape).is_empty());
    }

    #[test]
    fn pattern_fail() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:code \"abc\" ."));
        let shape = prop_shape(
            "S",
            &format!("{EX}code"),
            vec![Constraint::Pattern {
                regex: "^[A-Z]+$".to_owned(),
                flags: None,
                compiled: Arc::new(OnceLock::new()),
            }],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("Pattern"));
    }

    #[test]
    fn pattern_with_flags_case_insensitive() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:code \"abc\" ."));
        let shape = prop_shape(
            "S",
            &format!("{EX}code"),
            vec![Constraint::Pattern {
                regex: "^[A-Z]+$".to_owned(),
                flags: Some("i".to_owned()),
                compiled: Arc::new(OnceLock::new()),
            }],
        );
        // With flag "i", lowercase should now pass.
        assert!(validate_shape(&store, &ex("a"), &shape).is_empty());
    }

    // ── build_regex ────────────────────────────────────────────────────────────

    #[test]
    fn build_regex_rejects_unknown_flag() {
        // 'q' (XPath literal-match flag) is unsupported — must hard-fail.
        assert!(
            build_regex("foo", Some("q")).is_err(),
            "build_regex should reject unknown flag 'q'"
        );
        // Verify the error message identifies the offending character.
        let err = build_regex("foo", Some("q")).unwrap_err();
        assert!(
            err.contains('q'),
            "error message should mention the rejected flag character"
        );
    }

    #[test]
    fn build_regex_accepts_supported_flags() {
        // All four supported flags must compile without error.
        assert!(
            build_regex("foo", Some("i")).is_ok(),
            "flag 'i' should be accepted"
        );
        assert!(
            build_regex("foo", Some("s")).is_ok(),
            "flag 's' should be accepted"
        );
        assert!(
            build_regex("foo", Some("m")).is_ok(),
            "flag 'm' should be accepted"
        );
        assert!(
            build_regex("foo", Some("x")).is_ok(),
            "flag 'x' should be accepted"
        );
        assert!(
            build_regex("foo", Some("ismx")).is_ok(),
            "combined flags should be accepted"
        );
    }

    // ── minLength ──────────────────────────────────────────────────────────────

    #[test]
    fn min_length_pass() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:name \"Alice\" ."));
        let shape = prop_shape("S", &format!("{EX}name"), vec![Constraint::MinLength(3)]);
        assert!(validate_shape(&store, &ex("a"), &shape).is_empty());
    }

    #[test]
    fn min_length_fail() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:name \"Al\" ."));
        let shape = prop_shape("S", &format!("{EX}name"), vec![Constraint::MinLength(3)]);
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("MinLength"));
    }

    // ── uniqueLang ─────────────────────────────────────────────────────────────

    #[test]
    fn unique_lang_pass() {
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:label \"Hello\"@en, \"Bonjour\"@fr ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}label"),
            vec![Constraint::UniqueLang(true)],
        );
        assert!(validate_shape(&store, &ex("a"), &shape).is_empty());
    }

    #[test]
    fn unique_lang_fail() {
        // Load two English-tagged literals via N-Triples (Turtle deduplicates in the store).
        let nt = format!("<{EX}a> <{EX}label> \"Hello\"@en .\n<{EX}a> <{EX}label> \"Hi\"@en .\n");
        let store = IrDataGraph::new(
            crate::text_ingest::parse_ntriples_to_dataset(&nt).expect("N-Triples parse"),
        );
        let shape = prop_shape(
            "S",
            &format!("{EX}label"),
            vec![Constraint::UniqueLang(true)],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert!(!results.is_empty());
        assert!(component_iri(&results)[0].contains("UniqueLang"));
    }

    // ── minInclusive ───────────────────────────────────────────────────────────

    #[test]
    fn min_inclusive_pass() {
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:age \"18\"^^<{XSD}integer> ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}age"),
            vec![Constraint::MinInclusive(xsd_lit("18", "integer"))],
        );
        assert!(validate_shape(&store, &ex("a"), &shape).is_empty());
    }

    #[test]
    fn min_inclusive_fail() {
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:age \"17\"^^<{XSD}integer> ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}age"),
            vec![Constraint::MinInclusive(xsd_lit("18", "integer"))],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("MinInclusive"));
    }

    // ── maxInclusive ───────────────────────────────────────────────────────────

    #[test]
    fn max_inclusive_pass() {
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:score \"100\"^^<{XSD}integer> ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}score"),
            vec![Constraint::MaxInclusive(xsd_lit("100", "integer"))],
        );
        assert!(validate_shape(&store, &ex("a"), &shape).is_empty());
    }

    #[test]
    fn max_inclusive_fail() {
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:score \"101\"^^<{XSD}integer> ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}score"),
            vec![Constraint::MaxInclusive(xsd_lit("100", "integer"))],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("MaxInclusive"));
    }

    // ── minExclusive ─────────────────────────────────────────────────────────────

    #[test]
    fn min_exclusive_pass() {
        // The bound is exclusive: 19 > 18 passes.
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:age \"19\"^^<{XSD}integer> ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}age"),
            vec![Constraint::MinExclusive(xsd_lit("18", "integer"))],
        );
        assert!(validate_shape(&store, &ex("a"), &shape).is_empty());
    }

    #[test]
    fn min_exclusive_fail_on_equal() {
        // Equal to the bound must FAIL under sh:minExclusive (unlike minInclusive).
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:age \"18\"^^<{XSD}integer> ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}age"),
            vec![Constraint::MinExclusive(xsd_lit("18", "integer"))],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("MinExclusive"));
    }

    // ── maxExclusive ─────────────────────────────────────────────────────────────

    #[test]
    fn max_exclusive_pass() {
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:score \"99\"^^<{XSD}integer> ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}score"),
            vec![Constraint::MaxExclusive(xsd_lit("100", "integer"))],
        );
        assert!(validate_shape(&store, &ex("a"), &shape).is_empty());
    }

    #[test]
    fn max_exclusive_fail_on_equal() {
        // Equal to the bound must FAIL under sh:maxExclusive.
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:score \"100\"^^<{XSD}integer> ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}score"),
            vec![Constraint::MaxExclusive(xsd_lit("100", "integer"))],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("MaxExclusive"));
    }

    // ── and ────────────────────────────────────────────────────────────────────

    #[test]
    fn and_pass() {
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . @prefix rdf: <{RDF}> . ex:a rdf:type ex:Foo ."
        ));
        // sh:and ([ sh:nodeKind sh:IRI ] [ sh:class ex:Foo ]) on focus node directly.
        let member1 = shape_with("M1", vec![Constraint::NodeKind(NodeKindValue::Iri)]);
        let member2 = shape_with(
            "M2",
            vec![Constraint::Class(NamedNode::new_unchecked(format!(
                "{EX}Foo"
            )))],
        );
        let shape = shape_with("S", vec![Constraint::And(vec![member1, member2])]);
        assert!(validate_shape(&store, &ex("a"), &shape).is_empty());
    }

    #[test]
    fn and_fail_second_member() {
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . @prefix rdf: <{RDF}> . ex:a rdf:type ex:Bar ."
        ));
        // ex:a is IRI (passes M1) but type is ex:Bar not ex:Foo (fails M2).
        let member1 = shape_with("M1", vec![Constraint::NodeKind(NodeKindValue::Iri)]);
        let member2 = shape_with(
            "M2",
            vec![Constraint::Class(NamedNode::new_unchecked(format!(
                "{EX}Foo"
            )))],
        );
        let shape = shape_with("S", vec![Constraint::And(vec![member1, member2])]);
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("And"));
    }

    // ── or ─────────────────────────────────────────────────────────────────────

    #[test]
    fn or_pass_first_member() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:p ex:b ."));
        // ex:b is an IRI, passes M1.
        let member1 = shape_with("M1", vec![Constraint::NodeKind(NodeKindValue::Iri)]);
        let member2 = shape_with("M2", vec![Constraint::NodeKind(NodeKindValue::Literal)]);
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::Or(vec![member1, member2])],
        );
        assert!(validate_shape(&store, &ex("a"), &shape).is_empty());
    }

    #[test]
    fn or_fail_no_member() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:p ex:b ."));
        // Both members require Literal; ex:b is IRI → fails both.
        let member1 = shape_with("M1", vec![Constraint::NodeKind(NodeKindValue::Literal)]);
        let member2 = shape_with(
            "M2",
            vec![Constraint::MinLength(999)], // impossible length
        );
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::Or(vec![member1, member2])],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("Or"));
    }

    // ── xone ───────────────────────────────────────────────────────────────────

    #[test]
    fn xone_pass_exactly_one() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:p ex:b ."));
        // ex:b is IRI: M1 (IRI) passes, M2 (Literal) fails → exactly 1.
        let member1 = shape_with("M1", vec![Constraint::NodeKind(NodeKindValue::Iri)]);
        let member2 = shape_with("M2", vec![Constraint::NodeKind(NodeKindValue::Literal)]);
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::Xone(vec![member1, member2])],
        );
        assert!(validate_shape(&store, &ex("a"), &shape).is_empty());
    }

    #[test]
    fn xone_fail_zero() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:p \"hello\" ."));
        // Both require IRI; literal fails both → 0 conforming → violation.
        let member1 = shape_with("M1", vec![Constraint::NodeKind(NodeKindValue::Iri)]);
        let member2 = shape_with("M2", vec![Constraint::NodeKind(NodeKindValue::Iri)]);
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::Xone(vec![member1, member2])],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("Xone"));
    }

    #[test]
    fn xone_fail_two() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:p ex:b ."));
        // Both members allow IRI → 2 conforming → violation.
        let member1 = shape_with("M1", vec![Constraint::NodeKind(NodeKindValue::Iri)]);
        let member2 = shape_with("M2", vec![Constraint::NodeKind(NodeKindValue::Iri)]);
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::Xone(vec![member1, member2])],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("Xone"));
    }

    // ── node ───────────────────────────────────────────────────────────────────

    #[test]
    fn node_pass() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:p ex:b ."));
        // sh:node targets ex:b; inner shape requires IRI.
        let inner = shape_with("Inner", vec![Constraint::NodeKind(NodeKindValue::Iri)]);
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::Node(Box::new(inner))],
        );
        assert!(validate_shape(&store, &ex("a"), &shape).is_empty());
    }

    #[test]
    fn node_fail() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:p \"notAnIRI\" ."));
        let inner = shape_with("Inner", vec![Constraint::NodeKind(NodeKindValue::Iri)]);
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::Node(Box::new(inner))],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("NodeConstraintComponent"));
    }

    // ── inverse path property shape ────────────────────────────────────────────

    #[test]
    fn inverse_path_property_shape() {
        use crate::shapes::Path;
        // ex:child ex:parent ex:parent_node .
        // Shape on ex:parent_node checks inverse(ex:parent) has minCount 1.
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:child ex:parent ex:parent_node ."
        ));
        let shape = Shape {
            id: ex("S"),
            targets: vec![],
            constraints: vec![],
            property_shapes: vec![PropertyShape {
                path: Path::Inverse(Box::new(Path::Predicate(NamedNode::new_unchecked(
                    format!("{EX}parent"),
                )))),
                constraints: vec![Constraint::MinCount(1)],
                reifier_shapes: vec![],
                reification_required: false,
                severity: Severity::Violation,
                message: None,
                box_roles: vec![],
            }],
            severity: Severity::Violation,
            message: None,
            deactivated: false,
            box_roles: vec![],
        };
        // ex:parent_node has 1 inverse-parent (ex:child) → passes minCount(1).
        let results = validate_shape(&store, &ex("parent_node"), &shape);
        assert!(results.is_empty(), "expected pass, got: {results:?}");
    }

    #[test]
    fn inverse_path_property_shape_fail() {
        use crate::shapes::Path;
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:unrelated ex:something ex:other ."
        ));
        let shape = Shape {
            id: ex("S"),
            targets: vec![],
            constraints: vec![],
            property_shapes: vec![PropertyShape {
                path: Path::Inverse(Box::new(Path::Predicate(NamedNode::new_unchecked(
                    format!("{EX}parent"),
                )))),
                constraints: vec![Constraint::MinCount(1)],
                reifier_shapes: vec![],
                reification_required: false,
                severity: Severity::Violation,
                message: None,
                box_roles: vec![],
            }],
            severity: Severity::Violation,
            message: None,
            deactivated: false,
            box_roles: vec![],
        };
        // ex:orphan has no inverse-parent triples → fails minCount(1).
        let results = validate_shape(&store, &ex("orphan"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("MinCount"));
    }

    // ── xsd lexical validators (Gap D fix) ────────────────────────────────────

    #[test]
    fn xsd_integer_accepts_large_value() {
        // A valid xsd:integer beyond i64::MAX must PASS (no overflow rejection).
        let dt_iri = NamedNode::new_unchecked(format!("{XSD}integer"));
        let value = Term::Literal(Literal::new_typed_literal(
            "99999999999999999999999",
            dt_iri.clone(),
        ));
        assert!(
            check_datatype(&value, &dt_iri),
            "large integer should conform"
        );
    }

    #[test]
    fn xsd_integer_rejects_decimal_point() {
        // "3.5"^^xsd:integer is lexically invalid.
        let dt_iri = NamedNode::new_unchecked(format!("{XSD}integer"));
        let value = Term::Literal(Literal::new_typed_literal("3.5", dt_iri.clone()));
        assert!(
            !check_datatype(&value, &dt_iri),
            "decimal point in integer should violate"
        );
    }

    #[test]
    fn xsd_decimal_rejects_scientific_notation() {
        // "1e3"^^xsd:decimal is NOT a valid xsd:decimal lexical form.
        let dt_iri = NamedNode::new_unchecked(format!("{XSD}decimal"));
        let value = Term::Literal(Literal::new_typed_literal("1e3", dt_iri.clone()));
        assert!(
            !check_datatype(&value, &dt_iri),
            "scientific notation should violate xsd:decimal"
        );
    }

    #[test]
    fn xsd_decimal_accepts_plain() {
        // "3.14"^^xsd:decimal is valid.
        let dt_iri = NamedNode::new_unchecked(format!("{XSD}decimal"));
        let value = Term::Literal(Literal::new_typed_literal("3.14", dt_iri.clone()));
        assert!(
            check_datatype(&value, &dt_iri),
            "plain decimal should conform"
        );
    }

    #[test]
    fn xsd_double_accepts_scientific() {
        // "1e3"^^xsd:double is valid (scientific notation is allowed for double).
        let dt_iri = NamedNode::new_unchecked(format!("{XSD}double"));
        let value = Term::Literal(Literal::new_typed_literal("1e3", dt_iri.clone()));
        assert!(
            check_datatype(&value, &dt_iri),
            "scientific notation should conform for xsd:double"
        );
    }

    #[test]
    fn xsd_double_accepts_inf() {
        // "INF"^^xsd:double is a valid XSD special value.
        let dt_iri = NamedNode::new_unchecked(format!("{XSD}double"));
        let value = Term::Literal(Literal::new_typed_literal("INF", dt_iri.clone()));
        assert!(
            check_datatype(&value, &dt_iri),
            "INF should conform for xsd:double"
        );
    }

    #[test]
    fn xsd_double_rejects_plus_inf() {
        // "+INF" is NOT in the xsd:double/float lexical space (only INF, -INF, NaN).
        let dt_iri = NamedNode::new_unchecked(format!("{XSD}double"));
        let value = Term::Literal(Literal::new_typed_literal("+INF", dt_iri.clone()));
        assert!(
            !check_datatype(&value, &dt_iri),
            "+INF must not conform for xsd:double"
        );
    }

    #[test]
    fn xsd_float_accepts_scientific() {
        // "1e3"^^xsd:float is valid — same lexical space as double.
        let dt_iri = NamedNode::new_unchecked(format!("{XSD}float"));
        let value = Term::Literal(Literal::new_typed_literal("1e3", dt_iri.clone()));
        assert!(
            check_datatype(&value, &dt_iri),
            "scientific notation should conform for xsd:float"
        );
    }

    // ── deactivated shape ──────────────────────────────────────────────────────

    #[test]
    fn deactivated_shape_produces_no_results() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:p \"hello\" ."));
        // Would fail NodeKind(Iri) if active.
        let shape = Shape {
            id: ex("S"),
            targets: vec![],
            constraints: vec![Constraint::NodeKind(NodeKindValue::Iri)],
            property_shapes: vec![],
            severity: Severity::Violation,
            message: None,
            deactivated: true,
            box_roles: vec![],
        };
        // Focus node is a literal — would fail, but shape is deactivated.
        let literal_focus = Term::Literal(Literal::new_simple_literal("anything"));
        assert!(validate_shape(&store, &literal_focus, &shape).is_empty());
    }

    // ── #700: maxLength ────────────────────────────────────────────────────────

    #[test]
    fn max_length_pass() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:p \"abc\" ."));
        let shape = prop_shape("S", &format!("{EX}p"), vec![Constraint::MaxLength(5)]);
        let results = validate_shape(&store, &ex("a"), &shape);
        assert!(results.is_empty(), "\"abc\" (len 3) ≤ 5 must pass");
    }

    #[test]
    fn max_length_fail() {
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:p \"abcdef\" ."));
        let shape = prop_shape("S", &format!("{EX}p"), vec![Constraint::MaxLength(5)]);
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("MaxLength"));
    }

    // ── #700: languageIn ───────────────────────────────────────────────────────

    #[test]
    fn language_in_pass_prefix_match() {
        // "hello"@en-US matches the entry "en" by basic-filtering prefix match.
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:p \"hello\"@en-US ."));
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::LanguageIn(vec!["en".into(), "fr".into()])],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert!(results.is_empty(), "en-US must match entry \"en\"");
    }

    #[test]
    fn language_in_fail_unlisted_and_untagged() {
        // "guten"@de is not in the list → violation; "plain" has no tag → violation.
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:p \"guten\"@de , \"plain\" ."
        ));
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::LanguageIn(vec!["en".into(), "fr".into()])],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(
            results.len(),
            2,
            "both the de literal and the untagged literal violate"
        );
        assert!(component_iri(&results)[0].contains("LanguageIn"));
    }

    // ── #700: not ──────────────────────────────────────────────────────────────

    #[test]
    fn not_pass_when_inner_violated() {
        // Inner shape requires NodeKind(Iri); the value is a literal, so it does
        // NOT conform → sh:not is satisfied.
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:p \"lit\" ."));
        let inner = shape_with("Inner", vec![Constraint::NodeKind(NodeKindValue::Iri)]);
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::Not(Box::new(inner))],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert!(
            results.is_empty(),
            "literal does not conform to inner ⇒ not() passes"
        );
    }

    #[test]
    fn not_fail_when_inner_conforms() {
        // Inner shape requires NodeKind(Iri); the value IS an IRI, so it conforms
        // → sh:not is violated.
        let store = load_store(&format!("@prefix ex: <{EX}> . ex:a ex:p ex:b ."));
        let inner = shape_with("Inner", vec![Constraint::NodeKind(NodeKindValue::Iri)]);
        let shape = prop_shape(
            "S",
            &format!("{EX}p"),
            vec![Constraint::Not(Box::new(inner))],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1);
        assert!(component_iri(&results)[0].contains("NotConstraintComponent"));
    }

    // ── #700: closed ───────────────────────────────────────────────────────────

    fn closed_shape(ignored: Vec<NamedNode>, path_iris: &[&str]) -> Shape {
        use crate::shapes::Path;
        let property_shapes = path_iris
            .iter()
            .map(|p| PropertyShape {
                path: Path::Predicate(NamedNode::new_unchecked(*p)),
                constraints: vec![],
                reifier_shapes: vec![],
                reification_required: false,
                severity: Severity::Violation,
                message: None,
                box_roles: vec![],
            })
            .collect();
        Shape {
            id: ex("S"),
            targets: vec![],
            constraints: vec![Constraint::Closed { ignored }],
            property_shapes,
            severity: Severity::Violation,
            message: None,
            deactivated: false,
            box_roles: vec![],
        }
    }

    #[test]
    fn closed_pass_only_declared_predicates() {
        // ex:a uses only ex:name (declared) and rdf:type (always allowed).
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . @prefix rdf: <{RDF}> . ex:a a ex:Person ; ex:name \"Al\" ."
        ));
        let shape = closed_shape(vec![], &[&format!("{EX}name")]);
        let results = validate_shape(&store, &ex("a"), &shape);
        assert!(
            results.is_empty(),
            "only declared/rdf:type predicates ⇒ pass"
        );
    }

    #[test]
    fn closed_fail_extra_predicate() {
        // ex:a also uses ex:age, which is neither declared nor ignored.
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:name \"Al\" ; ex:age 30 ."
        ));
        let shape = closed_shape(vec![], &[&format!("{EX}name")]);
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1, "ex:age is an undeclared predicate");
        assert!(component_iri(&results)[0].contains("ClosedConstraintComponent"));
        assert_eq!(
            results[0].result_path.as_ref().map(ToString::to_string),
            Some(format!("<{EX}age>"))
        );
    }

    #[test]
    fn closed_violation_carries_predicate_box_roles() {
        use crate::model::purrdf;
        // The offending (undeclared) predicate ex:age declares a graph-box role;
        // the closed-world result must carry it as PATH attribution — closed
        // violations previously dropped predicate roles (#700 Gap B).
        let store = load_store(&format!(
            "@prefix ex: <{EX}> .\n\
             @prefix purrdf: <https://blackcatinformatics.ca/purrdf/> .\n\
             ex:age purrdf:graphBoxRole purrdf:boxRBox .\n\
             ex:a ex:name \"Al\" ; ex:age 30 .\n"
        ));
        let shape = closed_shape(vec![], &[&format!("{EX}name")]);
        let results = validate_shape(&store, &ex("a"), &shape);
        assert_eq!(results.len(), 1, "ex:age is an undeclared predicate");
        assert_eq!(
            role_iris(&results[0].path_box_roles),
            [purrdf::BOX_RBOX],
            "closed-world violation must carry the offending predicate's box roles"
        );
        assert!(
            role_iris(&results[0].result_box_roles).contains(&purrdf::BOX_RBOX),
            "merged result roles must include the predicate's path role"
        );
    }

    #[test]
    fn closed_pass_ignored_predicate() {
        // ex:age is undeclared but listed in sh:ignoredProperties ⇒ allowed.
        let store = load_store(&format!(
            "@prefix ex: <{EX}> . ex:a ex:name \"Al\" ; ex:age 30 ."
        ));
        let shape = closed_shape(
            vec![NamedNode::new_unchecked(format!("{EX}age"))],
            &[&format!("{EX}name")],
        );
        let results = validate_shape(&store, &ex("a"), &shape);
        assert!(results.is_empty(), "ignored predicate ex:age ⇒ pass");
    }
}

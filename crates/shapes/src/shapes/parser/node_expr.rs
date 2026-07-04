// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SHACL Core constraint parsing and SHACL-AF node-expression parsing.

use ::purrdf::FastSet;
use std::sync::{Arc, OnceLock};

use purrdf_sparql_algebra::{Query, SparqlParser};

use crate::components::{severity_from_term, Component, Validator, ValidatorKind};
use crate::data::{native_quads, GraphFilter};
use crate::expression::{FnCall, NodeExpr};
use crate::model::{rdf, sh};
use crate::term::{NamedNode, Term};

use crate::shapes::{ComponentValidator, Constraint, NodeKindValue, Parser, Shape};

impl Parser<'_> {
    /// Parse all constraints declared directly on a shape node.
    ///
    /// Does NOT include `sh:property` sub-shapes (handled separately).
    /// `is_property_shape` selects the right custom-component validator
    /// (`sh:propertyValidator` vs `sh:nodeValidator`) and is passed down from
    /// both node shapes and property shapes.
    pub(crate) fn parse_constraints(
        &mut self,
        id: &Term,
        is_property_shape: bool,
    ) -> Result<Vec<Constraint>, String> {
        let mut constraints: Vec<Constraint> = Vec::new();

        // sh:class — sorted for determinism
        let mut classes: Vec<NamedNode> = self
            .objects_of(id, sh::CLASS)
            .into_iter()
            .filter_map(|t| match t {
                Term::NamedNode(n) => Some(n),
                _ => None,
            })
            .collect();
        classes.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        for n in classes {
            constraints.push(Constraint::Class(n));
        }

        // sh:datatype
        let mut datatypes: Vec<NamedNode> = self
            .objects_of(id, sh::DATATYPE)
            .into_iter()
            .filter_map(|t| match t {
                Term::NamedNode(n) => Some(n),
                _ => None,
            })
            .collect();
        datatypes.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        for n in datatypes {
            constraints.push(Constraint::Datatype(n));
        }

        // sh:nodeKind
        for t in self.objects_of(id, sh::NODE_KIND) {
            if let Term::NamedNode(n) = &t {
                let nk = parse_node_kind(n.as_str())
                    .ok_or_else(|| format!("unknown sh:nodeKind value <{}> on {id}", n.as_str()))?;
                constraints.push(Constraint::NodeKind(nk));
            }
        }

        // sh:minCount
        for t in self.objects_of(id, sh::MIN_COUNT) {
            let v = crate::shapes::parse_u64(&t).ok_or_else(|| {
                format!("sh:minCount value is not a non-negative integer on {id}")
            })?;
            constraints.push(Constraint::MinCount(v));
        }

        // sh:maxCount
        for t in self.objects_of(id, sh::MAX_COUNT) {
            let v = crate::shapes::parse_u64(&t).ok_or_else(|| {
                format!("sh:maxCount value is not a non-negative integer on {id}")
            })?;
            constraints.push(Constraint::MaxCount(v));
        }

        // sh:minLength
        for t in self.objects_of(id, sh::MIN_LENGTH) {
            let v = crate::shapes::parse_u64(&t).ok_or_else(|| {
                format!("sh:minLength value is not a non-negative integer on {id}")
            })?;
            constraints.push(Constraint::MinLength(v));
        }

        // sh:maxLength
        for t in self.objects_of(id, sh::MAX_LENGTH) {
            let v = crate::shapes::parse_u64(&t).ok_or_else(|| {
                format!("sh:maxLength value is not a non-negative integer on {id}")
            })?;
            constraints.push(Constraint::MaxLength(v));
        }

        // sh:languageIn — an RDF list of language-tag string literals
        let mut lang_in_lists: Vec<Term> = self.objects_of(id, sh::LANGUAGE_IN);
        lang_in_lists.sort_by_key(ToString::to_string);
        for list_head in lang_in_lists {
            let items = self.walk_rdf_list(&list_head, id)?;
            let mut tags: Vec<String> = Vec::with_capacity(items.len());
            for item in items {
                match item {
                    Term::Literal(lit) => tags.push(lit.value().to_owned()),
                    other => {
                        return Err(format!(
                            "sh:languageIn list on {id} contains a non-literal language tag: {other}"
                        ));
                    }
                }
            }
            constraints.push(Constraint::LanguageIn(tags));
        }

        // sh:not — a single nested shape (mirrors sh:node)
        let mut not_refs: Vec<Term> = self.objects_of(id, sh::NOT);
        not_refs.sort_by_key(ToString::to_string);
        for not_ref in not_refs {
            let inner = self.parse_node_shape(not_ref)?;
            constraints.push(Constraint::Not(Box::new(inner)));
        }

        // sh:closed (+ sh:ignoredProperties) — node-shape-level closed-world check.
        // Only emit the constraint when sh:closed is true.
        let is_closed = self
            .first_object_of(id, sh::CLOSED)
            .is_some_and(|t| match &t {
                Term::Literal(lit) => lit.value() == "true",
                _ => false,
            });
        if is_closed {
            let mut ignored: Vec<NamedNode> = Vec::new();
            let mut ignored_lists: Vec<Term> = self.objects_of(id, sh::IGNORED_PROPERTIES);
            ignored_lists.sort_by_key(ToString::to_string);
            for list_head in ignored_lists {
                for item in self.walk_rdf_list(&list_head, id)? {
                    match item {
                        Term::NamedNode(n) => ignored.push(n),
                        // sh:ignoredProperties members must be IRIs; silently
                        // skipping a non-IRI would let a malformed shapes graph load
                        // and feed bad data downstream (hard-fail, no silent drop).
                        other => {
                            return Err(format!(
                                "sh:ignoredProperties list on {id} contains a non-IRI member: {other}"
                            ));
                        }
                    }
                }
            }
            ignored.sort_by(|a, b| a.as_str().cmp(b.as_str()));
            ignored.dedup();
            constraints.push(Constraint::Closed { ignored });
        }

        // sh:uniqueLang
        for t in self.objects_of(id, sh::UNIQUE_LANG) {
            if let Term::Literal(lit) = &t {
                let flag = lit.value() == "true";
                constraints.push(Constraint::UniqueLang(flag));
            }
        }

        // sh:minInclusive / sh:maxInclusive
        let mut min_inc: Vec<Term> = self.objects_of(id, sh::MIN_INCLUSIVE);
        min_inc.sort_by_key(ToString::to_string);
        for t in min_inc {
            constraints.push(Constraint::MinInclusive(t));
        }

        let mut max_inc: Vec<Term> = self.objects_of(id, sh::MAX_INCLUSIVE);
        max_inc.sort_by_key(ToString::to_string);
        for t in max_inc {
            constraints.push(Constraint::MaxInclusive(t));
        }

        // sh:minExclusive / sh:maxExclusive
        let mut min_exc: Vec<Term> = self.objects_of(id, sh::MIN_EXCLUSIVE);
        min_exc.sort_by_key(ToString::to_string);
        for t in min_exc {
            constraints.push(Constraint::MinExclusive(t));
        }

        let mut max_exc: Vec<Term> = self.objects_of(id, sh::MAX_EXCLUSIVE);
        max_exc.sort_by_key(ToString::to_string);
        for t in max_exc {
            constraints.push(Constraint::MaxExclusive(t));
        }

        // sh:hasValue
        let mut hv: Vec<Term> = self.objects_of(id, sh::HAS_VALUE);
        hv.sort_by_key(ToString::to_string);
        for t in hv {
            constraints.push(Constraint::HasValue(t));
        }

        // sh:in
        let mut in_lists: Vec<Term> = self.objects_of(id, sh::IN);
        in_lists.sort_by_key(ToString::to_string);
        for list_head in in_lists {
            let items = self.walk_rdf_list(&list_head, id)?;
            constraints.push(Constraint::In(items));
        }

        // sh:pattern + optional sh:flags
        let mut patterns: Vec<String> = self
            .objects_of(id, sh::PATTERN)
            .into_iter()
            .filter_map(|t| match t {
                Term::Literal(lit) => Some(lit.value().to_owned()),
                _ => None,
            })
            .collect();
        patterns.sort();
        let flags_val: Option<String> = self
            .objects_of(id, sh::FLAGS)
            .into_iter()
            .filter_map(|t| match t {
                Term::Literal(lit) => Some(lit.value().to_owned()),
                _ => None,
            })
            .min(); // take the lexicographically smallest if multiple
        for regex in patterns {
            constraints.push(Constraint::Pattern {
                regex,
                flags: flags_val.clone(),
                compiled: Arc::new(OnceLock::new()),
            });
        }

        // sh:and / sh:or / sh:xone — each is an RDF list of shape nodes
        let mut and_lists: Vec<Term> = self.objects_of(id, sh::AND);
        and_lists.sort_by_key(ToString::to_string);
        for list_head in and_lists {
            let members = self.parse_shape_list(&list_head, id)?;
            constraints.push(Constraint::And(members));
        }

        let mut or_lists: Vec<Term> = self.objects_of(id, sh::OR);
        or_lists.sort_by_key(ToString::to_string);
        for list_head in or_lists {
            let members = self.parse_shape_list(&list_head, id)?;
            constraints.push(Constraint::Or(members));
        }

        let mut xone_lists: Vec<Term> = self.objects_of(id, sh::XONE);
        xone_lists.sort_by_key(ToString::to_string);
        for list_head in xone_lists {
            let members = self.parse_shape_list(&list_head, id)?;
            constraints.push(Constraint::Xone(members));
        }

        // sh:node
        let mut node_refs: Vec<Term> = self.objects_of(id, sh::NODE);
        node_refs.sort_by_key(ToString::to_string);
        for node_ref in node_refs {
            let inner = self.parse_node_shape(node_ref)?;
            constraints.push(Constraint::Node(Box::new(inner)));
        }

        // sh:sparql — SHACL-AF SPARQL constraint components.
        // The blank node may or may not carry rdf:type sh:SPARQLConstraint;
        // we require only sh:select (which must be a SELECT query).
        let mut sparql_cnodes: Vec<Term> = self.objects_of(id, sh::SPARQL);
        sparql_cnodes.sort_by_key(ToString::to_string);
        for c_node in sparql_cnodes {
            // sh:select is required.
            let raw_select = self
                .first_object_of(&c_node, sh::SELECT)
                .and_then(|t| match t {
                    Term::Literal(lit) => Some(lit.value().to_owned()),
                    _ => None,
                })
                .ok_or_else(|| {
                    format!(
                        "sh:sparql constraint on shape {id} is missing a sh:select string literal"
                    )
                })?;
            // SHACL-AF sh:prefixes may be declared on the shape or the sh:sparql node.
            let select = format!("{}{raw_select}", self.prefix_header(&[id, &c_node]));

            // Parse-time query validation via the native parser (hard-fail on
            // unparsable queries). SHACL-SPARQL requires a SELECT; ASK/CONSTRUCT/
            // DESCRIBE parse but cannot bind ?this and would panic at eval — reject
            // at the boundary.
            match SparqlParser::new().parse_query(&select) {
                Ok(query @ Query::Select { .. }) => {
                    // The query runs with $this pre-bound to each focus node;
                    // the SHACL-SPARQL §5.2.1 pre-binding restrictions (no
                    // MINUS / SERVICE / VALUES, no `AS $this`, subqueries must
                    // project $this) reject it as a hard failure at load.
                    crate::prebinding::check_select(&query, &["this"])
                        .map_err(|e| format!("sh:sparql constraint on shape {id}: {e}"))?;
                }
                Ok(_) => {
                    return Err(format!(
                        "sh:sparql constraint on shape {id} must be a SELECT query (ASK/CONSTRUCT/DESCRIBE are not valid SHACL-SPARQL)"
                    ));
                }
                Err(e) => {
                    return Err(format!(
                        "sh:sparql constraint on shape {id} has an unparsable sh:select query: {e}"
                    ));
                }
            }

            // Optional per-constraint sh:message override.
            let mut messages: Vec<String> = self
                .objects_of(&c_node, sh::MESSAGE)
                .into_iter()
                .filter_map(|t| match t {
                    Term::Literal(lit) => Some(lit.value().to_owned()),
                    _ => None,
                })
                .collect();
            messages.sort();
            let message = messages.into_iter().next();

            // Optional per-constraint sh:severity override.
            let severity = self
                .first_object_of(&c_node, sh::SEVERITY)
                .and_then(|t| severity_from_term(&t));

            constraints.push(Constraint::Sparql {
                select,
                message,
                severity,
            });
        }

        // sh:expression — SHACL-AF §5.7 expression constraint component. Each
        // object is a node expression parsed via `parse_node_expr`; the optional
        // sh:message / sh:severity on the expression node override the shape
        // defaults at eval time (mirroring sh:sparql).
        let mut expr_nodes: Vec<Term> = self.objects_of(id, sh::EXPRESSION);
        expr_nodes.sort_by_key(ToString::to_string);
        for expr_node in expr_nodes {
            let expr = self.parse_node_expr(&expr_node)?;

            let mut messages: Vec<String> = self
                .objects_of(&expr_node, sh::MESSAGE)
                .into_iter()
                .filter_map(|t| match t {
                    Term::Literal(lit) => Some(lit.value().to_owned()),
                    _ => None,
                })
                .collect();
            messages.sort();
            let message = messages.into_iter().next();

            let severity = self
                .first_object_of(&expr_node, sh::SEVERITY)
                .and_then(|t| severity_from_term(&t));

            constraints.push(Constraint::Expression {
                expr,
                message,
                severity,
            });
        }

        // sh:equals / sh:disjoint / sh:lessThan / sh:lessThanOrEquals — the
        // property-pair constraint components (§4.3). Each object must be an IRI;
        // a non-IRI object is malformed and hard-fails (no silent drop).
        for (pred, make) in [
            (
                sh::EQUALS,
                Constraint::Equals as fn(NamedNode) -> Constraint,
            ),
            (sh::DISJOINT, Constraint::Disjoint as fn(_) -> _),
            (sh::LESS_THAN, Constraint::LessThan as fn(_) -> _),
            (
                sh::LESS_THAN_OR_EQUALS,
                Constraint::LessThanOrEquals as fn(_) -> _,
            ),
        ] {
            let mut props: Vec<NamedNode> = Vec::new();
            for t in self.objects_of(id, pred) {
                match t {
                    Term::NamedNode(n) => props.push(n),
                    other => {
                        return Err(format!(
                            "<{pred}> on shape {id} must be an IRI, got {other}"
                        ));
                    }
                }
            }
            props.sort_by(|a, b| a.as_str().cmp(b.as_str()));
            for n in props {
                constraints.push(make(n));
            }
        }

        // sh:qualifiedValueShape + sh:qualifiedMinCount / sh:qualifiedMaxCount
        // (§4.5.4–4.5.5). The counts require the shape and vice versa — a
        // dangling half of the pair is malformed and hard-fails.
        constraints.extend(self.parse_qualified_value_shapes(id)?);

        // Custom SHACL-SPARQL constraint components. A shape that carries values
        // for all required parameters of a declared component is treated as a
        // usage of that component. Components are processed in deterministic
        // order; parameter bindings follow the component's declared parameter
        // order. All validators applicable to the current shape scope are
        // emitted as separate constraints; if none apply, the component is
        // skipped silently.
        let shape_severity = self
            .first_object_of(id, sh::SEVERITY)
            .and_then(|t| severity_from_term(&t));
        let mut shape_messages: Vec<String> = self
            .objects_of(id, sh::MESSAGE)
            .into_iter()
            .filter_map(|t| match t {
                Term::Literal(lit) => Some(lit.value().to_owned()),
                _ => None,
            })
            .collect();
        shape_messages.sort();
        let shape_message = shape_messages.into_iter().next();

        let mut components: Vec<&Component> = self.component_registry.components.values().collect();
        components.sort_by(|a, b| a.id.as_str().cmp(b.id.as_str()));
        for component in components {
            let mut bindings: Vec<(String, Term)> = Vec::new();
            let mut missing_required = false;
            for param in &component.parameters {
                let values = self.objects_of(id, param.path.as_str());
                if values.len() > 1 {
                    return Err(format!(
                        "shape {id} declares {count} values for parameter <{path}> of component <{component}>, only one is allowed",
                        count = values.len(),
                        path = param.path,
                        component = component.id
                    ));
                }
                if let Some(value) = values.into_iter().next() {
                    bindings.push((param.name.clone(), value));
                } else if !param.optional {
                    missing_required = true;
                    break;
                }
            }
            if missing_required {
                continue;
            }

            let matching: Vec<&Validator> = if is_property_shape {
                component
                    .property_validators
                    .iter()
                    .chain(component.validators.iter())
                    .collect()
            } else {
                component
                    .node_validators
                    .iter()
                    .chain(component.validators.iter())
                    .collect()
            };
            if matching.is_empty() {
                continue;
            }

            for validator in matching {
                let component_validator = match &validator.kind {
                    ValidatorKind::Ask => ComponentValidator::Ask {
                        ask: validator.query_text.clone(),
                    },
                    ValidatorKind::Select => ComponentValidator::Select {
                        select: validator.query_text.clone(),
                    },
                };

                let severity = shape_severity
                    .clone()
                    .or_else(|| validator.severity.clone())
                    .or_else(|| component.severity.clone());
                let message = shape_message
                    .clone()
                    .or_else(|| validator.message.clone())
                    .or_else(|| component.message.clone());

                constraints.push(Constraint::Component {
                    component: component.id.clone(),
                    source_shape: id.clone(),
                    bindings: bindings.clone(),
                    validator: component_validator,
                    message,
                    severity,
                });
            }
        }

        Ok(constraints)
    }

    /// Parse the qualified-value-shape constraint(s) declared on `id`.
    ///
    /// Returns one [`Constraint::QualifiedValueShape`] per `sh:qualifiedValueShape`
    /// object (sorted for determinism). The declared `sh:qualifiedMinCount` /
    /// `sh:qualifiedMaxCount` apply to each. When
    /// `sh:qualifiedValueShapesDisjoint true` is set, the sibling qualified value
    /// shapes (§4.5.5: the values of `sh:property/sh:qualifiedValueShape` on the
    /// parents of `id`, minus the constraint's own shape) are parsed and stored.
    fn parse_qualified_value_shapes(&mut self, id: &Term) -> Result<Vec<Constraint>, String> {
        let mut qvs_nodes: Vec<Term> = self.objects_of(id, sh::QUALIFIED_VALUE_SHAPE);
        qvs_nodes.sort_by_key(ToString::to_string);

        let min_count = match self.first_object_of(id, sh::QUALIFIED_MIN_COUNT) {
            Some(t) => Some(crate::shapes::parse_u64(&t).ok_or_else(|| {
                format!("sh:qualifiedMinCount value is not a non-negative integer on {id}")
            })?),
            None => None,
        };
        let max_count = match self.first_object_of(id, sh::QUALIFIED_MAX_COUNT) {
            Some(t) => Some(crate::shapes::parse_u64(&t).ok_or_else(|| {
                format!("sh:qualifiedMaxCount value is not a non-negative integer on {id}")
            })?),
            None => None,
        };

        if qvs_nodes.is_empty() {
            // sh:qualifiedMinCount / sh:qualifiedMaxCount without an
            // sh:qualifiedValueShape leaves the constraint component INACTIVE
            // (its mandatory parameter is absent — W3C core/node/qualified-001
            // expects the dangling counts to be ignored, not a hard failure).
            return Ok(vec![]);
        }
        if min_count.is_none() && max_count.is_none() {
            return Err(format!(
                "sh:qualifiedValueShape on {id} requires sh:qualifiedMinCount or \
                 sh:qualifiedMaxCount"
            ));
        }

        let disjoint = self
            .first_object_of(id, sh::QUALIFIED_VALUE_SHAPES_DISJOINT)
            .is_some_and(|t| matches!(&t, Term::Literal(lit) if lit.value() == "true"));

        let mut out = Vec::with_capacity(qvs_nodes.len());
        for qvs_node in &qvs_nodes {
            let shape = self.parse_inline_shape(qvs_node.clone())?;
            let siblings = if disjoint {
                self.parse_sibling_qualified_shapes(id, qvs_node)?
            } else {
                vec![]
            };
            out.push(Constraint::QualifiedValueShape {
                shape: Box::new(shape),
                siblings,
                min_count,
                max_count,
                disjoint,
            });
        }
        Ok(out)
    }

    /// Collect and parse the sibling qualified value shapes of `own_qvs` (§4.5.5):
    /// all values of `sh:property/sh:qualifiedValueShape` reachable from the
    /// parents of the property shape `ps_id`, minus `own_qvs` itself.
    fn parse_sibling_qualified_shapes(
        &mut self,
        ps_id: &Term,
        own_qvs: &Term,
    ) -> Result<Vec<Shape>, String> {
        let property = Term::NamedNode(NamedNode::from(sh::PROPERTY));
        let mut sibling_nodes: Vec<Term> = Vec::new();
        let mut seen: FastSet<Term> = FastSet::default();
        // Parents: subjects of (?, sh:property, ps_id).
        let mut parents: Vec<Term> = native_quads(
            self.data,
            None,
            Some(&property),
            Some(ps_id),
            GraphFilter::AnyGraph,
        )
        .into_iter()
        .map(|(subject, _, _)| subject)
        .collect();
        parents.sort_by_key(Term::to_string);
        parents.dedup();
        for parent in &parents {
            let mut sibling_ps: Vec<Term> = self.objects_of(parent, sh::PROPERTY);
            sibling_ps.sort_by_key(Term::to_string);
            for ps in sibling_ps {
                let mut qvs: Vec<Term> = self.objects_of(&ps, sh::QUALIFIED_VALUE_SHAPE);
                qvs.sort_by_key(Term::to_string);
                for q in qvs {
                    if &q != own_qvs && seen.insert(q.clone()) {
                        sibling_nodes.push(q);
                    }
                }
            }
        }
        let mut siblings = Vec::with_capacity(sibling_nodes.len());
        for node in sibling_nodes {
            siblings.push(self.parse_inline_shape(node)?);
        }
        Ok(siblings)
    }

    // ── SHACL-AF node expressions (spec §5) ─────────────────────────────────────

    /// Parse a shapes-graph node into a SHACL-AF [`NodeExpr`] (spec §5).
    ///
    /// Paging/ordering wrappers (`sh:limit` / `sh:offset` / `sh:orderby`) are
    /// peeled first and applied on top of the node's *core* expression in SPARQL
    /// pipeline order (`ORDER BY` → `OFFSET` → `LIMIT`, with `LIMIT` outermost);
    /// everything else dispatches through [`parse_node_expr_core`].
    ///
    /// A blank node is guarded against cyclic self-reference (mirroring
    /// [`parse_inline_shape`](super::Parser::parse_inline_shape)); the guard key is
    /// namespaced so it never collides with the shape-parsing `in_flight` set.
    pub(crate) fn parse_node_expr(&mut self, node: &Term) -> Result<NodeExpr, String> {
        // NOTE: paging/ordering surface (`sh:limit`/`sh:offset`/`sh:orderby`) is
        // under-specified by SHACL-AF. Assumption pinned here (a later corpus
        // task validates it): these keys WRAP the same node's core expression —
        // the inner operand is this very node parsed with the paging keys
        // ignored, NOT a separate `sh:nodes` operand. A node carrying only paging
        // keys (no core expression) therefore hard-fails in `parse_node_expr_core`.
        let guard_key = format!("nodeexpr:{node}");
        let is_blank = matches!(node, Term::BlankNode(_));
        if is_blank {
            if self.in_flight.contains(&guard_key) {
                return Err(format!("cyclic node expression on {node}"));
            }
            self.in_flight.insert(guard_key.clone());
        }
        let result = self.parse_node_expr_wrapped(node);
        if is_blank {
            self.in_flight.remove(&guard_key);
        }
        result
    }

    /// Apply the paging/ordering wrappers on top of the core expression.
    fn parse_node_expr_wrapped(&mut self, node: &Term) -> Result<NodeExpr, String> {
        let mut expr = self.parse_node_expr_core(node)?;

        // ORDER BY (innermost wrapper). `sh:orderby` names the sort-key node
        // expression (evaluated element-as-focus); direction is the separate
        // `sh:desc` boolean flag (default ascending).
        if let Some(key_node) = self.first_object_of(node, sh::ORDERBY) {
            let key = self.parse_node_expr(&key_node)?;
            let descending = match self.first_object_of(node, sh::DESC) {
                None => false,
                Some(term @ Term::Literal(_)) => {
                    let Term::Literal(lit) = &term else {
                        unreachable!()
                    };
                    match purrdf_xsd::parse_by_iri(lit.value(), lit.datatype_str()) {
                        Ok(Some(purrdf_xsd::XsdValue::Boolean(b))) => b,
                        _ => {
                            return Err(format!(
                                "sh:desc must be an xsd:boolean literal, got {term}"
                            ));
                        }
                    }
                }
                Some(other) => {
                    return Err(format!(
                        "sh:desc must be an xsd:boolean literal, got {other}"
                    ));
                }
            };
            expr = NodeExpr::OrderBy {
                of: Box::new(expr),
                key: Box::new(key),
                descending,
            };
        }

        // OFFSET.
        if let Some(off) = self.first_object_of(node, sh::OFFSET) {
            let n = crate::shapes::parse_u64(&off).ok_or_else(|| {
                format!("sh:offset value is not a non-negative integer on {node}")
            })?;
            expr = NodeExpr::Offset {
                of: Box::new(expr),
                n,
            };
        }

        // LIMIT (outermost wrapper).
        if let Some(lim) = self.first_object_of(node, sh::LIMIT) {
            let n = crate::shapes::parse_u64(&lim)
                .ok_or_else(|| format!("sh:limit value is not a non-negative integer on {node}"))?;
            expr = NodeExpr::Limit {
                of: Box::new(expr),
                n,
            };
        }

        Ok(expr)
    }

    /// Parse the non-paging *core* of a node expression.
    ///
    /// Dispatches on the single structural SHACL-AF key the node carries, in a
    /// fixed deterministic order, and hard-fails when a node carries two
    /// mutually-exclusive expression keys (ambiguous).
    fn parse_node_expr_core(&mut self, node: &Term) -> Result<NodeExpr, String> {
        // Literals are always constant term expressions (they bear no triples).
        if matches!(node, Term::Literal(_)) {
            return Ok(NodeExpr::Constant(node.clone()));
        }
        // The focus-node expression `sh:this`.
        if let Term::NamedNode(n) = node {
            if n.as_str() == sh::THIS {
                return Ok(NodeExpr::This);
            }
        }

        // Which mutually-exclusive structural key does the node carry?
        let primary = [
            sh::PATH,
            sh::FILTER_SHAPE,
            sh::UNION,
            sh::INTERSECTION,
            sh::IF,
            sh::COUNT,
            sh::DISTINCT,
            sh::MIN,
            sh::MAX,
            sh::SUM,
            sh::EXISTS,
        ];
        let present: Vec<&str> = primary
            .into_iter()
            .filter(|&p| self.first_object_of(node, p).is_some())
            .collect();
        if present.len() > 1 {
            return Err(format!(
                "ambiguous node expression on {node}: multiple expression keys {present:?}"
            ));
        }

        if let Some(&key) = present.first() {
            return self.parse_structural_node_expr(node, key);
        }

        // No structural key: a function call, a plain constant IRI, or malformed.
        self.parse_call_or_constant(node)
    }

    /// Dispatch a node carrying exactly one structural expression `key`.
    fn parse_structural_node_expr(&mut self, node: &Term, key: &str) -> Result<NodeExpr, String> {
        match key {
            sh::PATH => {
                let path_node = self
                    .first_object_of(node, sh::PATH)
                    .ok_or_else(|| format!("sh:path node expression on {node} lost its object"))?;
                let path = self.parse_path(&path_node, node, &mut FastSet::default())?;
                Ok(NodeExpr::Path(path))
            }
            sh::FILTER_SHAPE => {
                let shape_ref = self
                    .first_object_of(node, sh::FILTER_SHAPE)
                    .ok_or_else(|| {
                        format!("sh:filterShape node expression on {node} lost its object")
                    })?;
                let nodes_obj = self.first_object_of(node, sh::NODES).ok_or_else(|| {
                    format!("sh:filterShape node expression on {node} requires sh:nodes")
                })?;
                let inner = self.parse_node_expr(&nodes_obj)?;
                let shape = self.parse_inline_shape(shape_ref)?;
                Ok(NodeExpr::Filter {
                    nodes: Box::new(inner),
                    shape: Box::new(shape),
                })
            }
            sh::UNION => Ok(NodeExpr::Union(self.parse_node_expr_list(node, sh::UNION)?)),
            sh::INTERSECTION => Ok(NodeExpr::Intersection(
                self.parse_node_expr_list(node, sh::INTERSECTION)?,
            )),
            sh::IF => {
                let cond_obj = self
                    .first_object_of(node, sh::IF)
                    .ok_or_else(|| format!("sh:if node expression on {node} lost its object"))?;
                let cond = self.parse_node_expr(&cond_obj)?;
                // Per spec a missing `sh:then`/`sh:else` yields the empty set; the
                // empty union is the canonical empty-set node expression.
                let then = match self.first_object_of(node, sh::THEN) {
                    Some(t) => self.parse_node_expr(&t)?,
                    None => NodeExpr::Union(vec![]),
                };
                let els = match self.first_object_of(node, sh::ELSE) {
                    Some(t) => self.parse_node_expr(&t)?,
                    None => NodeExpr::Union(vec![]),
                };
                Ok(NodeExpr::If {
                    cond: Box::new(cond),
                    then: Box::new(then),
                    els: Box::new(els),
                })
            }
            sh::COUNT => {
                let of_obj = self
                    .first_object_of(node, sh::COUNT)
                    .ok_or_else(|| format!("sh:count node expression on {node} lost its object"))?;
                // Distinct counting is `[ sh:count [ sh:distinct <expr> ] ]`: an
                // inner `sh:distinct` lowers to `Count { distinct: true, .. }`.
                match self.parse_node_expr(&of_obj)? {
                    NodeExpr::Distinct(inner) => Ok(NodeExpr::Count {
                        distinct: true,
                        of: inner,
                    }),
                    other => Ok(NodeExpr::Count {
                        distinct: false,
                        of: Box::new(other),
                    }),
                }
            }
            sh::DISTINCT => {
                let of_obj = self.first_object_of(node, sh::DISTINCT).ok_or_else(|| {
                    format!("sh:distinct node expression on {node} lost its object")
                })?;
                Ok(NodeExpr::Distinct(Box::new(self.parse_node_expr(&of_obj)?)))
            }
            sh::MIN => {
                let of_obj = self
                    .first_object_of(node, sh::MIN)
                    .ok_or_else(|| format!("sh:min node expression on {node} lost its object"))?;
                Ok(NodeExpr::Min(Box::new(self.parse_node_expr(&of_obj)?)))
            }
            sh::MAX => {
                let of_obj = self
                    .first_object_of(node, sh::MAX)
                    .ok_or_else(|| format!("sh:max node expression on {node} lost its object"))?;
                Ok(NodeExpr::Max(Box::new(self.parse_node_expr(&of_obj)?)))
            }
            sh::SUM => {
                let of_obj = self
                    .first_object_of(node, sh::SUM)
                    .ok_or_else(|| format!("sh:sum node expression on {node} lost its object"))?;
                Ok(NodeExpr::Sum(Box::new(self.parse_node_expr(&of_obj)?)))
            }
            sh::EXISTS => {
                // Adopted semantics: `sh:exists` is a node-expression predicate —
                // true iff its inner NODE EXPRESSION yields at least one node for
                // the focus. (A shape does not "produce nodes", so the operand is
                // an expression, not a shape.)
                let inner_obj = self.first_object_of(node, sh::EXISTS).ok_or_else(|| {
                    format!("sh:exists node expression on {node} lost its object")
                })?;
                let inner = self.parse_node_expr(&inner_obj)?;
                Ok(NodeExpr::Exists(Box::new(inner)))
            }
            other => Err(format!(
                "internal error: unhandled node-expression key <{other}> on {node}"
            )),
        }
    }

    /// Parse the RDF list at `(node, predicate)` into a vector of node expressions.
    fn parse_node_expr_list(
        &mut self,
        node: &Term,
        predicate: &str,
    ) -> Result<Vec<NodeExpr>, String> {
        let list_head = self
            .first_object_of(node, predicate)
            .ok_or_else(|| format!("<{predicate}> node expression on {node} lost its object"))?;
        let items = self.walk_rdf_list(&list_head, node)?;
        let mut exprs = Vec::with_capacity(items.len());
        for item in items {
            exprs.push(self.parse_node_expr(&item)?);
        }
        Ok(exprs)
    }

    /// Parse a node carrying no structural key: a function call or a plain
    /// constant IRI (a blank node with neither hard-fails).
    fn parse_call_or_constant(&mut self, node: &Term) -> Result<NodeExpr, String> {
        // A function-call node expression is always a blank node `[ <fn> ( … ) ]`.
        // A NamedNode reaching here (not a literal, not sh:this, no structural key)
        // is therefore a plain constant IRI — even when it bears unrelated outgoing
        // triples in the shapes graph (e.g. an `rdfs:label`).
        if matches!(node, Term::NamedNode(_)) {
            return Ok(NodeExpr::Constant(node.clone()));
        }
        // The SHACL-AF vocabulary terms that structure a node expression — none of
        // them can be the predicate of a function-call expression.
        const KNOWN: &[&str] = &[
            sh::PATH,
            sh::FILTER_SHAPE,
            sh::NODES,
            sh::UNION,
            sh::INTERSECTION,
            sh::IF,
            sh::THEN,
            sh::ELSE,
            sh::COUNT,
            sh::DISTINCT,
            sh::MIN,
            sh::MAX,
            sh::SUM,
            sh::LIMIT,
            sh::OFFSET,
            sh::ORDERBY,
            sh::DESC,
            sh::EXISTS,
        ];
        // Gather the candidate (function IRI, arg-list head) triples, ignoring
        // rdf:type (a classification triple) and every SHACL structural key.
        let mut candidates: Vec<(NamedNode, Term)> =
            native_quads(self.data, Some(node), None, None, GraphFilter::AnyGraph)
                .into_iter()
                .filter(|(_, predicate, _)| {
                    predicate.as_str() != rdf::TYPE && !KNOWN.contains(&predicate.as_str())
                })
                .map(|(_, predicate, object)| (predicate, object))
                .collect();
        candidates.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));
        candidates.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);

        if candidates.is_empty() {
            // No structural key and no function predicate: only blank nodes reach
            // here (NamedNodes returned early as constants above), and a blank
            // node with neither is malformed.
            return Err(format!(
                "unrecognised node expression on {node}: no SHACL-AF key and no function call"
            ));
        }
        if candidates.len() > 1 {
            return Err(format!(
                "ambiguous function-call node expression on {node}: multiple candidate predicates"
            ));
        }

        let (fn_iri, args_head) = candidates
            .into_iter()
            .next()
            .ok_or_else(|| format!("internal error: function-call candidate vanished on {node}"))?;
        // The single object must be an RDF list of argument node expressions
        // (`rdf:nil` is the empty argument list).
        let nil = Term::NamedNode(NamedNode::from(rdf::NIL));
        let is_list = args_head == nil || self.first_object_of(&args_head, rdf::FIRST).is_some();
        if !is_list {
            return Err(format!(
                "function-call node expression <{}> on {node} must carry an RDF list of arguments",
                fn_iri.as_str()
            ));
        }
        let items = self.walk_rdf_list(&args_head, node)?;
        let mut args = Vec::with_capacity(items.len());
        for item in items {
            args.push(self.parse_node_expr(&item)?);
        }
        // A user-defined function is typed `sh:SPARQLFunction` (or `sh:Function`)
        // in the shapes graph; anything else is treated as a builtin.
        let iri_term = Term::NamedNode(fn_iri.clone());
        let user_defined =
            self.has_type(&iri_term, sh::SPARQL_FUNCTION) || self.has_type(&iri_term, sh::FUNCTION);
        let call = if user_defined {
            FnCall::UserDefined { iri: fn_iri, args }
        } else {
            FnCall::Builtin { iri: fn_iri, args }
        };
        Ok(NodeExpr::Call(call))
    }
}

/// Parse `sh:nodeKind` object IRI into a [`NodeKindValue`].
fn parse_node_kind(iri: &str) -> Option<NodeKindValue> {
    match iri {
        "http://www.w3.org/ns/shacl#IRI" => Some(NodeKindValue::Iri),
        "http://www.w3.org/ns/shacl#BlankNode" => Some(NodeKindValue::BlankNode),
        "http://www.w3.org/ns/shacl#Literal" => Some(NodeKindValue::Literal),
        "http://www.w3.org/ns/shacl#BlankNodeOrIRI" => Some(NodeKindValue::BlankNodeOrIri),
        "http://www.w3.org/ns/shacl#BlankNodeOrLiteral" => Some(NodeKindValue::BlankNodeOrLiteral),
        "http://www.w3.org/ns/shacl#IRIOrLiteral" => Some(NodeKindValue::IriOrLiteral),
        _ => None,
    }
}

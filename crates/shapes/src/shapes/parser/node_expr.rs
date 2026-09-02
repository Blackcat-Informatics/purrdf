// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SHACL Core constraint parsing and SHACL-AF node-expression parsing.

use ::purrdf::FastSet;
use std::sync::{Arc, OnceLock};

use purrdf_sparql_algebra::{GraphPattern, Query, SparqlParser};

use crate::components::{Component, Validator, ValidatorKind, severity_from_term};
use crate::data::{GraphFilter, native_quads};
use crate::expression::{
    ArgKey, CustomFnKind, CustomFunction, FnCall, NodeExpr, sparql_ns_lowering,
};
use crate::model::{rdf, sh, shnex, sparql_ns};
use crate::term::{NamedNode, Term};

use crate::shapes::{ComponentValidator, Constraint, InFlight, NodeKindValue, Parser, Shape};

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
        // Remember which shape these constraints belong to, so a `sh:select` node
        // expression resolves shape-level `sh:prefixes` exactly as `sh:sparql`
        // does. Saved and restored rather than cleared: an inline shape parsed
        // inside an expression must not strip the enclosing shape's prefixes from
        // the expressions that follow it.
        let saved_shape = self.current_shape.replace(id.clone());
        let result = self.parse_constraints_inner(id, is_property_shape);
        self.current_shape = saved_shape;
        result
    }

    fn parse_constraints_inner(
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
        crate::term::sort_terms_canonical(&mut lang_in_lists);
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
        crate::term::sort_terms_canonical(&mut not_refs);
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
            crate::term::sort_terms_canonical(&mut ignored_lists);
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
        crate::term::sort_terms_canonical(&mut min_inc);
        for t in min_inc {
            constraints.push(Constraint::MinInclusive(t));
        }

        let mut max_inc: Vec<Term> = self.objects_of(id, sh::MAX_INCLUSIVE);
        crate::term::sort_terms_canonical(&mut max_inc);
        for t in max_inc {
            constraints.push(Constraint::MaxInclusive(t));
        }

        // sh:minExclusive / sh:maxExclusive
        let mut min_exc: Vec<Term> = self.objects_of(id, sh::MIN_EXCLUSIVE);
        crate::term::sort_terms_canonical(&mut min_exc);
        for t in min_exc {
            constraints.push(Constraint::MinExclusive(t));
        }

        let mut max_exc: Vec<Term> = self.objects_of(id, sh::MAX_EXCLUSIVE);
        crate::term::sort_terms_canonical(&mut max_exc);
        for t in max_exc {
            constraints.push(Constraint::MaxExclusive(t));
        }

        // sh:hasValue
        let mut hv: Vec<Term> = self.objects_of(id, sh::HAS_VALUE);
        crate::term::sort_terms_canonical(&mut hv);
        for t in hv {
            constraints.push(Constraint::HasValue(t));
        }

        // sh:in
        let mut in_lists: Vec<Term> = self.objects_of(id, sh::IN);
        crate::term::sort_terms_canonical(&mut in_lists);
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
        crate::term::sort_terms_canonical(&mut and_lists);
        for list_head in and_lists {
            let members = self.parse_shape_list(&list_head, id)?;
            constraints.push(Constraint::And(members));
        }

        let mut or_lists: Vec<Term> = self.objects_of(id, sh::OR);
        crate::term::sort_terms_canonical(&mut or_lists);
        for list_head in or_lists {
            let members = self.parse_shape_list(&list_head, id)?;
            constraints.push(Constraint::Or(members));
        }

        let mut xone_lists: Vec<Term> = self.objects_of(id, sh::XONE);
        crate::term::sort_terms_canonical(&mut xone_lists);
        for list_head in xone_lists {
            let members = self.parse_shape_list(&list_head, id)?;
            constraints.push(Constraint::Xone(members));
        }

        // sh:node
        let mut node_refs: Vec<Term> = self.objects_of(id, sh::NODE);
        crate::term::sort_terms_canonical(&mut node_refs);
        for node_ref in node_refs {
            let inner = self.parse_node_shape(node_ref)?;
            constraints.push(Constraint::Node(Box::new(inner)));
        }

        // sh:sparql — SHACL-AF SPARQL constraint components.
        // The blank node may or may not carry rdf:type sh:SPARQLConstraint;
        // we require only sh:select (which must be a SELECT query).
        let mut sparql_cnodes: Vec<Term> = self.objects_of(id, sh::SPARQL);
        crate::term::sort_terms_canonical(&mut sparql_cnodes);
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
                    // the pre-binding restrictions of SHACL 1.2 SPARQL
                    // Extensions, Appendix A (no
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
        crate::term::sort_terms_canonical(&mut expr_nodes);
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

        // sh:nodeByExpression — SHACL 1.2 Node Expressions §7.2. The expression
        // computes the NODE SHAPES every value node must conform to, so the
        // constraint also carries the shared top-level-shape index that resolves
        // those shape IRIs at validation time.
        let mut node_by_expr_nodes: Vec<Term> = self.objects_of(id, sh::NODE_BY_EXPRESSION);
        crate::term::sort_terms_canonical(&mut node_by_expr_nodes);
        for expr_node in node_by_expr_nodes {
            let expr = self.parse_node_expr(&expr_node)?;
            // Record the shape IRIs this expression already NAMES, so they are
            // resolved at load rather than at the first value node that happens to
            // reach the constraint. See `Parser::node_by_expr_constants`.
            self.record_node_by_expr_constants(id, &expr);

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

            constraints.push(Constraint::NodeByExpression {
                expr,
                shapes: self.share_node_shape_index(),
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
        crate::term::sort_terms_canonical(&mut qvs_nodes);

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
        crate::term::sort_terms_canonical(&mut parents);
        parents.dedup();
        for parent in &parents {
            let mut sibling_ps: Vec<Term> = self.objects_of(parent, sh::PROPERTY);
            crate::term::sort_terms_canonical(&mut sibling_ps);
            for ps in sibling_ps {
                let mut qvs: Vec<Term> = self.objects_of(&ps, sh::QUALIFIED_VALUE_SHAPE);
                crate::term::sort_terms_canonical(&mut qvs);
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
    /// EVERY node kind is guarded against cyclic self-reference (mirroring
    /// [`parse_inline_shape`](super::Parser::parse_inline_shape)), keyed by the
    /// node term under [`InFlight::NodeExpr`] so it can never collide with the
    /// shape-parsing entries in the same set.
    ///
    /// The guard must NOT be restricted to blank nodes: a cycle spelled with
    /// NAMED nodes (`ex:E sh:union ( ex:E )`) is just as reachable from a shapes
    /// document, and unbounded Rust recursion aborts the process — an
    /// uncatchable failure, not an error a caller can handle. The set is a
    /// STACK of in-flight parses (each entry removed on the way out), not a
    /// visited set, so a shared sub-expression referenced twice from disjoint
    /// branches still parses normally.
    pub(crate) fn parse_node_expr(&mut self, node: &Term) -> Result<NodeExpr, String> {
        // NOTE: paging/ordering surface (`sh:limit`/`sh:offset`/`sh:orderby`) is
        // under-specified by SHACL-AF. Assumption pinned here (a later corpus
        // task validates it): these keys WRAP the same node's core expression —
        // the inner operand is this very node parsed with the paging keys
        // ignored, NOT a separate `sh:nodes` operand. A node carrying only paging
        // keys (no core expression) therefore hard-fails in `parse_node_expr_core`.
        let guard_key = InFlight::NodeExpr(node.clone());
        if self.in_flight.contains(&guard_key) {
            return Err(format!("cyclic node expression on {node}"));
        }
        self.in_flight.insert(guard_key.clone());
        let result = self.parse_node_expr_wrapped(node);
        self.in_flight.remove(&guard_key);
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
            let descending = self.parse_desc_flag(node, sh::DESC)?;
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
    /// Dispatches on the single structural key the node carries, in a fixed
    /// deterministic order, and hard-fails when a node carries two mutually
    /// exclusive expression keys (ambiguous).
    ///
    /// Both spec surfaces are accepted: the SHACL Advanced Features `sh:` spelling
    /// and the SHACL 1.2 Node Expressions `shnex:` spelling. They are NOT two
    /// dialects with two behaviours — [`PRIMARY_KEYS`] maps each IRI onto one
    /// [`ExprKind`], every kind lowers to one [`NodeExpr`] arm, and that arm has
    /// exactly one evaluation path. A node that carries BOTH spellings of the same
    /// kind is ambiguous and hard-fails exactly like a node carrying two different
    /// kinds — the writer is asked which one they meant, never silently given one.
    fn parse_node_expr_core(&mut self, node: &Term) -> Result<NodeExpr, String> {
        // Literals are always constant term expressions (they bear no triples).
        // SHACL 1.2 Node Expressions §3.1.2: a literal expression evaluates to
        // itself.
        if matches!(node, Term::Literal(_)) {
            return Ok(NodeExpr::Constant(node.clone()));
        }
        // RDF 1.2 triple terms are likewise constants. SHACL 1.2 Node Expressions
        // §3.1.3: "The output nodes of a triple term expression are the list
        // consisting of exactly the node expression itself." A triple term bears no
        // outgoing triples of its own, so it can never be a structured expression,
        // and it must be recognised BEFORE the blank-node paths below or an
        // authored `<<( ex:s ex:p ex:o )>>` would be rejected as unrecognised.
        if matches!(node, Term::Triple(_)) {
            return Ok(NodeExpr::Constant(node.clone()));
        }
        // The focus-node expression `sh:this`.
        if let Term::NamedNode(n) = node
            && n.as_str() == sh::THIS
        {
            return Ok(NodeExpr::This);
        }

        // Which mutually-exclusive structural key does the node carry? Both the
        // `sh:` and the `shnex:` spelling of a kind appear in this scan, so a node
        // carrying both is caught by the very same arity check.
        let present: Vec<(&str, ExprKind)> = PRIMARY_KEYS
            .iter()
            .copied()
            .filter(|&(iri, _)| self.first_object_of(node, iri).is_some())
            .collect();
        if present.len() > 1 {
            let keys: Vec<&str> = present.iter().map(|&(iri, _)| iri).collect();
            return Err(format!(
                "ambiguous node expression on {node}: multiple expression keys {keys:?}"
            ));
        }

        if let Some(&(iri, kind)) = present.first() {
            // Prove every OTHER node-expression key on the node is one this kind
            // actually reads, before the arm reads the ones it knows about and
            // leaves the rest on the floor.
            self.check_expression_keys(node, iri, kind)?;
            return self.parse_structural_node_expr(node, iri, kind);
        }

        // No structural key: an empty expression, a function call, a plain constant
        // IRI, or malformed.
        self.parse_call_or_constant(node)
    }

    /// The `sh:desc` / `shnex:desc` descending flag on `node` (default ascending).
    fn parse_desc_flag(&self, node: &Term, predicate: &str) -> Result<bool, String> {
        match self.first_object_of(node, predicate) {
            None => Ok(false),
            Some(Term::Literal(lit)) => {
                match purrdf_xsd::parse_by_iri(lit.value(), lit.datatype_str()) {
                    Ok(Some(purrdf_xsd::XsdValue::Boolean(b))) => Ok(b),
                    _ => Err(format!(
                        "<{predicate}> must be an xsd:boolean literal, got {}",
                        Term::Literal(lit)
                    )),
                }
            }
            Some(other) => Err(format!(
                "<{predicate}> must be an xsd:boolean literal, got {other}"
            )),
        }
    }

    /// The `shnex:nodes` input-node expression of `node`, defaulting to the focus
    /// node when absent.
    ///
    /// SHACL 1.2 Node Expressions §4.3.1–§4.3.3 spell that default as "if omitted,
    /// defaults to the focus node", which is exactly [`NodeExpr::This`] — so the
    /// default is a real expression on the ordinary evaluation path, not a
    /// special case the evaluator has to know about.
    fn parse_shnex_nodes_or_focus(&mut self, node: &Term) -> Result<NodeExpr, String> {
        match self.first_object_of(node, shnex::NODES) {
            Some(nodes) => self.parse_node_expr(&nodes),
            None => Ok(NodeExpr::This),
        }
    }

    /// The REQUIRED `shnex:nodes` input-node expression of `node`.
    fn parse_shnex_nodes_required(&mut self, node: &Term, owner: &str) -> Result<NodeExpr, String> {
        let nodes = self
            .first_object_of(node, shnex::NODES)
            .ok_or_else(|| format!("{owner} node expression on {node} requires shnex:nodes"))?;
        self.parse_node_expr(&nodes)
    }

    /// Record the shape IRIs a `sh:nodeByExpression` expression already NAMES, for
    /// the load-time resolution check in [`Parser::parse`](super::Parser::parse).
    ///
    /// Only the two spellings whose answer is decided at load are recorded: a bare
    /// constant (`sh:nodeByExpression ex:MyShape`) and a list expression of them
    /// (`sh:nodeByExpression ( ex:A ex:B )`), including through the union and
    /// conditional combinators, whose branches are themselves already named.
    /// Everything else — a path, a query, a function call — genuinely produces its
    /// shape IRIs only during validation, and is resolved there.
    ///
    /// It is deliberately a RECORDING pass and not a check: the index it resolves
    /// against is filled at the very end of the parse, after every shape exists.
    fn record_node_by_expr_constants(&mut self, shape_id: &Term, expr: &NodeExpr) {
        match expr {
            NodeExpr::Constant(term @ Term::NamedNode(_)) => self
                .node_by_expr_constants
                .push((shape_id.clone(), term.clone())),
            NodeExpr::List(members) => {
                for member in members {
                    if matches!(member, Term::NamedNode(_)) {
                        self.node_by_expr_constants
                            .push((shape_id.clone(), member.clone()));
                    }
                }
            }
            NodeExpr::Union(operands) | NodeExpr::Concat(operands) => {
                for operand in operands {
                    self.record_node_by_expr_constants(shape_id, operand);
                }
            }
            NodeExpr::If { then, els, .. } => {
                self.record_node_by_expr_constants(shape_id, then);
                self.record_node_by_expr_constants(shape_id, els);
            }
            _ => {}
        }
    }

    /// Parse a SHAPE-VALUED operand of a node expression, having first PROVED the
    /// shapes graph describes `shape_ref` as a shape.
    ///
    /// [`Parser::parse_inline_shape`](super::Parser::parse_inline_shape) answers an
    /// undescribed node with an EMPTY shape, and an empty shape conforms to
    /// everything. Every shape-valued node expression makes conformance its
    /// answer, so an undefined shape IRI does not fail — it VACUOUSLY HOLDS, and
    /// each of these returns the "all clear" reading:
    ///
    /// * `shnex:matchAll` — every node conforms to an empty shape, so `true`.
    /// * `shnex:conformsToShape` — likewise `true`.
    /// * `shnex:findFirst` — the FIRST input node is "the first conforming one".
    /// * `sh:filterShape` / `shnex:filterShape` — no candidate is filtered out.
    /// * `shnex:nodesMatching` — every node in the data graph matches.
    ///
    /// A shapes document with `shnex:matchAll ex:Typo` therefore loads green,
    /// validates green, and checks nothing. That is the exact defect `sh:condition`
    /// carried; this is the same answer applied at EVERY shape-valued operand
    /// rather than at one of them.
    ///
    /// The test is [`Parser::node_is_a_shape`](super::Parser::node_is_a_shape), so
    /// an inline anonymous shape (`[ sh:property [ … ] ]`), a top-level
    /// `sh:PropertyShape`, and an untyped node the shapes graph makes SHACL
    /// statements about are all still accepted — only a node the shapes graph never
    /// described as a shape at all is refused.
    fn parse_shape_operand(
        &mut self,
        shape_ref: Term,
        owner: &str,
        node: &Term,
    ) -> Result<Shape, String> {
        if !self.node_is_a_shape(&shape_ref) {
            return Err(format!(
                "{owner} node expression on {node} names {shape_ref}, which the shapes graph does \
                 not describe as a shape; an undefined shape is EMPTY, and every node conforms to \
                 an empty shape, so this check would hold vacuously rather than fail"
            ));
        }
        self.parse_inline_shape(shape_ref)
    }

    /// Refuse a node-expression key that the SELECTED expression kind does not
    /// read, so an authored operand is never silently discarded.
    ///
    /// `parse_node_expr_core` picks a kind from [`PRIMARY_KEYS`] and the arm for
    /// that kind then reads its OWN operand keys by name. Every other
    /// node-expression key on the node is, without this check, simply never
    /// looked at — accepted and dropped. The failure is invisible and it changes
    /// the answer:
    ///
    /// * `[ shnex:matchAll ex:S ; sh:nodes … ]` — `shnex:matchAll` reads
    ///   `shnex:nodes`, so the operand vanishes and the default (the focus node)
    ///   silently takes its place, checking a different node than the author wrote.
    /// * `[ shnex:if … ; sh:then … ; sh:else … ]` — `shnex:if` reads
    ///   `shnex:then`/`shnex:else`, so BOTH branches become the empty expression.
    /// * `[ shnex:orderBy … ; sh:desc true ]` — the direction flag is dropped and
    ///   the sort silently runs ascending, which flips the answer of any
    ///   `shnex:limit` above it.
    /// * `[ shnex:pathValues ex:p ; shnex:nodes … ]` — a path expression has no
    ///   `shnex:nodes` operand; the input is dropped and the path is walked from
    ///   the ambient focus node instead.
    ///
    /// Two classes of predicate are refused: a key that IS node-expression
    /// vocabulary ([`NON_FUNCTION_KEYS`]) but belongs to another kind or to the
    /// other SPELLING of this one, and any unrecognised term in the `shnex:`
    /// namespace — that namespace is entirely node-expression vocabulary and is
    /// fully enumerated in [`crate::model::shnex`], so a term outside it is a
    /// misspelling, not an extension point.
    ///
    /// Everything else on the node is left alone, which is what keeps this from
    /// becoming an over-refusal: `rdf:type`, the `sh:message` / `sh:severity` an
    /// expression constraint carries, and any application vocabulary an author
    /// hangs off the node all pass through untouched.
    fn check_expression_keys(&self, node: &Term, iri: &str, kind: ExprKind) -> Result<(), String> {
        // The SHACL-AF paging wrappers are peeled by `parse_node_expr_wrapped`
        // from ANY node, whatever core kind it carries, so they are always
        // accepted. `sh:desc` is a modifier OF `sh:orderby`, so it is accepted
        // only when the wrapper it modifies is actually present — otherwise it
        // would itself be a silently-dropped key.
        let mut accepted: Vec<&str> = vec![iri, sh::LIMIT, sh::OFFSET, sh::ORDERBY];
        if self.first_object_of(node, sh::ORDERBY).is_some() {
            accepted.push(sh::DESC);
        }
        match kind {
            ExprKind::Path => accepted.push(shnex::FOCUS_NODE),
            ExprKind::FilterShape => accepted.push(if iri == sh::FILTER_SHAPE {
                sh::NODES
            } else {
                shnex::NODES
            }),
            ExprKind::If => {
                if iri == sh::IF {
                    accepted.extend([sh::THEN, sh::ELSE]);
                } else {
                    accepted.extend([shnex::THEN, shnex::ELSE]);
                }
            }
            ExprKind::List => accepted.push(rdf::REST),
            ExprKind::Remove
            | ExprKind::Limit
            | ExprKind::Offset
            | ExprKind::FlatMap
            | ExprKind::FindFirst
            | ExprKind::MatchAll => accepted.push(shnex::NODES),
            ExprKind::OrderBy => accepted.extend([shnex::NODES, shnex::DESC]),
            ExprKind::Select => accepted.push(sh::PREFIXES),
            ExprKind::Union
            | ExprKind::Intersection
            | ExprKind::Concat
            | ExprKind::Count
            | ExprKind::Distinct
            | ExprKind::Min
            | ExprKind::Max
            | ExprKind::Sum
            | ExprKind::Exists
            | ExprKind::Var
            | ExprKind::InstancesOf
            | ExprKind::NodesMatching
            | ExprKind::ConformsToShape
            | ExprKind::Arg => {}
        }

        let owner = Self::key_label(iri);
        for (_, predicate, _) in
            native_quads(self.data, Some(node), None, None, GraphFilter::AnyGraph)
        {
            let p = predicate.as_str();
            if accepted.contains(&p) {
                continue;
            }
            if NON_FUNCTION_KEYS.contains(&p) {
                return Err(format!(
                    "{owner} node expression on {node} also carries <{p}>, which {owner} does not \
                     read; it would be silently discarded. Spell the operand the way the \
                     expression key is spelled, or remove it"
                ));
            }
            // `shnex:arg0`, `shnex:arg1`, … are a custom function's own argument
            // paths on a CALL site, not node-expression structure, so they are
            // never a misspelling of one.
            if p.starts_with(shnex::NS) && !p.starts_with(shnex::ARG) {
                return Err(format!(
                    "{owner} node expression on {node} carries <{p}>, which is not a term of the \
                     SHACL 1.2 Node Expressions vocabulary"
                ));
            }
        }
        Ok(())
    }

    /// The prefixed spelling of a node-expression key, for a diagnostic that quotes
    /// the key the writer actually authored rather than its full IRI.
    fn key_label(iri: &str) -> String {
        if let Some(local) = iri.strip_prefix(shnex::NS) {
            format!("shnex:{local}")
        } else if let Some(local) = iri.strip_prefix(sh::NS) {
            format!("sh:{local}")
        } else if let Some(local) = iri.strip_prefix("http://www.w3.org/1999/02/22-rdf-syntax-ns#")
        {
            format!("rdf:{local}")
        } else {
            format!("<{iri}>")
        }
    }

    /// The prefixed spelling of a `shnex:` key, for error messages that quote the
    /// key the writer actually authored.
    fn shnex_label(iri: &str) -> String {
        match iri.strip_prefix(shnex::NS) {
            Some(local) => format!("shnex:{local}"),
            None => format!("<{iri}>"),
        }
    }

    /// Dispatch a node carrying exactly one structural expression key.
    ///
    /// `iri` is the spelling actually authored (used verbatim in error messages, so
    /// a writer is told about the key they wrote); `kind` is the spelling-independent
    /// kind it denotes.
    #[expect(
        clippy::too_many_lines,
        reason = "one arm per node-expression kind; splitting it would hide the \
                  exhaustive kind-to-IR correspondence this match exists to show"
    )]
    fn parse_structural_node_expr(
        &mut self,
        node: &Term,
        iri: &str,
        kind: ExprKind,
    ) -> Result<NodeExpr, String> {
        // Every structural key has an object — `parse_node_expr_core` only reaches
        // here for a key it just observed — so a missing object is an internal
        // inconsistency, reported as such rather than papered over. Resolved once,
        // up front, because the borrow of `self` must end before the arms below
        // recurse through `&mut self`.
        let object = self
            .first_object_of(node, iri)
            .ok_or_else(|| format!("<{iri}> node expression on {node} lost its object"))?;
        match kind {
            // `sh:path` (SHACL-AF) / `shnex:pathValues` (§4.1.4). Without an
            // explicit focus expression both are the value nodes of the path from
            // the evaluation context's focus node — literally the same arm. With
            // `shnex:focusNode` the spec adds a single-node requirement on the
            // computed focus, which needs its own arm to keep that failure mode.
            ExprKind::Path => {
                let path_node = object;
                let path = self.parse_path(&path_node, node, &mut FastSet::default())?;
                match self.first_object_of(node, shnex::FOCUS_NODE) {
                    None => Ok(NodeExpr::Path(path)),
                    Some(focus_node) => Ok(NodeExpr::PathValues {
                        path,
                        focus: Box::new(self.parse_node_expr(&focus_node)?),
                    }),
                }
            }
            // `sh:filterShape` (SHACL-AF) / `shnex:filterShape` (§4.2.5). The
            // operand key follows the spelling of the expression key: `sh:nodes`
            // for the `sh:` surface, `shnex:nodes` for the `shnex:` one.
            ExprKind::FilterShape => {
                let shape_ref = object;
                // The operand key follows the spelling of the expression key, and
                // the error names it in the same prefixed form the writer used.
                let (nodes_key, nodes_label, owner) = if iri == sh::FILTER_SHAPE {
                    (sh::NODES, "sh:nodes", "sh:filterShape")
                } else {
                    (shnex::NODES, "shnex:nodes", "shnex:filterShape")
                };
                let nodes_obj = self.first_object_of(node, nodes_key).ok_or_else(|| {
                    format!("{owner} node expression on {node} requires {nodes_label}")
                })?;
                let inner = self.parse_node_expr(&nodes_obj)?;
                let shape = self.parse_shape_operand(shape_ref, owner, node)?;
                Ok(NodeExpr::Filter {
                    nodes: Box::new(inner),
                    shape: Box::new(shape),
                })
            }
            ExprKind::Union => Ok(NodeExpr::Union(self.parse_node_expr_list(node, iri)?)),
            ExprKind::Intersection => Ok(NodeExpr::Intersection(
                self.parse_node_expr_list(node, iri)?,
            )),
            // `shnex:concat` (§4.2.3) — the SEQUENCE analogue of `sh:union`:
            // operand order and duplicates are both significant.
            ExprKind::Concat => Ok(NodeExpr::Concat(self.parse_node_expr_list(node, iri)?)),
            // `sh:if` (SHACL-AF) / `shnex:if` (§4.1.6).
            ExprKind::If => {
                let cond = self.parse_node_expr(&object)?;
                let (then_key, else_key) = if iri == sh::IF {
                    (sh::THEN, sh::ELSE)
                } else {
                    (shnex::THEN, shnex::ELSE)
                };
                // Per spec a missing then/else branch yields the empty list, which
                // is exactly what `NodeExpr::Empty` (§4.1.1) evaluates to.
                let then = match self.first_object_of(node, then_key) {
                    Some(t) => self.parse_node_expr(&t)?,
                    None => NodeExpr::Empty,
                };
                let els = match self.first_object_of(node, else_key) {
                    Some(t) => self.parse_node_expr(&t)?,
                    None => NodeExpr::Empty,
                };
                Ok(NodeExpr::If {
                    cond: Box::new(cond),
                    then: Box::new(then),
                    els: Box::new(els),
                })
            }
            // `sh:count` (SHACL-AF) / `shnex:count` (§4.4.1). Distinct counting is
            // `[ sh:count [ sh:distinct <expr> ] ]`: an inner distinct lowers to
            // `Count { distinct: true, .. }`.
            ExprKind::Count => match self.parse_node_expr(&object)? {
                NodeExpr::Distinct(inner) => Ok(NodeExpr::Count {
                    distinct: true,
                    of: inner,
                }),
                other => Ok(NodeExpr::Count {
                    distinct: false,
                    of: Box::new(other),
                }),
            },
            ExprKind::Distinct => Ok(NodeExpr::Distinct(Box::new(self.parse_node_expr(&object)?))),
            ExprKind::Min => Ok(NodeExpr::Min(Box::new(self.parse_node_expr(&object)?))),
            ExprKind::Max => Ok(NodeExpr::Max(Box::new(self.parse_node_expr(&object)?))),
            ExprKind::Sum => Ok(NodeExpr::Sum(Box::new(self.parse_node_expr(&object)?))),
            // `sh:exists` (adopted SHACL-AF) / `shnex:exists` (§4.1.5): a
            // node-expression predicate — true iff its inner NODE EXPRESSION yields
            // at least one node for the focus. (A shape does not "produce nodes",
            // so the operand is an expression, not a shape.)
            ExprKind::Exists => Ok(NodeExpr::Exists(Box::new(self.parse_node_expr(&object)?))),

            // ── SHACL 1.2 Node Expressions: `shnex:`-only kinds ────────────────
            // §4.1.2 Var expression. The name must be a non-empty string literal
            // (`sh:datatype xsd:string`, `sh:minLength 1`).
            ExprKind::Var => {
                let name = match object {
                    Term::Literal(lit) => lit.value().to_owned(),
                    other => {
                        return Err(format!(
                            "shnex:var on {node} must be a string literal, got {other}"
                        ));
                    }
                };
                if name.is_empty() {
                    return Err(format!("shnex:var on {node} must not be the empty string"));
                }
                Ok(NodeExpr::Var(name))
            }
            // §4.1.3 List expression. Its members ARE the output nodes, so they are
            // taken as terms, not re-parsed as expressions; the spec restricts them
            // to IRIs and literals, and a blank-node member hard-fails rather than
            // silently becoming an unevaluated structure.
            ExprKind::List => {
                let members = self.walk_rdf_list(node, node)?;
                for member in &members {
                    // A BLANK NODE member is refused: it would be an unevaluated
                    // structure smuggled in as a value. An RDF 1.2 TRIPLE TERM is
                    // not — §3.1.3 makes it a first-class constant expression, this
                    // very file recognises a bare one as such, and `shnex:concat`
                    // carries triple terms through the sequence-valued kinds, so
                    // refusing one only here would make a triple term legal
                    // everywhere in the language EXCEPT inside a list.
                    if !matches!(
                        member,
                        Term::NamedNode(_) | Term::Literal(_) | Term::Triple(_)
                    ) {
                        return Err(format!(
                            "shnex:ListExpression member on {node} must be an IRI, a literal or a \
                             triple term, got {member}"
                        ));
                    }
                }
                Ok(NodeExpr::List(members))
            }
            // §4.2.4 Remove expression: `shnex:remove` names the removed nodes and
            // `shnex:nodes` the input; both are mandatory.
            ExprKind::Remove => {
                let remove = self.parse_node_expr(&object)?;
                let nodes = self.parse_shnex_nodes_required(node, "shnex:remove")?;
                Ok(NodeExpr::Remove {
                    nodes: Box::new(nodes),
                    remove: Box::new(remove),
                })
            }
            // §4.2.6 / §4.2.7 Limit and Offset expressions. Unlike the SHACL-AF
            // `sh:limit` / `sh:offset` keys — which WRAP the same node's own core
            // expression — the `shnex:` spellings are named-parameter functions
            // with their own `shnex:nodes` operand, so they are cores, not wrappers.
            // Both spellings still lower to the same `NodeExpr` arm.
            ExprKind::Limit | ExprKind::Offset => {
                let label = Self::shnex_label(iri);
                let n = crate::shapes::parse_u64(&object).ok_or_else(|| {
                    format!("{label} value is not a non-negative integer on {node}")
                })?;
                let of = Box::new(self.parse_shnex_nodes_required(node, &label)?);
                Ok(if matches!(kind, ExprKind::Limit) {
                    NodeExpr::Limit { of, n }
                } else {
                    NodeExpr::Offset { of, n }
                })
            }
            // §4.2.8 OrderBy expression, likewise a core rather than a wrapper.
            ExprKind::OrderBy => {
                let key = self.parse_node_expr(&object)?;
                let of = self.parse_shnex_nodes_required(node, "shnex:orderBy")?;
                let descending = self.parse_desc_flag(node, shnex::DESC)?;
                Ok(NodeExpr::OrderBy {
                    of: Box::new(of),
                    key: Box::new(key),
                    descending,
                })
            }
            // §4.3.1 FlatMap expression; `shnex:nodes` defaults to the focus node.
            ExprKind::FlatMap => {
                let map = self.parse_node_expr(&object)?;
                let nodes = self.parse_shnex_nodes_or_focus(node)?;
                Ok(NodeExpr::FlatMap {
                    nodes: Box::new(nodes),
                    map: Box::new(map),
                })
            }
            // §4.3.2 / §4.3.3 FindFirst and MatchAll: a SHAPE operand plus an
            // optional `shnex:nodes` defaulting to the focus node.
            ExprKind::FindFirst | ExprKind::MatchAll => {
                let shape =
                    Box::new(self.parse_shape_operand(object, &Self::key_label(iri), node)?);
                let nodes = Box::new(self.parse_shnex_nodes_or_focus(node)?);
                Ok(if matches!(kind, ExprKind::FindFirst) {
                    NodeExpr::FindFirst { nodes, shape }
                } else {
                    NodeExpr::MatchAll { nodes, shape }
                })
            }
            // §4.5.1 InstancesOf expression: the class is constrained to
            // `sh:nodeKind sh:IRI`.
            ExprKind::InstancesOf => match object {
                Term::NamedNode(class) => Ok(NodeExpr::InstancesOf(class)),
                other => Err(format!(
                    "shnex:instancesOf on {node} must be an IRI, got {other}"
                )),
            },
            // §4.5.2 NodesMatching expression.
            ExprKind::NodesMatching => Ok(NodeExpr::NodesMatching(Box::new(
                self.parse_shape_operand(object, "shnex:nodesMatching", node)?,
            ))),
            // SHACL 1.2 SPARQL Extensions §6.1 (`sh:select`, function name
            // `sh:SelectExpression`) and §6.2 (`sh:sparqlExpr`, function name
            // `sh:SPARQLExprExpression`). §6.2 defines itself as an abbreviation:
            // the expression embedded into `$PREFIXES$ SELECT ($EXPR$ AS ?result)
            // WHERE {}`, which the specification prints as the "equivalent
            // expanded form" of the matching `sh:select`. The expansion happens
            // HERE, once, so the two spellings share one IR arm and one evaluator
            // rather than two parallel query paths.
            ExprKind::Select => {
                let Term::Literal(body) = &object else {
                    return Err(format!(
                        "<{iri}> on {node} must be a string literal, got {object}"
                    ));
                };
                // SHACL-AF `sh:prefixes` may be declared on the expression node OR
                // on the shape that carries it — the same two owners `sh:sparql`
                // reads (see `parse_constraints`, which builds its header from
                // `&[id, &c_node]`). Honouring only the expression node made the
                // identical declaration fail here as an "unparsable query".
                let header = match self.current_shape.clone() {
                    Some(shape) => self.prefix_header(&[&shape, node]),
                    None => self.prefix_header(&[node]),
                };
                let (query, key) = if iri == sh::SELECT {
                    (format!("{header}{}", body.value()), "sh:select")
                } else {
                    (
                        format!("{header}SELECT (({}) AS ?result) WHERE {{}}", body.value()),
                        "sh:sparqlExpr",
                    )
                };
                let parsed = SparqlParser::new().parse_query(&query).map_err(|e| {
                    format!("{key} node expression on {node} has an unparsable query: {e}")
                })?;
                if !matches!(parsed, Query::Select { .. }) {
                    return Err(format!(
                        "{key} node expression on {node} must be a SELECT query"
                    ));
                }
                // §6.1: the query "must be a valid SPARQL 1.2 SELECT query
                // projecting exactly one variable" — whose bindings ARE the output
                // nodes, so a two-column projection has no answer to give and is
                // refused at load rather than silently reduced to its first column.
                let variable = single_projected_variable(&parsed)
                    .map_err(|e| format!("{key} node expression on {node}: {e}"))?;
                Ok(NodeExpr::Select {
                    query,
                    variable,
                    key,
                })
            }
            // §6.3 Arg expression. The key is `sh:or ( [ sh:nodeKind sh:IRI ]
            // [ sh:datatype xsd:integer ] )`: an IRI names a custom NAMED parameter
            // function's parameter (§6.1), an integer a custom LIST parameter
            // function's zero-based argument (§6.2). Anything else names no
            // argument at all and is refused here rather than silently resolving
            // to nothing at every call.
            ExprKind::Arg => match &object {
                Term::NamedNode(name) => Ok(NodeExpr::Arg(ArgKey::Named(name.as_str().to_owned()))),
                Term::Literal(lit) => {
                    match purrdf_xsd::parse_by_iri(lit.value(), lit.datatype_str()) {
                        Ok(Some(purrdf_xsd::XsdValue::Integer { value, .. })) if value >= 0 => {
                            Ok(NodeExpr::Arg(ArgKey::Index(u64::try_from(value).map_err(
                                |e| format!("shnex:arg index {value} on {node} is not usable: {e}"),
                            )?)))
                        }
                        _ => Err(format!(
                            "shnex:arg on {node} must be an IRI or a non-negative xsd:integer, \
                             got {object}"
                        )),
                    }
                }
                other => Err(format!(
                    "shnex:arg on {node} must be an IRI or a non-negative xsd:integer, got {other}"
                )),
            },
            // §4.5.3 ConformsToShape expression: a LIST parameter function whose
            // argument list has exactly two members — the node expression under
            // test and an expression producing the shape IRI. The spec constrains
            // that second argument to `sh:nodeKind sh:IRI` ("Must produce the IRI
            // of a well-formed shape"), and this parser requires it to BE that IRI
            // so the shape is resolved once, here, against the shapes graph.
            ExprKind::ConformsToShape => {
                let members = self.walk_rdf_list(&object, node)?;
                let [node_arg, shape_arg] = members.as_slice() else {
                    return Err(format!(
                        "shnex:conformsToShape on {node} requires a list of exactly two members, \
                         got {}",
                        members.len()
                    ));
                };
                let Term::NamedNode(_) = shape_arg else {
                    return Err(format!(
                        "shnex:conformsToShape on {node} requires its second argument to be the \
                         IRI of a shape, got {shape_arg}"
                    ));
                };
                Ok(NodeExpr::ConformsToShape {
                    node: Box::new(self.parse_node_expr(node_arg)?),
                    shape: Box::new(self.parse_shape_operand(
                        shape_arg.clone(),
                        "shnex:conformsToShape",
                        node,
                    )?),
                })
            }
        }
    }

    /// Recognise and parse a custom NAMED parameter function call
    /// (SHACL 1.2 Node Expressions §6.1), or return `Ok(None)` when `candidates`
    /// mentions no key parameter and the node is therefore some other kind of call.
    ///
    /// The call site is a blank node whose predicates are the function's parameter
    /// `sh:path` IRIs — `[ ex:average [ shnex:pathValues ( ex:employee ex:income ) ] ]`.
    /// It never names the function, so recognition goes through the key-parameter
    /// index §6.1 requires to be disjoint across functions.
    ///
    /// # Errors
    ///
    /// Hard-fails when the node's key parameters identify TWO different functions
    /// (the node would then be two calls at once), and when a predicate is not one
    /// of the identified function's declared parameters — a mis-spelled parameter
    /// would otherwise bind nothing and evaluate to the empty list at every call.
    fn parse_named_parameter_call(
        &mut self,
        node: &Term,
        candidates: &[(NamedNode, Term)],
    ) -> Result<Option<NodeExpr>, String> {
        // Nothing declared: skip the whole question without touching the index.
        if self.custom_fns.is_empty() {
            return Ok(None);
        }
        let mut func: Option<Arc<CustomFunction>> = None;
        for (predicate, _) in candidates {
            let Some(candidate) = self.custom_fns.by_key_parameter(predicate.as_str()) else {
                continue;
            };
            match &func {
                Some(existing) if existing.iri != candidate.iri => {
                    return Err(format!(
                        "node expression on {node} carries the key parameters of two different \
                         custom named-parameter functions, <{}> and <{}>",
                        existing.iri.as_str(),
                        candidate.iri.as_str()
                    ));
                }
                _ => func = Some(Arc::clone(candidate)),
            }
        }
        let Some(func) = func else {
            return Ok(None);
        };
        let mut bound: Vec<(ArgKey, NodeExpr)> = Vec::with_capacity(candidates.len());
        for (predicate, object) in candidates {
            let key = ArgKey::Named(predicate.as_str().to_owned());
            if !func.params.contains(&key) {
                return Err(format!(
                    "node expression on {node} calls <{}> with <{}>, which is not one of its \
                     declared sh:parameter paths",
                    func.iri.as_str(),
                    predicate.as_str()
                ));
            }
            bound.push((key, self.parse_node_expr(object)?));
        }
        Ok(Some(NodeExpr::CustomCall { func, args: bound }))
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

    /// Parse a node carrying no structural key: an empty expression, a function
    /// call, or a plain constant IRI.
    fn parse_call_or_constant(&mut self, node: &Term) -> Result<NodeExpr, String> {
        // A function-call node expression is always a blank node `[ <fn> ( … ) ]`.
        // A NamedNode reaching here (not a literal, not sh:this, no structural key)
        // is therefore a plain constant IRI — even when it bears unrelated outgoing
        // triples in the shapes graph (e.g. an `rdfs:label`). SHACL 1.2 Node
        // Expressions §3.1.1 states this directly: an IRI expression evaluates to
        // itself.
        if matches!(node, Term::NamedNode(_)) {
            return Ok(NodeExpr::Constant(node.clone()));
        }
        // Every outgoing triple of the node, before any filtering — the empty
        // expression is defined by their TOTAL absence, so it must be decided here
        // rather than after the structural keys are filtered out.
        let outgoing = native_quads(self.data, Some(node), None, None, GraphFilter::AnyGraph);
        if outgoing.is_empty() {
            // SHACL 1.2 Node Expressions §4.1.1: "A blank node that is not the
            // subject of any triple is called an empty expression"; its output
            // nodes are the empty list. Only blank nodes reach here (IRIs returned
            // above as constants, literals and triple terms in the caller), so this
            // is exactly the spec's condition.
            return Ok(NodeExpr::Empty);
        }
        // Gather the candidate (function IRI, arg-list head) triples, ignoring
        // rdf:type (a classification triple), every SHACL structural key, and the
        // ANNOTATIONS an expression node legitimately carries.
        let mut candidates: Vec<(NamedNode, Term)> = outgoing
            .into_iter()
            .filter(|(_, predicate, _)| {
                let p = predicate.as_str();
                p != rdf::TYPE
                    && !NON_FUNCTION_KEYS.contains(&p)
                    && !CALL_SITE_ANNOTATIONS.contains(&p)
            })
            .map(|(_, predicate, object)| (predicate, object))
            .collect();
        candidates.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));
        candidates.dedup_by(|a, b| a.0 == b.0 && a.1 == b.1);

        if candidates.is_empty() {
            // The node bears triples, but every one of them is a structural key or
            // an rdf:type — so it names no expression kind and calls no function.
            return Err(format!(
                "unrecognised node expression on {node}: no SHACL node-expression key and no \
                 function call"
            ));
        }
        // A custom NAMED parameter function call (SHACL 1.2 Node Expressions §6.1) is
        // recognised by a KEY parameter, not by the function IRI: the call site
        // `[ ex:average <expr> ]` never mentions `ex:AverageExpression` at all. It is
        // therefore decided BEFORE the one-predicate rule below — such a call carries
        // one predicate per supplied argument, so more than one is normal rather than
        // ambiguous.
        if let Some(call) = self.parse_named_parameter_call(node, &candidates)? {
            return Ok(call);
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
        // SHACL 1.2 Node Expressions gives the object of a function-call predicate
        // THREE forms, and all three are read here:
        //
        // 1. `rdf:nil` — the empty argument list (`[ sparql:now () ]`).
        // 2. A well-formed SHACL list — the arguments in order
        //    (`[ sparql:plus ( 38 4 ) ]`).
        // 3. A single well-formed node expression given WITHOUT the list, which the
        //    specification defines as "equivalent to a function call with a list of
        //    one element" and prints as `[ sparql:abs -42 ]`.
        //
        // Form 3 is why this is not "must carry an RDF list": refusing it would
        // reject a document copied verbatim out of the specification. A malformed
        // object is still refused — it simply fails as the node expression it
        // claims to be, with that parse's own diagnostic, rather than under a
        // blanket rule about lists.
        let nil = Term::NamedNode(NamedNode::from(rdf::NIL));
        let is_list = args_head == nil || self.first_object_of(&args_head, rdf::FIRST).is_some();
        let items = if is_list {
            self.walk_rdf_list(&args_head, node)?
        } else {
            vec![args_head]
        };
        let mut args = Vec::with_capacity(items.len());
        for item in items {
            args.push(self.parse_node_expr(&item)?);
        }
        // A custom LIST parameter function call (SHACL 1.2 Node Expressions §6.2):
        // the function's own IRI IS its list parameter property, so the call site is
        // shaped exactly like every other `[ <fn> ( … ) ]` and is told apart by the
        // declaration the shapes graph carries. Checked before the SPARQL/builtin
        // routes so a declared function can never be mistaken for one of them.
        if let Some(func) = self.custom_fns.get(fn_iri.as_str()) {
            if matches!(func.kind, CustomFnKind::NamedParameter) {
                return Err(format!(
                    "<{}> on {node} is a sh:NamedParameterExpressionFunction and has no positional \
                     call form; supply its arguments under its parameters' own sh:path IRIs",
                    fn_iri.as_str()
                ));
            }
            // Arity is decided HERE, at load, because both the declaration and the
            // call are in hand: a call with too few or too many arguments would
            // otherwise read a missing `shnex:arg` as the empty list and quietly
            // contribute nothing.
            if args.len() < func.required || args.len() > func.params.len() {
                return Err(format!(
                    "sh:ListParameterExpressionFunction <{}> called on {node} with {} argument(s), \
                     but it declares {}..={}",
                    fn_iri.as_str(),
                    args.len(),
                    func.required,
                    func.params.len()
                ));
            }
            let mut bound: Vec<(ArgKey, NodeExpr)> = Vec::with_capacity(args.len());
            for (index, arg) in args.into_iter().enumerate() {
                let key = u64::try_from(index)
                    .map_err(|e| format!("argument index {index} on {node} is not usable: {e}"))?;
                bound.push((ArgKey::Index(key), arg));
            }
            return Ok(NodeExpr::CustomCall {
                func: Arc::clone(func),
                args: bound,
            });
        }
        // SHACL 1.2 Node Expressions §5: an IRI of the W3C SPARQL 1.2 term
        // vocabulary in call position IS the corresponding SPARQL function. The
        // name is resolved and the SPARQL surface text rendered ONCE, here, then
        // parse-checked — so an unknown name and a wrong-arity call are both
        // shapes-load failures, never a per-focus surprise or a silent empty
        // result.
        if let Some(local) = fn_iri.as_str().strip_prefix(sparql_ns::NS) {
            let form = sparql_ns_lowering(local)
                .map_err(|e| format!("SPARQL function node expression on {node}: {e}"))?;
            let rendered = form
                .render(fn_iri.as_str(), args.len())
                .map_err(|e| format!("SPARQL function node expression on {node}: {e}"))?;
            // The rendered text is evaluated as the projection of
            // `SELECT ((expr) AS ?result) WHERE {}` (see `crate::sparql::eval_scalar_expr`),
            // so it is validated in exactly that position.
            let probe = format!("SELECT (({rendered}) AS ?result) WHERE {{}}");
            SparqlParser::new().parse_query(&probe).map_err(|e| {
                format!(
                    "SPARQL function node expression <{}> on {node} does not render a valid SPARQL \
                     expression ({rendered}): {e}",
                    fn_iri.as_str()
                )
            })?;
            return Ok(NodeExpr::Call(FnCall::Sparql {
                iri: fn_iri,
                expr: rendered,
                args,
            }));
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

/// A node-expression KIND, independent of which spec surface spelled it.
///
/// SHACL Advanced Features and SHACL 1.2 Node Expressions give several of the same
/// operations two IRIs (`sh:union` / `shnex:concat` aside, which are genuinely
/// different operations). This enum is the one name each operation has inside
/// PurRDF: [`PRIMARY_KEYS`] maps every accepted IRI onto a kind, every kind lowers
/// to one [`NodeExpr`] arm, and that arm has exactly one evaluator. Nothing here is
/// conditional or feature-gated — two spec-defined surfaces, one implementation,
/// exactly as two RDF syntaxes parse to one graph model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExprKind {
    /// `sh:path` / `shnex:pathValues` — path value nodes.
    Path,
    /// `sh:filterShape` / `shnex:filterShape` — shape-filtered nodes.
    FilterShape,
    /// `sh:union` — SHACL-AF set union (no `shnex:` spelling exists).
    Union,
    /// `sh:intersection` / `shnex:intersection` — set intersection.
    Intersection,
    /// `shnex:concat` — sequence concatenation (no `sh:` spelling exists).
    Concat,
    /// `sh:if` / `shnex:if` — conditional.
    If,
    /// `sh:count` / `shnex:count` — cardinality.
    Count,
    /// `sh:distinct` / `shnex:distinct` — duplicate elimination.
    Distinct,
    /// `sh:min` / `shnex:min` — minimum.
    Min,
    /// `sh:max` / `shnex:max` — maximum.
    Max,
    /// `sh:sum` / `shnex:sum` — sum.
    Sum,
    /// `sh:exists` / `shnex:exists` — existence predicate.
    Exists,
    /// `shnex:var` — a scope/focus variable reference.
    Var,
    /// `rdf:first` — an RDF collection read as a `shnex:ListExpression`.
    List,
    /// `shnex:remove` — set difference preserving input order.
    Remove,
    /// `shnex:limit` — the named-parameter limit expression.
    Limit,
    /// `shnex:offset` — the named-parameter offset expression.
    Offset,
    /// `shnex:orderBy` — the named-parameter order-by expression.
    OrderBy,
    /// `shnex:flatMap` — per-node mapping with concatenation.
    FlatMap,
    /// `shnex:findFirst` — the first conforming input node.
    FindFirst,
    /// `shnex:matchAll` — whether every input node conforms.
    MatchAll,
    /// `shnex:instancesOf` — the SHACL instances of a class.
    InstancesOf,
    /// `shnex:nodesMatching` — every conforming node of the focus graph.
    NodesMatching,
    /// `shnex:conformsToShape` — a two-argument conformance predicate.
    ConformsToShape,
    /// `sh:select` / `sh:sparqlExpr` — a SPARQL-based node expression.
    Select,
    /// `shnex:arg` — an argument reference inside a custom function's body.
    Arg,
}

/// The single variable a SHACL 1.2 SPARQL-based node expression's SELECT query
/// projects (SPARQL Extensions §6.1).
///
/// The solution modifiers wrap the projection in the algebra, so they are peeled
/// exactly as `crate::prebinding`'s own walk peels them to reach the outermost
/// `Project`.
fn single_projected_variable(query: &Query) -> Result<String, String> {
    let Query::Select { pattern, .. } = query else {
        return Err("the query is not a SELECT".to_owned());
    };
    let mut node = pattern;
    loop {
        match node {
            GraphPattern::Slice { inner, .. }
            | GraphPattern::Distinct { inner }
            | GraphPattern::Reduced { inner }
            | GraphPattern::OrderBy { inner, .. } => node = inner,
            GraphPattern::Project { variables, .. } => {
                let [only] = variables.as_slice() else {
                    return Err(format!(
                        "the SELECT query must project exactly one variable, it projects {}",
                        variables.len()
                    ));
                };
                return Ok(only.as_str().to_owned());
            }
            _ => {
                return Err(
                    "the SELECT query has no projection, so it names no output variable".to_owned(),
                );
            }
        }
    }
}

/// Every IRI that identifies a node-expression kind, with the kind it identifies.
///
/// Both spec surfaces appear here, so `parse_node_expr_core`'s single
/// "exactly one key" check rejects a node carrying two DIFFERENT kinds and a node
/// carrying the two spellings of the SAME kind with the same message — the writer
/// is asked which they meant rather than silently given one of them.
///
/// `sh:limit` / `sh:offset` / `sh:orderby` are deliberately ABSENT: on the
/// SHACL-AF surface those keys WRAP the node's own core expression (peeled by
/// `parse_node_expr_wrapped`), whereas their `shnex:` counterparts are
/// named-parameter functions carrying their own `shnex:nodes` operand and so are
/// cores in their own right. Both spellings still lower to the same `NodeExpr` arm.
static PRIMARY_KEYS: &[(&str, ExprKind)] = &[
    // SHACL Advanced Features spellings.
    (sh::PATH, ExprKind::Path),
    (sh::FILTER_SHAPE, ExprKind::FilterShape),
    (sh::UNION, ExprKind::Union),
    (sh::INTERSECTION, ExprKind::Intersection),
    (sh::IF, ExprKind::If),
    (sh::COUNT, ExprKind::Count),
    (sh::DISTINCT, ExprKind::Distinct),
    (sh::MIN, ExprKind::Min),
    (sh::MAX, ExprKind::Max),
    (sh::SUM, ExprKind::Sum),
    (sh::EXISTS, ExprKind::Exists),
    // SHACL 1.2 Node Expressions spellings.
    (shnex::PATH_VALUES, ExprKind::Path),
    (shnex::FILTER_SHAPE, ExprKind::FilterShape),
    (shnex::INTERSECTION, ExprKind::Intersection),
    (shnex::CONCAT, ExprKind::Concat),
    (shnex::IF, ExprKind::If),
    (shnex::COUNT, ExprKind::Count),
    (shnex::DISTINCT, ExprKind::Distinct),
    (shnex::MIN, ExprKind::Min),
    (shnex::MAX, ExprKind::Max),
    (shnex::SUM, ExprKind::Sum),
    (shnex::EXISTS, ExprKind::Exists),
    (shnex::VAR, ExprKind::Var),
    (rdf::FIRST, ExprKind::List),
    (shnex::REMOVE, ExprKind::Remove),
    (shnex::LIMIT, ExprKind::Limit),
    (shnex::OFFSET, ExprKind::Offset),
    (shnex::ORDER_BY, ExprKind::OrderBy),
    (shnex::FLAT_MAP, ExprKind::FlatMap),
    (shnex::FIND_FIRST, ExprKind::FindFirst),
    (shnex::MATCH_ALL, ExprKind::MatchAll),
    (shnex::INSTANCES_OF, ExprKind::InstancesOf),
    (shnex::NODES_MATCHING, ExprKind::NodesMatching),
    (shnex::CONFORMS_TO_SHAPE, ExprKind::ConformsToShape),
    (shnex::ARG, ExprKind::Arg),
    // SHACL 1.2 SPARQL Extensions spellings.
    (sh::SELECT, ExprKind::Select),
    (sh::SPARQL_EXPR, ExprKind::Select),
];

/// Every vocabulary term that structures a node expression — none of them can be
/// the predicate of a function-call expression, so `parse_call_or_constant` must
/// not mistake one for a function IRI.
///
/// This is the union of [`PRIMARY_KEYS`], the operand/modifier keys of both
/// surfaces (`sh:nodes`, `sh:then`, `shnex:nodes`, `shnex:desc`, …) and the
/// SHACL-AF paging wrappers.
static NON_FUNCTION_KEYS: &[&str] = &[
    // SHACL Advanced Features.
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
    // SHACL 1.2 Node Expressions.
    shnex::VAR,
    shnex::PATH_VALUES,
    shnex::FOCUS_NODE,
    shnex::EXISTS,
    shnex::IF,
    shnex::THEN,
    shnex::ELSE,
    shnex::DISTINCT,
    shnex::INTERSECTION,
    shnex::CONCAT,
    shnex::REMOVE,
    shnex::NODES,
    shnex::FILTER_SHAPE,
    shnex::LIMIT,
    shnex::OFFSET,
    shnex::ORDER_BY,
    shnex::DESC,
    shnex::FLAT_MAP,
    shnex::FIND_FIRST,
    shnex::MATCH_ALL,
    shnex::COUNT,
    shnex::MIN,
    shnex::MAX,
    shnex::SUM,
    shnex::INSTANCES_OF,
    shnex::NODES_MATCHING,
    shnex::CONFORMS_TO_SHAPE,
    shnex::ARG,
    // SHACL 1.2 SPARQL Extensions: the two SPARQL-based expression keys and the
    // `sh:prefixes` each may carry (§6.1 / §6.2).
    sh::SELECT,
    sh::SPARQL_EXPR,
    sh::PREFIXES,
    // RDF collection cells — a list expression's own structure, never a call.
    rdf::FIRST,
    rdf::REST,
];

/// The SHACL annotations an expression node may carry ALONGSIDE its expression,
/// which therefore never name a function.
///
/// `parse_constraints` reads `sh:message` and `sh:severity` off the expression
/// node itself for both `sh:expression` and `sh:nodeByExpression`, so they are
/// authored there routinely. Without this table they would be counted as
/// candidate function predicates, and the entirely legal
/// `[ sparql:strlen ( … ) ; sh:message "…" ]` would fail to load as an "ambiguous
/// function-call node expression" — while the same annotation on a STRUCTURAL
/// expression (`[ sh:count … ; sh:message "…" ]`) loads fine, because that path
/// short-circuits before the candidate scan. That asymmetry was the bug.
///
/// They are deliberately NOT in [`NON_FUNCTION_KEYS`]: that table is the
/// node-expression VOCABULARY, and `check_expression_keys` refuses a member of it
/// that the selected kind does not read. An annotation is not vocabulary and must
/// stay ignorable everywhere.
static CALL_SITE_ANNOTATIONS: &[&str] = &[sh::MESSAGE, sh::SEVERITY, sh::DEACTIVATED];

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

#[cfg(test)]
mod tests {
    use super::{ExprKind, NON_FUNCTION_KEYS, PRIMARY_KEYS};
    use crate::model::{sh, shnex};
    use std::collections::BTreeSet;

    /// The kinds `PRIMARY_KEYS` gives BOTH a `sh:` and a `shnex:` spelling, as
    /// the table itself defines them.
    fn dual_spelled() -> BTreeSet<&'static str> {
        // (kind, has a `sh:` spelling, the `shnex:` local name if it has one).
        // `ExprKind` is deliberately neither `Ord` nor `Hash`, so the grouping is
        // a linear scan over a table of ~35 entries rather than a map.
        let mut surfaces: Vec<(ExprKind, bool, Option<&'static str>)> = Vec::new();
        for &(iri, kind) in PRIMARY_KEYS {
            let slot = match surfaces.iter_mut().find(|(k, _, _)| *k == kind) {
                Some(slot) => slot,
                None => {
                    surfaces.push((kind, false, None));
                    surfaces
                        .last_mut()
                        .expect("just pushed a surface entry for this kind")
                }
            };
            if iri.starts_with(sh::NS) {
                slot.1 = true;
            } else if let Some(local) = iri.strip_prefix(shnex::NS) {
                slot.2 = Some(local);
            }
        }
        surfaces
            .into_iter()
            .filter_map(|(_, has_af, shnex_local)| shnex_local.filter(|_| has_af))
            .collect()
    }

    /// The set of DUAL-SPELLED kinds is pinned EXACTLY, so a new one cannot be
    /// added without also being added to the integration test that proves the two
    /// spellings agree.
    ///
    /// `sh_and_shnex_spellings_produce_identical_results` (in
    /// `tests/node_expressions.rs`) is a hand-written array of fixture pairs; a
    /// twelfth dual-spelled kind would simply not appear in it and nothing would
    /// notice. This is the binding: the array covers exactly the names below, and
    /// growing the table without growing the array fails HERE, naming the kind.
    #[test]
    fn the_dual_spelled_kinds_are_exactly_these() {
        let expected: BTreeSet<&str> = [
            "count",
            "distinct",
            "exists",
            "filterShape",
            "if",
            "intersection",
            "max",
            "min",
            "pathValues",
            "sum",
        ]
        .into_iter()
        .collect();
        assert_eq!(
            dual_spelled(),
            expected,
            "the dual-spelled kinds changed — add the new one to \
             `sh_and_shnex_spellings_produce_identical_results` in \
             tests/node_expressions.rs, then update this list"
        );
    }

    /// Every IRI in `PRIMARY_KEYS` is also in `NON_FUNCTION_KEYS`.
    ///
    /// `parse_call_or_constant` decides "is this a function call?" by subtracting
    /// `NON_FUNCTION_KEYS` from the node's predicates, and `check_expression_keys`
    /// decides "is this key vocabulary?" the same way. A primary key missing from
    /// that table would therefore be read as a FUNCTION IRI — a structural key
    /// silently reinterpreted as a call.
    #[test]
    fn every_primary_key_is_node_expression_vocabulary() {
        let non_function: BTreeSet<&str> = NON_FUNCTION_KEYS.iter().copied().collect();
        let missing: Vec<&str> = PRIMARY_KEYS
            .iter()
            .map(|&(iri, _)| iri)
            .filter(|iri| !non_function.contains(iri))
            .collect();
        assert!(
            missing.is_empty(),
            "these expression keys would be mistaken for function IRIs: {missing:?}"
        );
    }
}

// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Parsing for SHACL-AF rules (`sh:rule`): `sh:TripleRule` and `sh:SPARQLRule`.

use purrdf_sparql_algebra::{Query, SparqlParser};

use crate::model::sh;
use crate::rules::{OrderKey, Rule, RuleBody};
use crate::term::Term;

use crate::shapes::Parser;

impl Parser<'_> {
    /// Parse every `sh:rule` attached to shape `id` into a [`Rule`], in stable
    /// (rule-node string) order.
    ///
    /// # Errors
    ///
    /// Hard-fails on a malformed rule — a rule that is neither a `sh:TripleRule`
    /// nor a `sh:SPARQLRule`, one that is ambiguously both, a `sh:TripleRule`
    /// missing one of `sh:subject`/`sh:predicate`/`sh:object`, a `sh:SPARQLRule`
    /// whose `sh:construct` is missing / unparsable / not a CONSTRUCT / violates
    /// the pre-binding restrictions, or a non-numeric `sh:order`.
    pub(crate) fn parse_rules(&mut self, id: &Term) -> Result<Vec<Rule>, String> {
        let mut rule_nodes: Vec<Term> = self.objects_of(id, sh::RULE);
        crate::term::sort_terms_canonical(&mut rule_nodes);
        let mut rules: Vec<Rule> = Vec::with_capacity(rule_nodes.len());
        for rule_node in rule_nodes {
            rules.push(self.parse_rule(id, &rule_node)?);
        }
        Ok(rules)
    }

    /// Parse a single `sh:rule` node into a [`Rule`].
    fn parse_rule(&mut self, shape_id: &Term, rule_node: &Term) -> Result<Rule, String> {
        let deactivated = self
            .first_object_of(rule_node, sh::DEACTIVATED)
            .is_some_and(|t| matches!(&t, Term::Literal(lit) if lit.value() == "true"));

        let order = match self.first_object_of(rule_node, sh::ORDER) {
            None => None,
            Some(Term::Literal(lit)) => {
                let value = lit.value().parse::<f64>().map_err(|_| {
                    format!(
                        "sh:order on rule {rule_node} must be a numeric literal, got \"{}\"",
                        lit.value()
                    )
                })?;
                Some(OrderKey::new(value))
            }
            Some(other) => {
                return Err(format!(
                    "sh:order on rule {rule_node} must be a numeric literal, got {other}"
                ));
            }
        };

        let mut conditions: Vec<Term> = self.objects_of(rule_node, sh::CONDITION);
        crate::term::sort_terms_canonical(&mut conditions);

        // Dispatch on rule kind: an explicit rdf:type OR the presence of the
        // kind's structural keys. A node that is both (or neither) is malformed.
        let is_triple_type = self.has_type(rule_node, sh::TRIPLE_RULE);
        let is_sparql_type = self.has_type(rule_node, sh::SPARQL_RULE);
        let has_spo = self.first_object_of(rule_node, sh::SUBJECT).is_some()
            || self.first_object_of(rule_node, sh::PREDICATE).is_some()
            || self.first_object_of(rule_node, sh::OBJECT).is_some();
        let has_construct = self.first_object_of(rule_node, sh::CONSTRUCT).is_some();

        let is_triple = is_triple_type || has_spo;
        let is_sparql = is_sparql_type || has_construct;

        let body = match (is_triple, is_sparql) {
            (true, true) => {
                return Err(format!(
                    "rule {rule_node} on shape {shape_id} is ambiguous: it looks like both a \
                     sh:TripleRule (sh:subject/predicate/object) and a sh:SPARQLRule (sh:construct)"
                ));
            }
            (true, false) => self.parse_triple_rule(shape_id, rule_node)?,
            (false, true) => self.parse_sparql_rule(shape_id, rule_node)?,
            (false, false) => {
                return Err(format!(
                    "rule {rule_node} on shape {shape_id} is not a recognised SHACL rule: it is \
                     neither a sh:TripleRule (sh:subject/predicate/object) nor a sh:SPARQLRule \
                     (sh:construct)"
                ));
            }
        };

        Ok(Rule {
            id: rule_node.clone(),
            body,
            conditions,
            order,
            deactivated,
        })
    }

    /// Parse a `sh:TripleRule` head (`sh:subject`/`sh:predicate`/`sh:object` node
    /// expressions — all three required).
    fn parse_triple_rule(&mut self, shape_id: &Term, rule_node: &Term) -> Result<RuleBody, String> {
        let subject_node = self
            .first_object_of(rule_node, sh::SUBJECT)
            .ok_or_else(|| {
                format!("sh:TripleRule {rule_node} on shape {shape_id} is missing sh:subject")
            })?;
        let predicate_node = self
            .first_object_of(rule_node, sh::PREDICATE)
            .ok_or_else(|| {
                format!("sh:TripleRule {rule_node} on shape {shape_id} is missing sh:predicate")
            })?;
        let object_node = self.first_object_of(rule_node, sh::OBJECT).ok_or_else(|| {
            format!("sh:TripleRule {rule_node} on shape {shape_id} is missing sh:object")
        })?;

        let subject = self.parse_node_expr(&subject_node)?;
        let predicate = self.parse_node_expr(&predicate_node)?;
        let object = self.parse_node_expr(&object_node)?;

        Ok(RuleBody::Triple {
            subject,
            predicate,
            object,
        })
    }

    /// Parse a `sh:SPARQLRule` head (a `sh:construct` CONSTRUCT query). The query
    /// is validated (parseable + CONSTRUCT-form + pre-binding-legal) at load time;
    /// the `$this`-bearing prefix header is prepended.
    fn parse_sparql_rule(&self, shape_id: &Term, rule_node: &Term) -> Result<RuleBody, String> {
        let raw = self
            .first_object_of(rule_node, sh::CONSTRUCT)
            .and_then(|t| match t {
                Term::Literal(lit) => Some(lit.value().to_owned()),
                _ => None,
            })
            .ok_or_else(|| {
                format!(
                    "sh:SPARQLRule {rule_node} on shape {shape_id} is missing a sh:construct \
                     string literal"
                )
            })?;
        // SHACL-AF sh:prefixes may be declared on the shape or the rule node.
        let construct = format!("{}{raw}", self.prefix_header(&[shape_id, rule_node]));

        match SparqlParser::new().parse_query(&construct) {
            Ok(query @ Query::Construct { .. }) => {
                // The query runs with $this pre-bound to each focus node; the
                // SHACL-SPARQL §5.2.1 pre-binding restrictions reject an illegal
                // body (MINUS/SERVICE/VALUES, `AS $this`, …) as a hard failure.
                crate::prebinding::check_construct(&query, &["this"])
                    .map_err(|e| format!("sh:SPARQLRule {rule_node} on shape {shape_id}: {e}"))?;
            }
            Ok(_) => {
                return Err(format!(
                    "sh:SPARQLRule {rule_node} on shape {shape_id} must be a CONSTRUCT query"
                ));
            }
            Err(e) => {
                return Err(format!(
                    "sh:SPARQLRule {rule_node} on shape {shape_id} has an unparsable \
                     sh:construct query: {e}"
                ));
            }
        }

        Ok(RuleBody::Sparql { construct })
    }
}

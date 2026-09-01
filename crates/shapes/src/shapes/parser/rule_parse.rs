// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Parsing for SHACL-AF rules (`sh:rule`): `sh:TripleRule` and `sh:SPARQLRule`.

use purrdf_sparql_algebra::{Query, SparqlParser};

use crate::model::sh;
use crate::rules::{
    OrderKey, Rule, RuleBody, RuleSchedule, construct_template_mints_blank, node_expr_mints_blank,
};
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
    /// the pre-binding restrictions, or a non-numeric / non-finite `sh:order`.
    pub(crate) fn parse_rules(&mut self, id: &Term) -> Result<Vec<Rule>, String> {
        // Inside a `sh:condition`'s own shape parse, rules are not read: a
        // condition is checked by `conforms`, which never looks at them. See
        // `Parser::parse_rules_enabled`.
        if !self.parse_rules_enabled {
            return Ok(Vec::new());
        }
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
                // `sh:order` is decimal-valued, and the layered scheduler uses it
                // to PARTITION execution into strata. `NaN` and `±INF` parse as
                // `f64` but denote no decimal and no position in the stratum
                // order (`NaN` is not even equal to itself, so "same stratum"
                // would be undefined for it). Refuse rather than schedule a rule
                // at an unresolvable position.
                if !value.is_finite() {
                    return Err(format!(
                        "sh:order on rule {rule_node} must be a finite decimal, got \"{}\"",
                        lit.value()
                    ));
                }
                Some(OrderKey::new(value))
            }
            Some(other) => {
                return Err(format!(
                    "sh:order on rule {rule_node} must be a numeric literal, got {other}"
                ));
            }
        };

        // `sh:condition` is RESOLVED HERE, at shapes-load, exactly as
        // `sh:filterShape` already is: a condition is a SHAPE, so the parser owns
        // it, and an unresolvable one is a load failure.
        //
        // Resolving it at firing time instead was wrong twice over. It looked the
        // condition up by IRI STRING in an index of top-level node shapes, so an
        // INLINE condition (`sh:condition [ sh:property [ … ] ]`) and a top-level
        // `sh:PropertyShape` condition — both perfectly legal SHACL — were absent
        // from that index and refused. And the lookup only ran once the owning
        // shape had produced a focus node, so a rule whose shape targets nothing
        // never reached it: a nonsense `sh:condition` on an untargeted shape LOADED
        // GREEN and the rule entailed as if the condition had held. A rule must
        // never fire on a condition that was not evaluated.
        let mut condition_nodes: Vec<Term> = self.objects_of(rule_node, sh::CONDITION);
        crate::term::sort_terms_canonical(&mut condition_nodes);
        let conditions = self.parse_conditions(shape_id, rule_node, condition_nodes)?;

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

        let (body, schedule) = match (is_triple, is_sparql) {
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
            schedule,
        })
    }

    /// Resolve every `sh:condition` node of a rule into a parsed [`Shape`].
    ///
    /// The sub-parse runs with a FRESH in-flight set and with rule parsing
    /// disabled. Both are needed, and for the same case: a shape whose rule names
    /// that shape itself as its condition (the W3C `square-triple` case is exactly
    /// this — `ex:Rectangle`'s rule is conditioned on `ex:Rectangle`). The
    /// enclosing shape is in flight at this point, so the ordinary cycle guard
    /// would hand back the EMPTY stand-in shape, and an empty shape conforms to
    /// everything — the condition would silently always hold, which is precisely
    /// the failure mode a condition exists to prevent. Clearing the in-flight set
    /// resolves the real shape; disabling rules is what keeps that finite, and
    /// costs nothing because `conforms` never reads a shape's rules.
    fn parse_conditions(
        &mut self,
        shape_id: &Term,
        rule_node: &Term,
        condition_nodes: Vec<Term>,
    ) -> Result<Vec<crate::shapes::Shape>, String> {
        if condition_nodes.is_empty() {
            return Ok(Vec::new());
        }
        let saved_in_flight = std::mem::take(&mut self.in_flight);
        let saved_rules = std::mem::replace(&mut self.parse_rules_enabled, false);

        let mut conditions: Vec<crate::shapes::Shape> = Vec::with_capacity(condition_nodes.len());
        let mut outcome = Ok(());
        for condition_node in condition_nodes {
            if !self.node_is_a_shape(&condition_node) {
                outcome = Err(format!(
                    "sh:condition {condition_node} on rule {rule_node} of shape {shape_id} does \
                     not resolve to a shape in the shapes graph; a rule must never fire on a \
                     condition that cannot be evaluated"
                ));
                break;
            }
            match self.parse_inline_shape(condition_node) {
                Ok(shape) => conditions.push(shape),
                Err(e) => {
                    outcome = Err(e);
                    break;
                }
            }
        }

        // Restore on EVERY path, including the error one: a parser left with a
        // cleared in-flight set would lose its cycle guard for the rest of the
        // document.
        self.in_flight = saved_in_flight;
        self.parse_rules_enabled = saved_rules;
        outcome.map(|()| conditions)
    }

    /// Whether `node` is authored as a SHAPE in the shapes graph.
    ///
    /// A shape is a node the shapes graph makes SHACL statements about: it is
    /// explicitly typed `sh:NodeShape` / `sh:PropertyShape`, or it is the subject
    /// of at least one triple whose predicate is a SHACL term (`sh:path`,
    /// `sh:property`, `sh:minCount`, `sh:not`, …). That covers the whole legal
    /// range a `sh:condition` may name — a top-level node shape, a top-level
    /// `sh:PropertyShape`, and an anonymous inline shape — while refusing a node
    /// the shapes graph never described as a shape at all.
    ///
    /// The distinction matters because [`Parser::parse_inline_shape`] answers an
    /// undescribed node with an EMPTY shape, and an empty shape conforms to
    /// everything: without this test, `sh:condition ex:NotAShape` would not fail,
    /// it would silently hold.
    fn node_is_a_shape(&self, node: &Term) -> bool {
        if self.has_type(node, sh::NODE_SHAPE) || self.has_type(node, sh::PROPERTY_SHAPE) {
            return true;
        }
        crate::data::native_quads(
            self.data,
            Some(node),
            None,
            None,
            crate::data::GraphFilter::AnyGraph,
        )
        .iter()
        .any(|(_, predicate, _)| predicate.as_str().starts_with(sh::NS))
    }

    /// Parse a `sh:TripleRule` head (`sh:subject`/`sh:predicate`/`sh:object` node
    /// expressions — all three required), together with its SRL §4.4 schedule.
    ///
    /// The head is a run-once rule exactly when a subject or object expression
    /// puts a blank node into the derived triple (see
    /// [`node_expr_mints_blank`]); the predicate cannot, since a triple predicate
    /// must be an IRI.
    fn parse_triple_rule(
        &mut self,
        shape_id: &Term,
        rule_node: &Term,
    ) -> Result<(RuleBody, RuleSchedule), String> {
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

        let schedule = if node_expr_mints_blank(&subject) || node_expr_mints_blank(&object) {
            RuleSchedule::Once
        } else {
            RuleSchedule::General
        };

        Ok((
            RuleBody::Triple {
                subject,
                predicate,
                object,
            },
            schedule,
        ))
    }

    /// Parse a `sh:SPARQLRule` head (a `sh:construct` CONSTRUCT query), together
    /// with its SRL §4.4 schedule. The query is validated (parseable +
    /// CONSTRUCT-form + pre-binding-legal) at load time; the `$this`-bearing
    /// prefix header is prepended.
    ///
    /// The schedule is read off the PARSED algebra: a blank node anywhere in the
    /// CONSTRUCT template is minted fresh per solution, making the rule a
    /// run-once rule (see [`construct_template_mints_blank`]).
    fn parse_sparql_rule(
        &self,
        shape_id: &Term,
        rule_node: &Term,
    ) -> Result<(RuleBody, RuleSchedule), String> {
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

        let schedule = match SparqlParser::new().parse_query(&construct) {
            Ok(query @ Query::Construct { .. }) => {
                // The query runs with $this pre-bound to each focus node; the
                // pre-binding restrictions (SHACL 1.2 SPARQL Extensions,
                // Appendix A) reject an illegal
                // body (MINUS/SERVICE/VALUES, `AS $this`, …) as a hard failure.
                crate::prebinding::check_construct(&query, &["this"])
                    .map_err(|e| format!("sh:SPARQLRule {rule_node} on shape {shape_id}: {e}"))?;
                if construct_template_mints_blank(&query) {
                    RuleSchedule::Once
                } else {
                    RuleSchedule::General
                }
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
        };

        Ok((RuleBody::Sparql { construct }, schedule))
    }
}

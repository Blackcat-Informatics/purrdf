// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Parsing SHACL 1.2 Inference Rules: shape rules (`sh:rule`), global rules, rule
//! sets, SPARQL rule templates and the rules entailment regime.
//!
//! # Rule types
//!
//! SHACL 1.2 Inference Rules, "Syntax of SHACL Rules": "Each SHACL rule has at least one
//! rdf:type which is an IRI." "Rule R has rule type T if R is a SHACL instance of T." A
//! rule's types decide how it executes, and this engine supports three: `sh:TripleRule`,
//! `sh:SPARQLRule`, and every SPARQL rule template the shapes graph declares ("The
//! instances of sh:SPARQLRuleTemplate (the SPARQL rule templates) are classes. The
//! instances of those classes are the actual rule instances."). "If a rules engine is not
//! able to execute a given rule because it does not support any of the rule types of the
//! rule, then it reports a failure" — so a rule node of no supported type, an untyped one
//! included, is a load error.
//!
//! # Rule nodes against the census
//!
//! Every `sh:` predicate of a rule node, of a rule set and of a SPARQL rule template is
//! looked up in the census ([`crate::spec::census`]): a term the census does not know, a
//! term it classifies as refused, and a term that belongs on some other kind of node
//! (a constraint parameter on a rule) are load errors naming the term and the node, never
//! silently walked past.

use purrdf_rdf::FastSet;

use crate::model::{rdf, sh, xsd};
use crate::rules::{OrderKey, Rule, RuleBody, RuleGraph, RuleSetDeclaration, check_construct};
use crate::shapes::Parser;
use crate::spec::census::{self, TermClass};
use crate::term::{NamedNode, Term};

use super::shacl_instance::ShaclInstances;

/// The `sh:` terms a RULE node may carry, besides the non-validating ones: the rule
/// vocabulary of SHACL 1.2 Inference Rules and the characteristics a rule shares with a
/// shape (`sh:deactivated`, `sh:order`, `sh:prefixes`).
const RULE_TERMS: [&str; 13] = [
    sh::CONSTRUCT,
    sh::PREFIXES,
    sh::SUBJECT,
    sh::PREDICATE,
    sh::OBJECT,
    sh::CONDITION,
    sh::ORDER,
    sh::LAYER,
    sh::RUN_ONCE,
    sh::DEACTIVATED,
    sh::EXPECTED_PREDICATE,
    sh::RULE_PROCESSOR,
    sh::RULE,
];

/// The `sh:` terms a RULE SET node may carry, besides the non-validating ones.
const RULE_SET_TERMS: [&str; 3] = [sh::HAS_RULE, sh::INCLUDES_RULE_SET, sh::RULE_PROCESSOR];

/// The `sh:` terms a SPARQL RULE TEMPLATE may carry, besides the non-validating ones:
/// "A SPARQL rule template can declare parameters with the same syntax as SPARQL-based
/// constraint components. Each SPARQL rule template has exactly one value of
/// sh:construct […] a SPARQL rule template can use the property sh:prefixes".
const TEMPLATE_TERMS: [&str; 3] = [sh::PARAMETER_PROPERTY, sh::CONSTRUCT, sh::PREFIXES];

/// The pre-bound variable names a template parameter may not take: `$this` and the
/// shape context a shape rule pre-binds.
use crate::sparql::THIS_AND_SHAPE_CONTEXT as RESERVED_VARIABLES;

/// The one rule type a rule node executes as.
enum RuleKind {
    /// `sh:TripleRule`.
    Triple,
    /// `sh:SPARQLRule`.
    Sparql,
    /// An instance of the SPARQL rule template of this IRI.
    Template(Term),
}

impl Parser<'_> {
    /// Parse every `sh:rule` attached to shape `id` into a [`Rule`], in stable
    /// (rule-node string) order.
    ///
    /// # Errors
    ///
    /// Hard-fails on a malformed rule — see [`Self::parse_rule`].
    pub(crate) fn parse_rules(&mut self, id: &Term) -> Result<Vec<Rule>, String> {
        let mut rule_nodes: Vec<Term> = self.objects_of(id, sh::RULE);
        crate::term::sort_terms_canonical(&mut rule_nodes);
        let mut rules: Vec<Rule> = Vec::with_capacity(rule_nodes.len());
        for rule_node in rule_nodes {
            rules.push(self.parse_rule(Some(id), &rule_node)?);
        }
        Ok(rules)
    }

    /// Parse the shapes graph's global rules, rule sets and entailment declaration.
    ///
    /// SHACL 1.2 Inference Rules: "A global rule is a rule that is not linked to a shape
    /// by a sh:rule predicate." The global rules are the SHACL instances of a supported
    /// rule type, the instances of a SPARQL rule template, and the members of a rule set
    /// (`sh:hasRule`: "those values are rules"), that no `sh:rule` triple names.
    ///
    /// # Errors
    ///
    /// A malformed global rule, rule set or template; an `sh:entailment` value that is neither
    /// `sh:RulesEntailment` nor a declared rule set.
    pub(crate) fn parse_rule_graph(&mut self) -> Result<RuleGraph, String> {
        let (entailment, entailment_rule_sets) = self.parse_entailment()?;
        let linked: FastSet<Term> = self
            .quads_with(None, Some(sh::RULE), None)
            .into_iter()
            .map(|(_, _, object)| object)
            .collect();
        let mut candidates: FastSet<Term> = FastSet::default();
        for node in self.rule_typed_nodes() {
            candidates.insert(node);
        }
        for (_, _, member) in self.quads_with(None, Some(sh::HAS_RULE), None) {
            candidates.insert(member);
        }
        let mut globals: Vec<Term> = candidates
            .into_iter()
            .filter(|node| !linked.contains(node))
            .collect();
        crate::term::sort_terms_canonical(&mut globals);
        let mut global_rules = Vec::with_capacity(globals.len());
        for node in globals {
            global_rules.push(self.parse_rule(None, &node)?);
        }
        let rule_sets = self.parse_rule_sets()?;
        for template in self.rule_templates() {
            self.check_census(&template, "SPARQL rule template", &TEMPLATE_TERMS)?;
        }
        Ok(RuleGraph {
            global_rules,
            rule_sets,
            entailment,
            entailment_rule_sets,
        })
    }

    /// The entailment regimes the shapes graph declares: whether it names
    /// `sh:RulesEntailment`, and the rule sets it names, in IRI order.
    ///
    /// SHACL 1.2 Core: "If a shapes graph contains any triple with the predicate
    /// sh:entailment and the object E and the SHACL processor does not support E as an
    /// entailment regime for the given data graph then the processor MUST signal a
    /// failure." This processor supports `sh:RulesEntailment` and, under the law
    /// that reads the editor's draft's "Entailment using Rule Sets" ([`ShaclProfile::ED_20261016`](crate::profile::ShaclProfile::ED_20261016),
    /// SHACL 1.2 Inference Rules: "Any rule set can be used as a value of
    /// sh:entailment, i.e. any IRI that is a SHACL instance of sh:RuleSet in the shapes
    /// graph."), the rule sets of the shapes graph. The working draft states only
    /// `sh:RulesEntailment`, so under it a rule set is a regime this processor does
    /// not support, like every other value.
    fn parse_entailment(&self) -> Result<(bool, Vec<NamedNode>), String> {
        let mut rules = false;
        let mut sets: Vec<NamedNode> = Vec::new();
        let mut instances = ShaclInstances::new(self.data);
        for (subject, _, object) in self.quads_with(None, Some(sh::ENTAILMENT), None) {
            if object == Term::NamedNode(NamedNode::from(sh::RULES_ENTAILMENT)) {
                rules = true;
                continue;
            }
            let is_rule_set =
                matches!(&object, Term::NamedNode(_)) && self.is_rule_set(&mut instances, &object);
            if is_rule_set && !self.profile.reads_editors_draft_semantics() {
                return Err(format!(
                    "the shapes graph declares {subject} sh:entailment {object}, a rule set; \
                     under {} the only entailment regime this processor supports from \
                     SHACL 1.2 Inference Rules is sh:RulesEntailment (\"the IRI \
                     sh:RulesEntailment, which executes all rules of the shapes graph\"), and \
                     SHACL requires a processor to signal a failure for a regime it does not \
                     support; rule sets as sh:entailment values are first defined by the \
                     editor's draft {}",
                    self.profile,
                    crate::profile::ShaclProfile::ED_20261016
                ));
            }
            if let Term::NamedNode(iri) = &object
                && is_rule_set
            {
                if !sets.contains(iri) {
                    sets.push(iri.clone());
                }
                continue;
            }
            return Err(format!(
                "the shapes graph declares {subject} sh:entailment {object}; this processor \
                 supports the entailment regime sh:RulesEntailment and the rule sets of the \
                 shapes graph (SHACL instances of sh:RuleSet), and SHACL requires a processor \
                 to signal a failure for a regime it does not support"
            ));
        }
        sets.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        Ok((rules, sets))
    }

    /// Whether `term` is a SHACL instance of `sh:RuleSet` in the shapes graph: the one
    /// test by which a rule set is recognised, for the declarations
    /// ([`Self::parse_rule_sets`]) and for the values of `sh:entailment`
    /// ([`Self::parse_entailment`]) alike. An untyped subject of `sh:hasRule` or
    /// `sh:includesRuleSet` is not one, since SHACL does no RDFS inference from their
    /// `rdfs:domain`.
    fn is_rule_set(&self, instances: &mut ShaclInstances<'_>, term: &Term) -> bool {
        crate::data::resolve_id(self.data, term)
            .is_some_and(|node| instances.is_instance(node, self.data.term_id_by_iri(sh::RULE_SET)))
    }

    /// Every node that is a SHACL instance of `sh:Rule`, `sh:TripleRule` or
    /// `sh:SPARQLRule`, or an instance of a SPARQL rule template, in canonical order.
    fn rule_typed_nodes(&self) -> Vec<Term> {
        let templates = self.rule_templates();
        let mut instances = ShaclInstances::new(self.data);
        let classes: Vec<Option<purrdf_rdf::TermId>> =
            [sh::RULE_CLASS, sh::TRIPLE_RULE, sh::SPARQL_RULE]
                .iter()
                .map(|iri| self.data.term_id_by_iri(iri))
                .collect();
        let mut out: Vec<Term> = Vec::new();
        for (subject, _, class) in self.quads_with(None, Some(rdf::TYPE), None) {
            let is_rule = templates.contains(&class)
                || crate::data::resolve_id(self.data, &subject).is_some_and(|node| {
                    classes
                        .iter()
                        .any(|class| instances.is_instance(node, *class))
                });
            if is_rule && !out.contains(&subject) {
                out.push(subject);
            }
        }
        crate::term::sort_terms_canonical(&mut out);
        out
    }

    /// Every SPARQL rule template: the SHACL instances of `sh:SPARQLRuleTemplate`.
    fn rule_templates(&self) -> Vec<Term> {
        let mut instances = ShaclInstances::new(self.data);
        let class = self.data.term_id_by_iri(sh::SPARQL_RULE_TEMPLATE);
        let mut out: Vec<Term> = Vec::new();
        for (subject, _, _) in self.quads_with(None, Some(rdf::TYPE), None) {
            if !out.contains(&subject)
                && crate::data::resolve_id(self.data, &subject)
                    .is_some_and(|node| instances.is_instance(node, class))
            {
                out.push(subject);
            }
        }
        crate::term::sort_terms_canonical(&mut out);
        out
    }

    /// Parse every rule set, in IRI order, and resolve each one's include closure.
    ///
    /// SHACL 1.2 Inference Rules: "All SHACL instances of sh:RuleSet have an IRI. Rule sets
    /// can have values for sh:hasRule and those values are rules. The values of
    /// sh:includesRuleSet at a rule set are IRIs." A rule set is a SHACL instance of
    /// `sh:RuleSet` and nothing else: `sh:hasRule` and `sh:includesRuleSet` declare
    /// `rdfs:domain sh:RuleSet`, but SHACL does no RDFS inference, so an untyped subject
    /// of either is not a rule set (the members of its `sh:hasRule` are still global
    /// rules). [`Self::parse_entailment`] reads "rule set" by the same test.
    /// `sh:includesRuleSet` has `rdfs:range sh:RuleSet`, so an included IRI that is not a
    /// declared rule set is refused here, at load, whichever regimes the shapes graph
    /// names; include cycles are allowed, the closure being a set.
    fn parse_rule_sets(&self) -> Result<Vec<RuleSetDeclaration>, String> {
        let mut nodes: Vec<Term> = Vec::new();
        let mut instances = ShaclInstances::new(self.data);
        for (subject, _, _) in self.quads_with(None, Some(rdf::TYPE), None) {
            if self.is_rule_set(&mut instances, &subject) {
                nodes.push(subject);
            }
        }
        crate::term::sort_terms_canonical(&mut nodes);
        nodes.dedup();
        let mut sets = Vec::with_capacity(nodes.len());
        for node in nodes {
            self.check_census(&node, "rule set", &RULE_SET_TERMS)?;
            let Term::NamedNode(id) = node.clone() else {
                return Err(format!(
                    "rule set {node} is not an IRI; SHACL 1.2 Inference Rules: \"All SHACL \
                     instances of sh:RuleSet have an IRI\""
                ));
            };
            let mut rules = self.objects_of(&node, sh::HAS_RULE);
            crate::term::sort_terms_canonical(&mut rules);
            let mut includes: Vec<NamedNode> = Vec::new();
            for value in self.objects_of(&node, sh::INCLUDES_RULE_SET) {
                let Term::NamedNode(iri) = value else {
                    return Err(format!(
                        "sh:includesRuleSet on rule set {node} must be an IRI, got {value}"
                    ));
                };
                includes.push(iri);
            }
            includes.sort_by(|a, b| a.as_str().cmp(b.as_str()));
            sets.push(RuleSetDeclaration {
                id,
                rules,
                includes,
                processors: self.processors_of(&node)?,
            });
        }
        for set in &sets {
            for include in &set.includes {
                if !sets.iter().any(|known| known.id == *include) {
                    return Err(format!(
                        "rule set {} includes {} through sh:includesRuleSet, which is not a \
                         declared rule set (no SHACL instance of sh:RuleSet of that IRI); \
                         declare it with `{} a sh:RuleSet`",
                        Term::NamedNode(set.id.clone()),
                        Term::NamedNode(include.clone()),
                        Term::NamedNode(include.clone())
                    ));
                }
            }
        }
        Ok(sets)
    }

    /// The `sh:ruleProcessor` values of `node`: "The values of sh:ruleProcessor are
    /// IRIs, or literals with datatype xsd:string."
    fn processors_of(&self, node: &Term) -> Result<Vec<Term>, String> {
        let mut out = self.objects_of(node, sh::RULE_PROCESSOR);
        for value in &out {
            let ok = match value {
                Term::NamedNode(_) => true,
                Term::Literal(lit) => lit.datatype_str() == xsd::STRING && lit.language().is_none(),
                Term::BlankNode(_) | Term::Triple(_) => false,
            };
            if !ok {
                return Err(format!(
                    "sh:ruleProcessor on {node} must be an IRI or an xsd:string literal, got \
                     {value}"
                ));
            }
        }
        crate::term::sort_terms_canonical(&mut out);
        Ok(out)
    }

    /// Check every `sh:` predicate of `node` against the census: it must be known,
    /// implemented, and one of `allowed` or a non-validating term.
    fn check_census(&self, node: &Term, kind: &str, allowed: &[&str]) -> Result<(), String> {
        for (_, predicate, _) in self.quads_with_subject(node) {
            let p = predicate.as_str();
            if !census::is_census_namespace(p) {
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
                return Err(self.refuse_shacl_js(
                    node,
                    p,
                    format!(
                        "{kind} {node} uses <{p}>, which is not evaluated by this engine: {why}"
                    ),
                ));
            }
            if !allowed.contains(&p) && row.class != TermClass::NonValidating {
                return Err(format!(
                    "{kind} {node} carries <{p}>, which is not a property of a {kind}{}; it is \
                     refused rather than silently ignored",
                    census::no_processing_note(p)
                ));
            }
        }
        Ok(())
    }

    /// Every triple with `node` as subject.
    fn quads_with_subject(&self, node: &Term) -> Vec<(Term, NamedNode, Term)> {
        crate::data::native_quads(
            self.data,
            Some(node),
            None,
            None,
            crate::data::GraphFilter::AnyGraph,
        )
    }

    /// The rule type of `rule_node`.
    fn rule_kind(&self, rule_node: &Term) -> Result<RuleKind, String> {
        let templates = self.rule_templates();
        let mut instances = ShaclInstances::new(self.data);
        let node = crate::data::resolve_id(self.data, rule_node);
        let is = |instances: &mut ShaclInstances<'_>, iri: &str| {
            node.is_some_and(|node| instances.is_instance(node, self.data.term_id_by_iri(iri)))
        };
        let mut kinds: Vec<RuleKind> = Vec::new();
        if is(&mut instances, sh::TRIPLE_RULE) {
            kinds.push(RuleKind::Triple);
        }
        if is(&mut instances, sh::SPARQL_RULE) {
            kinds.push(RuleKind::Sparql);
        }
        for class in self.objects_of(rule_node, rdf::TYPE) {
            if templates.contains(&class) {
                kinds.push(RuleKind::Template(class));
            }
        }
        match kinds.len() {
            1 => Ok(kinds.pop().expect("one kind")),
            // A `sh:JSRule` a shape names is a rule this engine cannot execute
            // because it has no JavaScript engine: the same failure SHACL requires, typed
            // and naming the extension. One nothing names is inert vocabulary.
            0 if is(&mut instances, sh::JS_RULE) => Err(self.refuse_shacl_js(
                rule_node,
                sh::JS_RULE,
                format!(
                    "rule {rule_node} is a sh:JSRule: {}; SHACL 1.2 Inference Rules: \"If a \
                     rules engine is not able to execute a given rule because it does not \
                     support any of the rule types of the rule, then it reports a failure\"",
                    census::JS
                ),
            )),
            0 => Err(format!(
                "rule {rule_node} is not a recognised SHACL rule: it is an instance of none of \
                 the rule types this engine executes (sh:TripleRule, sh:SPARQLRule, or a \
                 sh:SPARQLRuleTemplate of the shapes graph); SHACL 1.2 Inference Rules: \"If a \
                 rules engine is not able to execute a given rule because it does not support \
                 any of the rule types of the rule, then it reports a failure\""
            )),
            _ => Err(format!(
                "rule {rule_node} is ambiguous: it is an instance of several rule types, and \
                 their instructions (sh:subject/predicate/object, sh:construct, a template's \
                 query) would each derive something different"
            )),
        }
    }

    /// Parse one rule node — a shape rule of `shape`, or a global rule when `shape` is
    /// `None`.
    ///
    /// # Errors
    ///
    /// Hard-fails on a malformed rule: a term the census refuses on a rule node, a rule of
    /// no supported type or of several, a malformed triple rule, SPARQL rule or template
    /// instance, or an ill-typed or repeated `sh:layer`, `sh:order`, `sh:runOnce` or
    /// `sh:deactivated`.
    fn parse_rule(&mut self, shape: Option<&Term>, rule_node: &Term) -> Result<Rule, String> {
        let kind = self.rule_kind(rule_node)?;
        let declared_parameters = match &kind {
            RuleKind::Template(template) => self.template_parameters(template)?,
            RuleKind::Triple | RuleKind::Sparql => Vec::new(),
        };
        let mut allowed: Vec<&str> = RULE_TERMS.to_vec();
        allowed.extend(declared_parameters.iter().map(|(path, _, _)| path.as_str()));
        self.check_census(rule_node, "rule", &allowed)?;
        if self.first_object_of(rule_node, sh::RULE).is_some() {
            return Err(format!(
                "rule {rule_node} carries sh:rule; sh:rule links a SHAPE to a rule"
            ));
        }
        self.at_most_one(
            rule_node,
            &[sh::DEACTIVATED, sh::LAYER, sh::ORDER, sh::RUN_ONCE],
        )?;
        let deactivated = self.deactivated_of(rule_node)?;
        let layer = self.numeric_of(rule_node, sh::LAYER)?;
        let order = self.numeric_of(rule_node, sh::ORDER)?;
        let run_once = match self.first_object_of(rule_node, sh::RUN_ONCE) {
            None => false,
            Some(value) => crate::shapes::parser_boolean(&value).ok_or_else(|| {
                format!(
                    "sh:runOnce on rule {rule_node} must be an xsd:boolean literal, got {value} \
                     (SHACL 1.2 Inference Rules: \"The values of sh:runOnce at rules are \
                     literals with datatype xsd:boolean\")"
                )
            })?,
        };
        let mut condition_nodes: Vec<Term> = self.objects_of(rule_node, sh::CONDITION);
        crate::term::sort_terms_canonical(&mut condition_nodes);
        let owner = shape.cloned().unwrap_or_else(|| rule_node.clone());
        let conditions = self.parse_conditions(&owner, rule_node, condition_nodes)?;
        let prebound_this = shape.is_some();

        let body = match kind {
            RuleKind::Triple => self.parse_triple_rule(rule_node)?,
            RuleKind::Sparql => {
                let owners: Vec<&Term> = shape
                    .into_iter()
                    .chain(std::iter::once(rule_node))
                    .collect();
                let construct = format!(
                    "{}{}",
                    self.prefix_header(&owners)?,
                    self.construct_of(rule_node)?
                );
                check_construct(
                    self,
                    rule_node,
                    &construct,
                    if prebound_this { &["this"] } else { &[] },
                    prebound_this,
                )?;
                RuleBody::Sparql {
                    construct,
                    parameters: Vec::new(),
                }
            }
            RuleKind::Template(template) => self.parse_template_instance(
                rule_node,
                &template,
                &declared_parameters,
                prebound_this,
            )?,
        };
        let expected_predicates = self.expected_predicates_of(rule_node)?;
        let processors = self.processors_of(rule_node)?;
        if let RuleBody::Sparql {
            construct,
            parameters,
        } = &body
        {
            let occurrence = crate::shapes::RuleOccurrence {
                purpose: if prebound_this {
                    crate::profile::QueryPurpose::ConstructRule
                } else {
                    crate::profile::QueryPurpose::GlobalConstructRule
                },
                parameters: declared_parameters
                    .into_iter()
                    .map(|(_, variable, _)| variable)
                    .collect(),
                construct: construct.clone(),
                substitutions: parameters.clone(),
            };
            let mut sources = self.sparql_sources.borrow_mut();
            let uses = sources.rules.entry(rule_node.clone()).or_default();
            if let Some(shape) = shape {
                uses.linked.insert(shape.clone(), occurrence);
            } else {
                uses.global = Some(occurrence);
            }
        }
        Ok(Rule {
            id: rule_node.clone(),
            body,
            conditions,
            layer,
            order,
            run_once,
            deactivated,
            expected_predicates,
            processors,
        })
    }

    /// Refuse a second value of any of `predicates` on `node`: "Each rule may have at most
    /// one value for the property" `sh:deactivated`, `sh:layer`, `sh:order`, `sh:runOnce`.
    fn at_most_one(&self, node: &Term, predicates: &[&str]) -> Result<(), String> {
        for predicate in predicates {
            if self.objects_of(node, predicate).len() > 1 {
                return Err(format!(
                    "rule {node} has more than one value for <{predicate}>; SHACL 1.2 Inference \
                     Rules: \"Each rule may have at most one value\" for it"
                ));
            }
        }
        Ok(())
    }

    /// A rule's `sh:layer` or `sh:order`: "literals with datatype xsd:integer or
    /// xsd:decimal". A finite value only: the value partitions execution, and `NaN` or an
    /// infinity has no position.
    fn numeric_of(&self, node: &Term, predicate: &str) -> Result<Option<OrderKey>, String> {
        let Some(value) = self.first_object_of(node, predicate) else {
            return Ok(None);
        };
        let parsed = match &value {
            Term::Literal(lit)
                if lit.datatype_str() == xsd::INTEGER || lit.datatype_str() == xsd::DECIMAL =>
            {
                lit.value().parse::<f64>().ok().filter(|v| v.is_finite())
            }
            _ => None,
        };
        parsed.map(|v| Some(OrderKey::new(v))).ok_or_else(|| {
            format!(
                "<{predicate}> on rule {node} must be an xsd:integer or xsd:decimal literal, got \
                 {value}"
            )
        })
    }

    /// The one `sh:construct` string of `node`: "exactly one value for the property
    /// sh:construct. The values of sh:construct are literals with datatype xsd:string."
    fn construct_of(&self, node: &Term) -> Result<String, String> {
        let values = self.objects_of(node, sh::CONSTRUCT);
        match values.as_slice() {
            [Term::Literal(lit)] if lit.datatype_str() == xsd::STRING => Ok(lit.value().to_owned()),
            [] => Err(format!("{node} is missing its sh:construct query")),
            [other] => Err(format!(
                "sh:construct on {node} must be an xsd:string literal, got {other}"
            )),
            _ => Err(format!("{node} has more than one sh:construct query")),
        }
    }

    /// The rule's `sh:expectedPredicate` values, sorted and deduplicated.
    ///
    /// SHACL 1.2 Inference Rules, "Expected Derived Triples": "The expected derived
    /// triples of a rule are the derived triples for all values of the property
    /// sh:expectedPredicate at the rule." A derived triple's predicate is that value, so a
    /// value that is not an IRI names no derived triple and is refused.
    fn expected_predicates_of(&self, rule_node: &Term) -> Result<Vec<NamedNode>, String> {
        let mut predicates: Vec<NamedNode> = Vec::new();
        for value in self.objects_of(rule_node, sh::EXPECTED_PREDICATE) {
            let Term::NamedNode(predicate) = value else {
                return Err(format!(
                    "sh:expectedPredicate on rule {rule_node} must be an IRI (a predicate), got \
                     {value}"
                ));
            };
            predicates.push(predicate);
        }
        predicates.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        predicates.dedup();
        Ok(predicates)
    }

    /// Resolve every `sh:condition` node of a rule into parsed shapes.
    ///
    /// SHACL 1.2 Inference Rules: "The values of sh:condition at a rule must be well-formed
    /// shapes. If the value C of sh:condition is a SHACL instance of both sh:NodeShape and
    /// rdfs:Class, then the focus nodes must also conform to the constraints of the
    /// non-deactivated SHACL superclasses of C that are also SHACL instances of both
    /// sh:NodeShape and rdfs:Class." Those superclasses join the list. A superclass
    /// deactivated by a node expression joins it too: the expression is evaluated at the
    /// focus node the condition judges, where a deactivated shape is conformed to.
    ///
    /// The sub-parse runs with a FRESH in-flight set and with rule parsing disabled: a
    /// shape whose rule names that shape itself as its condition is legal, and the
    /// ordinary cycle guard would hand back the EMPTY stand-in shape, which conforms to
    /// everything — the condition would silently always hold.
    fn parse_conditions(
        &mut self,
        owner: &Term,
        rule_node: &Term,
        condition_nodes: Vec<Term>,
    ) -> Result<Vec<crate::shapes::Shape>, String> {
        if condition_nodes.is_empty() {
            return Ok(Vec::new());
        }
        let mut nodes: Vec<Term> = Vec::new();
        for node in condition_nodes {
            if !self.node_is_a_shape(&node) {
                return Err(format!(
                    "sh:condition {node} on rule {rule_node} of {owner} does not resolve to a \
                     shape in the shapes graph; a rule must never fire on a condition that \
                     cannot be evaluated"
                ));
            }
            self.class_shape_closure(&node, &mut nodes)?;
        }
        let mut conditions: Vec<crate::shapes::Shape> = Vec::with_capacity(nodes.len());
        let mut outcome = Ok(());
        for node in nodes {
            match self.parse_inline_shape(node) {
                Ok(shape) => conditions.push(shape),
                Err(e) => {
                    outcome = Err(e);
                    break;
                }
            }
        }
        outcome.map(|()| conditions)
    }

    /// Record `node`, and — when it is a SHACL instance of both `sh:NodeShape` and
    /// `rdfs:Class` — its non-deactivated SHACL superclasses that are too, each once.
    fn class_shape_closure(&self, node: &Term, out: &mut Vec<Term>) -> Result<(), String> {
        let mut instances = ShaclInstances::new(self.data);
        let class_shape = |instances: &mut ShaclInstances<'_>, term: &Term| {
            crate::data::resolve_id(self.data, term)
                .is_some_and(|id| instances.is_node_shape(id) && instances.is_class(id))
        };
        if !out.contains(node) {
            out.push(node.clone());
        }
        if !class_shape(&mut instances, node) {
            return Ok(());
        }
        let mut pending = vec![node.clone()];
        let mut seen = vec![node.clone()];
        while let Some(class) = pending.pop() {
            for superclass in self.objects_of(&class, crate::model::rdfs::SUB_CLASS_OF) {
                if seen.contains(&superclass) {
                    continue;
                }
                seen.push(superclass.clone());
                pending.push(superclass.clone());
                if class_shape(&mut instances, &superclass)
                    && !self.constantly_deactivated(&superclass)?
                    && !out.contains(&superclass)
                {
                    out.push(superclass);
                }
            }
        }
        Ok(())
    }

    /// Whether `node` belongs to the shapes graph's authoritative shape census.
    /// A node-expression key alone does not declare a shape. The distinction
    /// matters because an undescribed node parses as an EMPTY shape, which conforms to
    /// everything.
    pub(super) fn node_is_a_shape(&self, node: &Term) -> bool {
        // Expression keys such as sh:if and sh:path are not a shape declaration.
        // Use the same complete membership rule as source admission, including
        // subclass instances and otherwise-empty shape-valued operands.
        self.shape_census
            .get_or_init(|| self.spec_shapes().into_iter().collect())
            .contains(node)
    }

    /// Parse a `sh:TripleRule`: "Each triple rule must have at most one value of the
    /// property sh:subject (which must be a well-formed node expression)", and likewise
    /// `sh:predicate` and `sh:object`. An absent one is the focus node at execution.
    fn parse_triple_rule(&mut self, rule_node: &Term) -> Result<RuleBody, String> {
        let mut parts: [Option<crate::expression::NodeExpr>; 3] = [None, None, None];
        for (slot, predicate) in [sh::SUBJECT, sh::PREDICATE, sh::OBJECT].iter().enumerate() {
            let values = self.objects_of(rule_node, predicate);
            if values.len() > 1 {
                return Err(format!(
                    "sh:TripleRule {rule_node} has more than one value for <{predicate}>; SHACL \
                     1.2 Inference Rules: \"Each triple rule must have at most one value\" of it"
                ));
            }
            if let Some(node) = values.into_iter().next() {
                parts[slot] = Some(self.parse_node_expr(&node)?);
            }
        }
        let [subject, predicate, object] = parts;
        Ok(RuleBody::Triple {
            subject,
            predicate,
            object,
        })
    }

    /// The parameters of SPARQL rule template `template`: `(path IRI, variable name,
    /// optional?)` in path order.
    fn template_parameters(&self, template: &Term) -> Result<Vec<(String, String, bool)>, String> {
        let mut out: Vec<(String, String, bool)> = Vec::new();
        for declaration in self.objects_of(template, sh::PARAMETER_PROPERTY) {
            let Some(Term::NamedNode(path)) = self.first_object_of(&declaration, sh::PATH) else {
                return Err(format!(
                    "parameter {declaration} of SPARQL rule template {template} must have an IRI \
                     sh:path"
                ));
            };
            let optional = match self.first_object_of(&declaration, sh::OPTIONAL) {
                None => false,
                Some(value) => crate::shapes::parser_boolean(&value).ok_or_else(|| {
                    format!("sh:optional on parameter {declaration} must be an xsd:boolean")
                })?,
            };
            let variable = purrdf_iri::local_name(path.as_str()).to_owned();
            if RESERVED_VARIABLES.contains(&variable.as_str()) {
                return Err(format!(
                    "parameter {declaration} of SPARQL rule template {template} names the \
                     variable ${variable}, which a SHACL rule pre-binds itself"
                ));
            }
            if out.iter().any(|(_, known, _)| *known == variable) {
                return Err(format!(
                    "SPARQL rule template {template} declares two parameters whose variable is \
                     ${variable}"
                ));
            }
            out.push((path.as_str().to_owned(), variable, optional));
        }
        out.sort();
        Ok(out)
    }

    /// Parse an instance of SPARQL rule template `template`.
    ///
    /// SHACL 1.2 Inference Rules, "Execution of rules based on SPARQL rule templates":
    /// "Let Q be the SPARQL CONSTRUCT query that is produced from the value of sh:construct
    /// at T in the rules graph, using the prefix declarations at T. Using a pre-binding map
    /// for each declared sh:parameter of T at R similar to SPARQL-based constraint
    /// components, evaluate Q like a corresponding SPARQL Rule but using the extra
    /// pre-bound variables. Report a failure if any of the declared parameters that are not
    /// declared as sh:optional true have no value in R." An instance with several values
    /// for one parameter has no single pre-binding map, and is refused.
    fn parse_template_instance(
        &self,
        rule_node: &Term,
        template: &Term,
        declared_parameters: &[(String, String, bool)],
        prebound_this: bool,
    ) -> Result<RuleBody, String> {
        if !matches!(template, Term::NamedNode(_)) {
            return Err(format!(
                "SPARQL rule template {template} is not an IRI; SHACL 1.2 Inference Rules: \
                 \"Each SPARQL rule template is an IRI.\""
            ));
        }
        let construct = format!(
            "{}{}",
            self.prefix_header(&[template])?,
            self.construct_of(template)?
        );
        let mut parameters: Vec<(String, Term)> = Vec::new();
        for (path, variable, optional) in declared_parameters {
            let values = self.objects_of(rule_node, path);
            match values.as_slice() {
                [] if *optional => {}
                [] => {
                    return Err(format!(
                        "rule {rule_node} is an instance of SPARQL rule template {template} but \
                         has no value for its non-optional parameter <{path}>; SHACL 1.2 \
                         Inference Rules: \"Report a failure if any of the declared parameters \
                         that are not declared as sh:optional true have no value in R\""
                    ));
                }
                [value] => parameters.push((variable.clone(), value.clone())),
                _ => {
                    return Err(format!(
                        "rule {rule_node} has {} values for the parameter <{path}> of SPARQL \
                         rule template {template}; a template instance pre-binds one value per \
                         parameter",
                        values.len()
                    ));
                }
            }
        }
        let mut prebound: Vec<&str> = Vec::new();
        if prebound_this {
            prebound.push("this");
        }
        if self.profile == crate::profile::ShaclProfile::LEGACY {
            prebound.extend(parameters.iter().map(|(name, _)| name.as_str()));
        } else {
            prebound.extend(
                declared_parameters
                    .iter()
                    .map(|(_, variable, _)| variable.as_str()),
            );
        }
        check_construct(self, rule_node, &construct, &prebound, prebound_this)?;
        Ok(RuleBody::Sparql {
            construct,
            parameters,
        })
    }
}

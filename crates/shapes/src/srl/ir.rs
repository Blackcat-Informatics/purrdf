// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The rule-set IR: the one representation every rule frontend lowers to.
//!
//! Two rule languages reach PurRDF's rules engine, and both arrive here:
//!
//! * **SPARQL 1.2 RL** rule sets are this IR's native shape. Its abstract syntax is
//!   transcribed element for element — "A rule is a pair of a rule head (often just
//!   "head") and a rule body (often just "body")", a body "is a sequence of rule
//!   elements; that is, each sequence element is one of a triple pattern element, a
//!   filter element, a negation element, or an assignment element", and "A rule set is
//!   a collection of zero or more rules, a collection of zero or more data blocks, and a
//!   collection of zero or more rule set imports" — so a parsed SRL document lowers
//!   into it field for field. Imports are resolved before a rule set is built, which is
//!   what "A resolved rule set is a rule set which has no imports" makes of them.
//! * **SHACL rules** (`sh:TripleRule`, `sh:SPARQLRule`, instances of a
//!   `sh:SPARQLRuleTemplate`) are opaque to rule-level analysis — a node expression or
//!   a SPARQL CONSTRUCT query is not a sequence of rule elements — so each is carried as
//!   a [`IrRuleBody::Shacl`] PRODUCER, evaluated by the SHACL machinery against the
//!   evaluation graph, and scheduled by its declared `sh:layer`, `sh:order` and
//!   `sh:runOnce`.
//!
//! Both kinds lower onto `purrdf-datalog` ([`super::lower`]): a triple pattern becomes a
//! clause atom, a filter or an assignment a guard literal, a negation element a negated
//! conjunction, and a SHACL producer a clause whose body is one model-reading guard. The
//! rule set is then evaluated by that crate's ordered schedule — under the declared
//! schedule for SHACL rules, under the rule-level stratification for SPARQL 1.2 RL —
//! and that is the only rule evaluator PurRDF has.
//!
//! # Aggregation
//!
//! Neither language defines an aggregation element: SPARQL 1.2 RL's rule elements are
//! the four above, and SHACL's aggregating node expressions (`shnex:count`, `shnex:sum`,
//! …) are evaluated inside a producer against the graph the rule runs over. So the IR
//! has no aggregate literal and the stratifier needs no aggregation stratum.

use purrdf_sparql_algebra::Expression;

use crate::rules::{OrderKey, Rule as ShaclRule};
use crate::shapes::Shape;
use crate::term::{NamedNode, Term};

/// A rule set: rules, and the triples of its data blocks.
#[derive(Debug, Clone)]
pub struct RuleSet<'a> {
    /// The rules, in authored order.
    pub rules: Vec<IrRule<'a>>,
    /// The triples of every data block. SPARQL 1.2 RL: "A data block is a set of
    /// triples. These triples are added to the inference graph as additional facts and
    /// are included in the inference process."
    pub data: Vec<[Term; 3]>,
    /// How the rule set is scheduled.
    pub scheduling: Scheduling,
}

/// How a rule set's execution order is decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheduling {
    /// By each rule's declared layer, order and run-once flag — SHACL 1.2 Inference
    /// Rules, "General Execution Instructions for SHACL Rules". Rules with the same
    /// layer and order run concurrently.
    Declared,
    /// By SPARQL 1.2 RL's rule-level stratification: "A stratification of a rule set is
    /// a sequence of stratification layers", computed from the rules' dependencies, with
    /// run-once rules decided by [`IrRule::is_run_once`].
    Stratified,
}

/// One rule.
#[derive(Debug, Clone)]
pub struct IrRule<'a> {
    /// The rule's identity. SPARQL 1.2 RL: "An identifier for the rule, which is a
    /// blank node or iri"; a SHACL rule's node.
    pub id: Term,
    /// What the rule derives, and from what.
    pub body: IrRuleBody<'a>,
    /// The rule's declared place in a [`Scheduling::Declared`] schedule. Ignored under
    /// [`Scheduling::Stratified`], which computes the place.
    pub schedule: DeclaredSchedule,
    /// The predicates whose expected derived triples the rule expects to be present
    /// while its layer runs (SHACL 1.2 Inference Rules, "Expected Derived Triples").
    pub expected_predicates: Vec<NamedNode>,
}

impl IrRule<'_> {
    /// Whether the rule is a RUN-ONCE rule.
    ///
    /// For an element rule, SPARQL 1.2 RL's definition: "Rules involving assignments and
    /// rules that create blank nodes in their rule head are run-once rules." For a SHACL
    /// rule, SHACL 1.2 Inference Rules': "A rule that has true as its value for
    /// sh:runOnce is a run-once rule" — and only that, since "Custom rule processors may
    /// compute additional run-once rules when sh:runOnce is absent" and the standard
    /// processor is not a custom one.
    #[must_use]
    pub fn is_run_once(&self) -> bool {
        match &self.body {
            IrRuleBody::Elements(rule) => {
                rule.body
                    .iter()
                    .any(|element| matches!(element, Element::Assign { .. }))
                    || rule.head.iter().any(TriplePattern::has_blank_node)
            }
            IrRuleBody::Shacl(_) => self.schedule.run_once,
        }
    }
}

/// A rule's declared scheduling properties (SHACL 1.2 Inference Rules).
#[derive(Debug, Clone, Copy)]
pub struct DeclaredSchedule {
    /// `sh:layer`: "Layers with a smaller numeric value will be executed before those
    /// with a larger number. […] If unspecified […] the default layer of a rule is 0."
    pub layer: OrderKey,
    /// `sh:order`: "within the same layer, rules with larger order values will be
    /// executed after those with smaller values. […] Rules with the same order are
    /// executed concurrently and must not see each other's inferences before they have
    /// all completed." Default 0.
    pub order: OrderKey,
    /// `sh:runOnce`: "Rules may use the property sh:runOnce to instruct a rules engine
    /// that the rule is only executed once and before the other rules in the same
    /// layer."
    pub run_once: bool,
}

impl Default for DeclaredSchedule {
    fn default() -> Self {
        Self {
            layer: OrderKey::new(0.0),
            order: OrderKey::new(0.0),
            run_once: false,
        }
    }
}

/// What a rule derives, and from what.
#[derive(Debug, Clone)]
#[allow(
    clippy::large_enum_variant,
    reason = "one rule per variant instance, built once per evaluation; boxing would \
              obscure the 1:1 mapping with the two rule languages"
)]
pub enum IrRuleBody<'a> {
    /// A SPARQL 1.2 RL rule: head templates over a sequence of rule elements.
    Elements(ElementRule),
    /// A SHACL rule, evaluated by the SHACL machinery ([`ShaclProducer`]).
    Shacl(ShaclProducer<'a>),
}

/// A SPARQL 1.2 RL rule: "a pair of a rule head (often just "head") and a rule body".
#[derive(Debug, Clone)]
pub struct ElementRule {
    /// The head: "A rule head is a sequence of triple templates." A blank node in a
    /// template is fresh per solution: "Blank nodes can be used in the rule head, and
    /// each generates a fresh blank node each time rule evaluation generates triples."
    pub head: Vec<TriplePattern>,
    /// The body elements, in authored order.
    pub body: Vec<Element>,
    /// `rule.data`: "A boolean flag; if false, the rule body is matched against the
    /// base graph" — set by `WHERE DATA`, which makes every pattern of the body match
    /// the base graph only ("It is also possible to perform all rule body matching
    /// against the base graph by using WHERE DATA").
    pub data: bool,
}

/// One rule element.
#[derive(Debug, Clone)]
pub enum Element {
    /// A triple pattern element: "a triple pattern that appears as a rule element".
    Pattern(TriplePattern),
    /// A filter element: "an expression that appears as a rule element It is used to
    /// restrict the values of variables in pattern matching." Kept when its effective
    /// boolean value is true; an evaluation error rejects the solution, as SPARQL's
    /// FILTER does.
    Filter(Expression),
    /// A negation element: "It has a negation element body comprised of a sequence of
    /// triple pattern elements and filter elements." With `data` set (`NOT DATA`) the
    /// inner patterns match the base graph only.
    Negation {
        /// The negation element body: patterns and filters only.
        elements: Vec<Self>,
        /// `negation.data`: match the inner patterns against the base graph.
        data: bool,
    },
    /// An assignment element: "a pair consisting of a variable, called the assignment
    /// variable, and an expression, called the assignment expression". "If evaluating
    /// the expression in an assignment causes an error, then the current solution mapping
    /// is rejected by the assignment."
    Assign {
        /// The assignment variable's name, without `?`.
        variable: String,
        /// The assignment expression.
        expression: Expression,
    },
}

/// A triple pattern or triple template: "3-tuple where each element is either a
/// variable or an RDF term (which might be a triple term). The second element of the
/// tuple must be an IRI or a variable."
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TriplePattern {
    /// Position 1.
    pub subject: PatternTerm,
    /// Position 2: an IRI or a variable.
    pub predicate: PatternTerm,
    /// Position 3.
    pub object: PatternTerm,
}

/// One position of a [`TriplePattern`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PatternTerm {
    /// A variable, by name without `?`.
    Variable(String),
    /// A blank node, by label. In a body pattern it "behave[s] like variables"
    /// ("each blank node in a triple pattern in R.body is replaced by a variable which
    /// is not used in the rule"); in a head template it is fresh per solution.
    BlankNode(String),
    /// A constant RDF term. A triple term here is GROUND; a triple term with variables
    /// is [`PatternTerm::Triple`].
    Term(Term),
    /// A triple term pattern with at least one variable or blank node inside.
    Triple(Box<TriplePattern>),
}

impl PatternTerm {
    /// Whether this position is, or nests, a blank node.
    fn has_blank_node(&self) -> bool {
        match self {
            Self::BlankNode(_) => true,
            Self::Triple(inner) => inner.has_blank_node(),
            Self::Variable(_) | Self::Term(_) => false,
        }
    }

    /// Record the variables of this position, blank nodes excluded.
    fn variables(&self, into: &mut Vec<String>) {
        match self {
            Self::Variable(name) => {
                if !into.contains(name) {
                    into.push(name.clone());
                }
            }
            Self::Triple(inner) => inner.variables(into),
            Self::BlankNode(_) | Self::Term(_) => {}
        }
    }
}

impl TriplePattern {
    /// A pattern over three positions.
    #[must_use]
    pub fn new(subject: PatternTerm, predicate: PatternTerm, object: PatternTerm) -> Self {
        Self {
            subject,
            predicate,
            object,
        }
    }

    /// The three positions, in order.
    #[must_use]
    pub fn positions(&self) -> [&PatternTerm; 3] {
        [&self.subject, &self.predicate, &self.object]
    }

    /// Whether any position is, or nests, a blank node.
    #[must_use]
    pub fn has_blank_node(&self) -> bool {
        self.positions()
            .into_iter()
            .any(PatternTerm::has_blank_node)
    }

    /// The pattern's variables, in first-occurrence order, blank nodes excluded.
    #[must_use]
    pub fn variable_names(&self) -> Vec<String> {
        let mut out = Vec::new();
        self.variables(&mut out);
        out
    }

    /// Record the pattern's variables into `into`.
    fn variables(&self, into: &mut Vec<String>) {
        for position in self.positions() {
            position.variables(into);
        }
    }
}

/// A SHACL rule as a producer: the rule and the shapes that link it.
#[derive(Debug, Clone)]
pub struct ShaclProducer<'a> {
    /// The parsed SHACL rule.
    pub rule: &'a ShaclRule,
    /// The non-deactivated shapes linking the rule through `sh:rule`, whose target
    /// nodes are its focus nodes; empty for a GLOBAL rule. SHACL 1.2 Inference Rules:
    /// "If the rule is a shape rule (i.e., it is linked to at least one shape via
    /// sh:rule), then the rule is executed for each target node of each of the linked
    /// and non-deactivated shapes that conform to all non-deactivated conditions of the
    /// rule. […] If the rule is a global rule (i.e., not linked to any shape via
    /// sh:rule), then the rule is executed with an empty set of focus nodes".
    pub shapes: Vec<&'a Shape>,
}

/// The variables an expression mentions, in first-occurrence order.
#[must_use]
pub fn expression_variables(expression: &Expression) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    visit_expression(expression, &mut |e| {
        if let Expression::Variable(v) | Expression::Bound(v) = e
            && !out.iter().any(|seen| seen == v.as_str())
        {
            out.push(v.as_str().to_owned());
        }
    });
    out
}

/// Whether an expression holds an `EXISTS` pattern — which a rule-element expression
/// may not: SPARQL 1.2 RL's negation is its negation element, and "The syntax of NOT
/// limits the inner body to triple patterns and filters, and does not allow nested
/// patterns, unlike SPARQL FILTER NOT EXISTS".
#[must_use]
pub fn expression_has_exists(expression: &Expression) -> bool {
    let mut found = false;
    visit_expression(expression, &mut |e| {
        found |= matches!(e, Expression::Exists(_));
    });
    found
}

/// Visit `expression` and every sub-expression, pre-order.
pub(crate) fn visit_expression(expression: &Expression, f: &mut dyn FnMut(&Expression)) {
    f(expression);
    match expression {
        Expression::NamedNode(_)
        | Expression::Literal(_)
        | Expression::Variable(_)
        | Expression::Bound(_)
        | Expression::Exists(_) => {}
        Expression::Or(a, b)
        | Expression::And(a, b)
        | Expression::Equal(a, b)
        | Expression::SameTerm(a, b)
        | Expression::Greater(a, b)
        | Expression::GreaterOrEqual(a, b)
        | Expression::Less(a, b)
        | Expression::LessOrEqual(a, b)
        | Expression::Add(a, b)
        | Expression::Subtract(a, b)
        | Expression::Multiply(a, b)
        | Expression::Divide(a, b) => {
            visit_expression(a, f);
            visit_expression(b, f);
        }
        Expression::UnaryPlus(a) | Expression::UnaryMinus(a) | Expression::Not(a) => {
            visit_expression(a, f);
        }
        Expression::In(a, list) => {
            visit_expression(a, f);
            for item in list {
                visit_expression(item, f);
            }
        }
        Expression::If(a, b, c) => {
            visit_expression(a, f);
            visit_expression(b, f);
            visit_expression(c, f);
        }
        Expression::Coalesce(list) | Expression::FunctionCall(_, list) => {
            for item in list {
                visit_expression(item, f);
            }
        }
    }
}

// ── Well-formedness ─────────────────────────────────────────────────────────────

impl ElementRule {
    /// SPARQL 1.2 RL, "Well-formedness Conditions": "A rule is a well-formed rule if the
    /// sequence of the rule body is a well-formed sequence given V0 is the empty set, and
    /// each variable in a triple template of the rule head is an element of Vall."
    ///
    /// # Errors
    ///
    /// A message naming the first violated condition.
    pub fn check_well_formed(&self) -> Result<(), String> {
        let all = check_sequence(&self.body, &[], false)?;
        for template in &self.head {
            for variable in template.variable_names() {
                if !all.contains(&variable) {
                    return Err(format!(
                        "head variable ?{variable} is not bound by the rule body (SPARQL 1.2 \
                         RL: \"each variable in a triple template of the rule head is an \
                         element of Vall\")"
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Check a sequence of rule elements against the well-formedness conditions, given the
/// initial variables `initial`, and return `Vall`.
///
/// "If elti is a filter element then every variable mentioned in a filter element is
/// an element of Vi-1. If elti is a assignment element then every variable mentioned in
/// the assignment expression is an element of Vi-1 [and] the assignment variable is an
/// not element of Vi-1. If elti is a negation element then the sequence of rule
/// elements in the negation element body is a well-formed sequence given the set of
/// variables Vi-1." A negation element body holds "a sequence of triple pattern elements
/// and filter elements" only.
fn check_sequence(
    elements: &[Element],
    initial: &[String],
    inside_negation: bool,
) -> Result<Vec<String>, String> {
    let mut bound: Vec<String> = initial.to_vec();
    for element in elements {
        match element {
            Element::Pattern(pattern) => {
                pattern.variables(&mut bound);
            }
            Element::Filter(expression) => {
                refuse_exists(expression)?;
                for variable in expression_variables(expression) {
                    if !bound.contains(&variable) {
                        return Err(format!(
                            "filter reads ?{variable} before any earlier element binds it \
                             (SPARQL 1.2 RL: \"every variable mentioned in a filter element is \
                             an element of Vi-1\")"
                        ));
                    }
                }
            }
            Element::Assign {
                variable,
                expression,
            } => {
                if inside_negation {
                    return Err(format!(
                        "an assignment to ?{variable} inside a negation element; a negation \
                         element body is \"comprised of a sequence of triple pattern elements \
                         and filter elements\""
                    ));
                }
                refuse_exists(expression)?;
                for read in expression_variables(expression) {
                    if !bound.contains(&read) {
                        return Err(format!(
                            "assignment to ?{variable} reads ?{read} before any earlier element \
                             binds it"
                        ));
                    }
                }
                if bound.contains(variable) {
                    return Err(format!(
                        "assignment to ?{variable}, which an earlier element already binds \
                         (SPARQL 1.2 RL: \"the assignment variable is an not element of \
                         Vi-1\")"
                    ));
                }
                bound.push(variable.clone());
            }
            Element::Negation { elements, .. } => {
                if inside_negation {
                    return Err(
                        "a negation element nested inside a negation element; its body holds \
                         triple patterns and filters only"
                            .to_owned(),
                    );
                }
                check_sequence(elements, &bound, true)?;
            }
        }
    }
    Ok(bound)
}

/// Refuse an `EXISTS` inside a rule-element expression (see [`expression_has_exists`]).
fn refuse_exists(expression: &Expression) -> Result<(), String> {
    if expression_has_exists(expression) {
        return Err(
            "an EXISTS pattern inside a rule-element expression; SPARQL 1.2 RL expresses \
             absence with its negation element (NOT { … })"
                .to_owned(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use purrdf_sparql_algebra::{Expression, Variable};

    use super::*;

    fn var(name: &str) -> PatternTerm {
        PatternTerm::Variable(name.to_owned())
    }

    fn iri(local: &str) -> PatternTerm {
        PatternTerm::Term(Term::NamedNode(NamedNode::from(
            format!("http://example.org/{local}").as_str(),
        )))
    }

    fn pattern(s: PatternTerm, p: &str, o: PatternTerm) -> TriplePattern {
        TriplePattern::new(s, iri(p), o)
    }

    fn rule(head: Vec<TriplePattern>, body: Vec<Element>) -> ElementRule {
        ElementRule {
            head,
            body,
            data: false,
        }
    }

    fn v(name: &str) -> Expression {
        Expression::Variable(Variable::new(name))
    }

    #[test]
    fn well_formedness_follows_the_specification() {
        // Well formed: pattern, filter over its variable, assignment, head over both.
        let ok = rule(
            vec![pattern(var("x"), "q", var("y"))],
            vec![
                Element::Pattern(pattern(var("x"), "p", var("v"))),
                Element::Filter(v("v")),
                Element::Assign {
                    variable: "y".to_owned(),
                    expression: v("v"),
                },
            ],
        );
        ok.check_well_formed().expect("well formed");

        // A filter before the pattern binding its variable.
        let early = rule(
            Vec::new(),
            vec![
                Element::Filter(v("v")),
                Element::Pattern(pattern(var("x"), "p", var("v"))),
            ],
        );
        assert!(early.check_well_formed().is_err());

        // An assignment rebinding a variable.
        let rebind = rule(
            Vec::new(),
            vec![
                Element::Pattern(pattern(var("x"), "p", var("v"))),
                Element::Assign {
                    variable: "v".to_owned(),
                    expression: v("x"),
                },
            ],
        );
        assert!(rebind.check_well_formed().is_err());

        // A head variable the body never binds.
        let unbound = rule(
            vec![pattern(var("x"), "q", var("z"))],
            vec![Element::Pattern(pattern(var("x"), "p", var("v")))],
        );
        assert!(unbound.check_well_formed().is_err());

        // A negation whose filter reads its own inner pattern's variable is fine.
        let negation = rule(
            vec![pattern(var("x"), "q", var("v"))],
            vec![
                Element::Pattern(pattern(var("x"), "p", var("v"))),
                Element::Negation {
                    elements: vec![
                        Element::Pattern(pattern(var("x"), "r", var("w"))),
                        Element::Filter(v("w")),
                    ],
                    data: false,
                },
            ],
        );
        negation.check_well_formed().expect("well formed");
    }

    #[test]
    fn run_once_follows_each_language() {
        let assign = IrRule {
            id: Term::blank("r"),
            body: IrRuleBody::Elements(rule(
                vec![pattern(var("x"), "q", var("y"))],
                vec![
                    Element::Pattern(pattern(var("x"), "p", var("v"))),
                    Element::Assign {
                        variable: "y".to_owned(),
                        expression: v("v"),
                    },
                ],
            )),
            schedule: DeclaredSchedule::default(),
            expected_predicates: Vec::new(),
        };
        assert!(assign.is_run_once(), "an assignment makes a run-once rule");
        let blank = IrRule {
            body: IrRuleBody::Elements(rule(
                vec![pattern(
                    PatternTerm::BlankNode("b".to_owned()),
                    "q",
                    var("x"),
                )],
                vec![Element::Pattern(pattern(var("x"), "p", var("v")))],
            )),
            ..assign.clone()
        };
        assert!(
            blank.is_run_once(),
            "a head blank node makes a run-once rule"
        );
        let general = IrRule {
            body: IrRuleBody::Elements(rule(
                vec![pattern(var("x"), "q", var("v"))],
                vec![Element::Pattern(pattern(var("x"), "p", var("v")))],
            )),
            ..assign
        };
        assert!(!general.is_run_once());
    }
}

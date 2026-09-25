// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Lowering the rule-set IR onto `purrdf-datalog`'s clause IR.
//!
//! One IR rule lowers to exactly one [`DlClause`], at the same index, so a datalog
//! derivation names its IR rule directly:
//!
//! | IR | clause |
//! |---|---|
//! | triple pattern element | a positive atom `triple(s, p, o, g)` |
//! | filter element | a filter guard, evaluated by SPARQL |
//! | assignment element | an assignment guard, evaluated by SPARQL |
//! | negation element | a negated conjunction of its patterns and filters |
//! | head template | a head atom; several templates a conjunctive head |
//! | head blank node | an output of a fresh-blank-node guard, one per label per solution |
//! | triple term with variables | a guard that takes a triple term apart (body) or builds one (head) |
//! | SHACL rule | one model-reading producer guard binding `?s ?p ?o`, head `triple(?s, ?p, ?o)` |
//!
//! # The graph position
//!
//! Every atom addresses the evaluation graph — the store's DEFAULT graph — except a
//! pattern that must match the BASE graph only (`WHERE DATA`, `NOT DATA`), which
//! addresses [`BASE_GRAPH`]: a partition holding a copy of the base graph, keyed by a
//! surface no RDF term renders to (an IRI renders `<…>`, a literal `"…"`, a blank node
//! `_:…`, a triple term `<<(…)>>`). It is an evaluation-internal partition key, not a
//! graph name: nothing reads it but the atoms lowered here, and it never reaches an
//! output.

use std::collections::BTreeMap;

use purrdf_datalog::clause::{ClauseAtom, ClauseTerm, DlClause, HeadDisjunct};
use purrdf_datalog::guard::{Guard, GuardReads, GuardSite, Negation};
use purrdf_sparql_algebra::{Expression, Function, GraphPattern, Variable};

use super::ir::{
    Element, ElementRule, IrRuleBody, PatternTerm, RuleSet, ShaclProducer, TriplePattern,
    expression_variables,
};
use crate::term::Term;

/// The partition the base graph is mirrored into when a rule matches it alone.
pub(crate) const BASE_GRAPH: &str = "#base";

/// How one guard of the lowered program is evaluated.
#[derive(Debug, Clone)]
pub(crate) enum GuardImpl<'r, 'a> {
    /// A SHACL rule's execution.
    Producer(&'r ShaclProducer<'a>),
    /// A filter: `query` binds `?result` to `true` or `false`, reading `variables`.
    Filter {
        /// The scalar SELECT evaluating the expression's effective boolean value.
        query: String,
        /// The canonical SPARQL variable names the query reads, in guard-input order.
        variables: Vec<String>,
    },
    /// An assignment: `query` binds `?result`, reading `variables`.
    Assign {
        /// The scalar SELECT evaluating the expression.
        query: String,
        /// The canonical SPARQL variable names the query reads, in guard-input order.
        variables: Vec<String>,
    },
    /// A fresh blank node per call.
    FreshBlank,
    /// Take a triple term apart against `pattern`. The guard's first input is the triple
    /// term; the remaining inputs are the clause variables in `pattern` already bound,
    /// its outputs the rest, both in `names` order.
    MatchTriple {
        /// The pattern, over clause variable names.
        pattern: LoweredTriple,
        /// The clause variables the guard reads after the triple term.
        inputs: Vec<String>,
        /// The clause variables the guard binds.
        outputs: Vec<String>,
    },
    /// Build a triple term from `template`, reading `inputs`.
    BuildTriple {
        /// The template, over clause variable names.
        template: LoweredTriple,
        /// The clause variables the guard reads.
        inputs: Vec<String>,
    },
}

/// A triple term pattern over clause variable names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoweredTriple(pub(crate) [LoweredPosition; 3]);

/// One position of a [`LoweredTriple`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LoweredPosition {
    /// A clause variable.
    Variable(String),
    /// A constant term.
    Constant(Term),
    /// A nested triple term pattern.
    Triple(Box<LoweredTriple>),
}

impl LoweredTriple {
    /// The clause variables of the pattern, in first-occurrence order.
    pub(crate) fn variables(&self, out: &mut Vec<String>) {
        for position in &self.0 {
            match position {
                LoweredPosition::Variable(name) => {
                    if !out.contains(name) {
                        out.push(name.clone());
                    }
                }
                LoweredPosition::Triple(inner) => inner.variables(out),
                LoweredPosition::Constant(_) => {}
            }
        }
    }
}

/// A rule set lowered onto the clause IR.
#[derive(Debug)]
pub(crate) struct Lowered<'r, 'a> {
    /// One clause per IR rule, at the IR rule's index.
    pub(crate) clauses: Vec<DlClause>,
    /// How every guard is evaluated, by `(clause, site)`.
    pub(crate) guards: BTreeMap<(usize, GuardSite), GuardImpl<'r, 'a>>,
    /// Whether some atom matches [`BASE_GRAPH`].
    pub(crate) uses_base: bool,
}

/// Lower `set`. `now`, when given, replaces every `NOW()` call — SPARQL 1.2 RL:
/// "NOW() is permitted and is defined to return the same point in time throughout a rule
/// set evaluation."
pub(crate) fn lower<'r, 'a>(
    set: &'r RuleSet<'a>,
    now: Option<&purrdf_sparql_algebra::Literal>,
) -> Lowered<'r, 'a> {
    let mut lowered = Lowered {
        clauses: Vec::with_capacity(set.rules.len()),
        guards: BTreeMap::new(),
        uses_base: false,
    };
    for (index, rule) in set.rules.iter().enumerate() {
        let clause = match &rule.body {
            IrRuleBody::Shacl(producer) => {
                let outputs = vec!["?s".to_owned(), "?p".to_owned(), "?o".to_owned()];
                lowered
                    .guards
                    .insert((index, GuardSite::Body(0)), GuardImpl::Producer(producer));
                DlClause::datalog(
                    ClauseAtom::quad(
                        ClauseTerm::var("?s"),
                        ClauseTerm::var("?p"),
                        ClauseTerm::var("?o"),
                        ClauseTerm::DefaultGraph,
                    ),
                    Vec::new(),
                )
                .with_guards(vec![Guard::new(
                    format!("shacl-rule {}", rule.id),
                    Vec::new(),
                    outputs,
                    GuardReads::Model,
                )])
            }
            IrRuleBody::Elements(element_rule) => {
                let mut builder = ClauseBuilder {
                    index,
                    now,
                    guards: Vec::new(),
                    impls: &mut lowered.guards,
                    generated: 0,
                    uses_base: false,
                    scope: None,
                };
                let clause = builder.element_rule(element_rule);
                lowered.uses_base |= builder.uses_base;
                clause
            }
        };
        lowered.clauses.push(clause);
    }
    lowered
}

/// The clause variable of an SRL variable.
fn variable(name: &str) -> String {
    format!("?{name}")
}

/// The clause variable of a BODY blank node: "each blank node in a triple pattern in
/// R.body is replaced by a variable which is not used in the rule". `.` never starts a
/// SPARQL variable name, so no generated name collides with an authored one.
fn body_blank(label: &str) -> String {
    format!("?.b{label}")
}

/// The clause variable a HEAD blank node's fresh-node guard binds.
fn head_blank(label: &str) -> String {
    format!("?.h{label}")
}

/// The clause-term constant of an RDF term: an IRI as an IRI, everything else by its
/// lexical surface — the same surface the store interns the term by.
pub(crate) fn constant(term: &Term) -> ClauseTerm {
    match term {
        Term::NamedNode(iri) => ClauseTerm::iri(iri.as_str()),
        other => ClauseTerm::literal(other.to_string()),
    }
}

/// Lowers one element rule.
struct ClauseBuilder<'m, 'r, 'a> {
    /// The rule's index.
    index: usize,
    /// The `NOW()` replacement, if any.
    now: Option<&'m purrdf_sparql_algebra::Literal>,
    /// The body guards, in order.
    guards: Vec<Guard>,
    /// The guard implementation table.
    impls: &'m mut BTreeMap<(usize, GuardSite), GuardImpl<'r, 'a>>,
    /// Generated-variable counter.
    generated: usize,
    /// Whether an atom addresses [`BASE_GRAPH`].
    uses_base: bool,
    /// While lowering negation element `index`: the enclosing rule's bound clause
    /// variables. Every other variable of the element is LOCAL to it — SPARQL 1.2 RL
    /// evaluates the element body "given the set of variables Vi-1", so a variable the
    /// rule binds only LATER is a different, existentially quantified variable inside —
    /// and is renamed apart.
    scope: Option<(usize, Vec<String>)>,
}

/// What lowering a pattern's positions produced: the atom, and the triple-term patterns
/// it deferred to guards.
struct PendingTriples(Vec<(String, LoweredTriple)>);

impl ClauseBuilder<'_, '_, '_> {
    /// Scope a clause variable name: inside a negation element, a variable the enclosing
    /// rule does not bind is renamed apart.
    fn scoped(&self, plain: String) -> String {
        match &self.scope {
            Some((index, outer)) if !outer.contains(&plain) => {
                format!("?.n{index}.{}", &plain[1..])
            }
            _ => plain,
        }
    }

    /// The clause variable of an SRL variable in the current scope.
    fn var(&self, name: &str) -> String {
        self.scoped(variable(name))
    }

    /// The clause variable of a body blank node in the current scope.
    fn blank(&self, label: &str) -> String {
        self.scoped(body_blank(label))
    }

    /// A body triple term pattern over clause variable names in the current scope.
    fn body_triple(&self, pattern: &TriplePattern) -> LoweredTriple {
        LoweredTriple(pattern.positions().map(|position| match position {
            PatternTerm::Variable(name) => LoweredPosition::Variable(self.var(name)),
            PatternTerm::BlankNode(label) => LoweredPosition::Variable(self.blank(label)),
            PatternTerm::Term(term) => LoweredPosition::Constant(term.clone()),
            PatternTerm::Triple(inner) => {
                LoweredPosition::Triple(Box::new(self.body_triple(inner)))
            }
        }))
    }

    /// A fresh generated variable.
    fn fresh(&mut self) -> String {
        self.generated += 1;
        format!("?.g{}", self.generated)
    }

    /// Lower a body position: a triple term with variables becomes a fresh variable and a
    /// deferred take-apart guard.
    fn body_position(
        &mut self,
        position: &PatternTerm,
        pending: &mut PendingTriples,
    ) -> ClauseTerm {
        match position {
            PatternTerm::Variable(name) => ClauseTerm::var(self.var(name)),
            PatternTerm::BlankNode(label) => ClauseTerm::var(self.blank(label)),
            PatternTerm::Term(term) => constant(term),
            PatternTerm::Triple(inner) => {
                let name = self.fresh();
                pending.0.push((name.clone(), self.body_triple(inner)));
                ClauseTerm::var(name)
            }
        }
    }

    /// Lower a body pattern to an atom in `graph`.
    fn body_atom(
        &mut self,
        pattern: &TriplePattern,
        base: bool,
        pending: &mut PendingTriples,
    ) -> ClauseAtom {
        let [s, p, o] = pattern
            .positions()
            .map(|position| self.body_position(position, pending));
        self.uses_base |= base;
        ClauseAtom::quad(
            s,
            p,
            o,
            if base {
                ClauseTerm::literal(BASE_GRAPH)
            } else {
                ClauseTerm::DefaultGraph
            },
        )
    }

    /// Lower an expression to a SPARQL scalar query over canonical variable names, and
    /// the clause variables it reads in the same order.
    fn expression_query(
        &self,
        expression: &Expression,
        filter: bool,
    ) -> (String, Vec<String>, Vec<String>) {
        let names = expression_variables(expression);
        let canonical: Vec<String> = (0..names.len()).map(|i| format!("v{i}")).collect();
        let mut rewritten = expression.clone();
        rewrite_expression(&mut rewritten, &names, &canonical, self.now);
        let expression = if filter {
            // The effective boolean value, with an evaluation error read as false: IF
            // raises the error, the projection is then unbound, and the solution drops.
            Expression::If(
                Box::new(rewritten),
                Box::new(Expression::Literal(boolean(true))),
                Box::new(Expression::Literal(boolean(false))),
            )
        } else {
            rewritten
        };
        let query = pattern_query(expression);
        (
            query,
            canonical,
            names.iter().map(|name| self.var(name)).collect(),
        )
    }

    /// Lower an element rule.
    fn element_rule(&mut self, rule: &ElementRule) -> DlClause {
        let mut atoms: Vec<ClauseAtom> = Vec::new();
        let mut pending = PendingTriples(Vec::new());
        // Positive patterns first: they bind before any guard runs.
        for element in &rule.body {
            if let Element::Pattern(pattern) = element {
                let atom = self.body_atom(pattern, rule.data, &mut pending);
                atoms.push(atom);
            }
        }
        let mut bound: Vec<String> = Vec::new();
        for atom in &atoms {
            for term in atom.terms() {
                if let Some(name) = term.variable()
                    && !bound.iter().any(|b| b == name)
                {
                    bound.push(name.to_owned());
                }
            }
        }
        // The take-apart guards of the positive patterns' triple terms.
        for (name, pattern) in std::mem::take(&mut pending.0) {
            self.take_apart(name, pattern, &mut bound, None);
        }
        // Filters, assignments and negations, in authored order.
        let mut negations: Vec<Negation> = Vec::new();
        for element in &rule.body {
            match element {
                Element::Pattern(_) => {}
                Element::Filter(expression) => {
                    let (query, variables, inputs) = self.expression_query(expression, true);
                    let site = GuardSite::Body(self.guards.len());
                    let name = format!("srl-filter {query}");
                    self.impls
                        .insert((self.index, site), GuardImpl::Filter { query, variables });
                    self.guards.push(Guard::filter(name, inputs));
                }
                Element::Assign {
                    variable: target,
                    expression,
                } => {
                    let (query, variables, inputs) = self.expression_query(expression, false);
                    let site = GuardSite::Body(self.guards.len());
                    let name = format!("srl-assign {query}");
                    self.impls
                        .insert((self.index, site), GuardImpl::Assign { query, variables });
                    let output = variable(target);
                    bound.push(output.clone());
                    self.guards.push(Guard::assign(name, inputs, output));
                }
                Element::Negation { elements, data } => {
                    let index = negations.len();
                    negations.push(self.negation(index, elements, *data || rule.data, &bound));
                }
            }
        }
        // The head: fresh blank nodes first, then built triple terms, then the atoms.
        let mut head_blanks: Vec<String> = Vec::new();
        for template in &rule.head {
            collect_head_blanks(template, &mut head_blanks);
        }
        for label in &head_blanks {
            let site = GuardSite::Body(self.guards.len());
            self.impls.insert((self.index, site), GuardImpl::FreshBlank);
            self.guards.push(Guard::new(
                format!("srl-bnode {label}"),
                Vec::new(),
                vec![head_blank(label)],
                GuardReads::Bindings,
            ));
        }
        let head: Vec<ClauseAtom> = rule
            .head
            .iter()
            .map(|template| {
                let [s, p, o] = template
                    .positions()
                    .map(|position| self.head_position(position));
                ClauseAtom::quad(s, p, o, ClauseTerm::DefaultGraph)
            })
            .collect();
        let clause = if head.len() == 1 {
            DlClause::datalog(head.into_iter().next().expect("one head atom"), atoms)
        } else {
            DlClause::new(vec![HeadDisjunct::new(head)], Vec::new(), atoms)
        };
        clause
            .with_guards(std::mem::take(&mut self.guards))
            .with_negations(negations)
    }

    /// Lower a head position: a triple term with variables becomes a built triple term.
    fn head_position(&mut self, position: &PatternTerm) -> ClauseTerm {
        match position {
            PatternTerm::Variable(name) => ClauseTerm::var(variable(name)),
            PatternTerm::BlankNode(label) => ClauseTerm::var(head_blank(label)),
            PatternTerm::Term(term) => constant(term),
            PatternTerm::Triple(inner) => {
                let template = head_triple(inner);
                let mut inputs = Vec::new();
                template.variables(&mut inputs);
                let output = self.fresh();
                let site = GuardSite::Body(self.guards.len());
                self.impls.insert(
                    (self.index, site),
                    GuardImpl::BuildTriple {
                        template,
                        inputs: inputs.clone(),
                    },
                );
                self.guards.push(Guard::new(
                    format!("srl-triple {inner:?}"),
                    inputs,
                    vec![output.clone()],
                    GuardReads::Bindings,
                ));
                ClauseTerm::var(output)
            }
        }
    }

    /// Emit a take-apart guard for the triple term bound to `name` against `pattern`,
    /// into the body (`negation` `None`) or into a negated conjunction's guard list.
    fn take_apart(
        &mut self,
        name: String,
        pattern: LoweredTriple,
        bound: &mut Vec<String>,
        negation: Option<(usize, &mut Vec<Guard>)>,
    ) {
        let mut variables = Vec::new();
        pattern.variables(&mut variables);
        let (inputs, outputs): (Vec<String>, Vec<String>) =
            variables.into_iter().partition(|v| bound.contains(v));
        bound.extend(outputs.iter().cloned());
        let mut guard_inputs = vec![name];
        guard_inputs.extend(inputs.iter().cloned());
        let guard = Guard::new(
            format!("srl-match {pattern:?}"),
            guard_inputs,
            outputs.clone(),
            GuardReads::Bindings,
        );
        let implementation = GuardImpl::MatchTriple {
            pattern,
            inputs,
            outputs,
        };
        match negation {
            None => {
                let site = GuardSite::Body(self.guards.len());
                self.impls.insert((self.index, site), implementation);
                self.guards.push(guard);
            }
            Some((index, guards)) => {
                let site = GuardSite::Negation {
                    negation: index,
                    guard: guards.len(),
                };
                self.impls.insert((self.index, site), implementation);
                guards.push(guard);
            }
        }
    }

    /// Lower a negation element's body into a negated conjunction.
    fn negation(
        &mut self,
        index: usize,
        elements: &[Element],
        base: bool,
        outer: &[String],
    ) -> Negation {
        let saved = self.scope.replace((index, outer.to_vec()));
        let negation = self.negation_body(index, elements, base, outer);
        self.scope = saved;
        negation
    }

    /// [`Self::negation`], inside the element's scope.
    fn negation_body(
        &mut self,
        index: usize,
        elements: &[Element],
        base: bool,
        outer: &[String],
    ) -> Negation {
        let mut atoms: Vec<ClauseAtom> = Vec::new();
        let mut pending = PendingTriples(Vec::new());
        for element in elements {
            if let Element::Pattern(pattern) = element {
                let atom = self.body_atom(pattern, base, &mut pending);
                atoms.push(atom);
            }
        }
        let mut bound: Vec<String> = outer.to_vec();
        for atom in &atoms {
            for term in atom.terms() {
                if let Some(name) = term.variable()
                    && !bound.iter().any(|b| b == name)
                {
                    bound.push(name.to_owned());
                }
            }
        }
        let mut guards: Vec<Guard> = Vec::new();
        for (name, pattern) in pending.0 {
            self.take_apart(name, pattern, &mut bound, Some((index, &mut guards)));
        }
        for element in elements {
            if let Element::Filter(expression) = element {
                let (query, variables, inputs) = self.expression_query(expression, true);
                let site = GuardSite::Negation {
                    negation: index,
                    guard: guards.len(),
                };
                let name = format!("srl-filter {query}");
                self.impls
                    .insert((self.index, site), GuardImpl::Filter { query, variables });
                guards.push(Guard::filter(name, inputs));
            }
        }
        Negation::new(atoms, guards)
    }
}

/// A HEAD triple term template over clause variable names.
fn head_triple(pattern: &TriplePattern) -> LoweredTriple {
    LoweredTriple(pattern.positions().map(|position| match position {
        PatternTerm::Variable(name) => LoweredPosition::Variable(variable(name)),
        PatternTerm::BlankNode(label) => LoweredPosition::Variable(head_blank(label)),
        PatternTerm::Term(term) => LoweredPosition::Constant(term.clone()),
        PatternTerm::Triple(inner) => LoweredPosition::Triple(Box::new(head_triple(inner))),
    }))
}

/// Record the head blank-node labels of `template`, in first-occurrence order.
fn collect_head_blanks(template: &TriplePattern, out: &mut Vec<String>) {
    for position in template.positions() {
        match position {
            PatternTerm::BlankNode(label) => {
                if !out.contains(label) {
                    out.push(label.clone());
                }
            }
            PatternTerm::Triple(inner) => collect_head_blanks(inner, out),
            PatternTerm::Variable(_) | PatternTerm::Term(_) => {}
        }
    }
}

/// An `xsd:boolean` literal.
fn boolean(value: bool) -> purrdf_sparql_algebra::Literal {
    purrdf_sparql_algebra::Literal::new_typed(
        if value { "true" } else { "false" },
        purrdf_sparql_algebra::NamedNode::new(crate::model::xsd::BOOLEAN)
            .expect("xsd:boolean is an absolute IRI"),
    )
}

/// The single-row scalar SELECT binding `?result` to `expression`.
pub(crate) fn pattern_query(expression: Expression) -> String {
    purrdf_sparql_algebra::pattern_to_select_query(&GraphPattern::Extend {
        inner: Box::new(GraphPattern::Bgp {
            patterns: Vec::new(),
        }),
        variable: Variable::new("result"),
        expression,
    })
}

/// Rename `names[i]` to `canonical[i]` throughout `expression`, and replace every `NOW()`
/// by `now` when given.
fn rewrite_expression(
    expression: &mut Expression,
    names: &[String],
    canonical: &[String],
    now: Option<&purrdf_sparql_algebra::Literal>,
) {
    let rename = |variable: &mut Variable| {
        if let Some(position) = names.iter().position(|name| name == variable.as_str()) {
            *variable = Variable::new(canonical[position].clone());
        }
    };
    match expression {
        Expression::Variable(v) | Expression::Bound(v) => rename(v),
        Expression::FunctionCall(Function::Now, args) if args.is_empty() => {
            if let Some(now) = now {
                *expression = Expression::Literal(now.clone());
            }
        }
        Expression::NamedNode(_) | Expression::Literal(_) | Expression::Exists(_) => {}
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
            rewrite_expression(a, names, canonical, now);
            rewrite_expression(b, names, canonical, now);
        }
        Expression::UnaryPlus(a) | Expression::UnaryMinus(a) | Expression::Not(a) => {
            rewrite_expression(a, names, canonical, now);
        }
        Expression::In(a, list) => {
            rewrite_expression(a, names, canonical, now);
            for item in list {
                rewrite_expression(item, names, canonical, now);
            }
        }
        Expression::If(a, b, c) => {
            rewrite_expression(a, names, canonical, now);
            rewrite_expression(b, names, canonical, now);
            rewrite_expression(c, names, canonical, now);
        }
        Expression::Coalesce(list) | Expression::FunctionCall(_, list) => {
            for item in list {
                rewrite_expression(item, names, canonical, now);
            }
        }
    }
}

/// Whether any element expression of `set` calls `NOW()`.
pub(crate) fn uses_now(set: &RuleSet<'_>) -> bool {
    let mut found = false;
    let mut look = |expression: &Expression| {
        super::ir::visit_expression(expression, &mut |e| {
            found |= matches!(e, Expression::FunctionCall(Function::Now, _));
        });
    };
    for rule in &set.rules {
        if let IrRuleBody::Elements(rule) = &rule.body {
            visit_elements(&rule.body, &mut look);
        }
    }
    found
}

/// Visit every expression of `elements`, negation bodies included.
fn visit_elements(elements: &[Element], f: &mut dyn FnMut(&Expression)) {
    for element in elements {
        match element {
            Element::Filter(expression) | Element::Assign { expression, .. } => f(expression),
            Element::Negation { elements, .. } => visit_elements(elements, f),
            Element::Pattern(_) => {}
        }
    }
}

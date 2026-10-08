// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrowed structural admission for compiler-built query algebra.

use std::collections::BTreeSet;

use crate::walk::NodeRef;
use crate::{
    AggregateExpression, Expression, Function, GraphPattern, GroundTerm, Literal, NamedNodePattern,
    ParseError, PropertyPathExpression, Query, Result, TermPattern, TriplePattern, Variable,
};

/// A term position that may hold a triple term: the one shape the nesting count
/// walks, shared by pattern terms and `VALUES` cells.
trait TripleTermSlot: Sized {
    /// The subject and object of the triple term this is, or `None` for a
    /// non-triple term.
    fn subject_and_object(&self) -> Option<(&Self, &Self)>;
}

/// The slot implementation for each term kind whose `Triple` variant holds a
/// triple with `subject` and `object` of that same kind.
macro_rules! triple_term_slot {
    ($($term:ty),+) => {$(
        impl TripleTermSlot for $term {
            fn subject_and_object(&self) -> Option<(&Self, &Self)> {
                match self {
                    Self::Triple(triple) => Some((&triple.subject, &triple.object)),
                    _ => None,
                }
            }
        }
    )+};
}

triple_term_slot!(TermPattern, GroundTerm);

/// How many triple terms `root`'s longest chain holds, the outermost included.
///
/// The single nesting count for every term kind. Counted iteratively, following
/// the object position in a loop and parking a subject only when it is itself a
/// triple term, so it needs no more stack however deep the term nests and does
/// not allocate unless a subject nests.
fn triple_term_nesting<T: TripleTermSlot>(root: &T) -> usize {
    let mut deepest = 0;
    let mut pending: Vec<(&T, usize)> = Vec::new();
    let mut next = Some((root, 0));
    while let Some((term, above)) = next.take().or_else(|| pending.pop()) {
        if let Some((subject, object)) = term.subject_and_object() {
            let depth = above + 1;
            deepest = deepest.max(depth);
            if subject.subject_and_object().is_some() {
                pending.push((subject, depth));
            }
            next = Some((object, depth));
        }
    }
    deepest
}

impl TermPattern {
    /// How many triple terms this term's longest chain holds, the outermost included:
    /// `0` for an IRI, blank node, literal or variable, `1` for `<<( ?s ?p ?o )>>`, `2`
    /// for `<<( ?s ?p <<( ?s ?p ?o )>> )>>`.
    ///
    /// Counted iteratively, so it needs no more stack however deep the term nests, and
    /// without allocating unless a triple term's subject is itself one.
    #[must_use]
    pub fn triple_term_nesting(&self) -> usize {
        triple_term_nesting(self)
    }

    /// Call `visit` on every variable this term mentions, a quoted triple term's own
    /// positions included — subject, predicate and object, at every depth. A
    /// variable mentioned twice is visited twice.
    ///
    /// The one variable walk over a term pattern every analysis shares. It keeps its
    /// own work list, so a term nested to any depth needs no more machine stack, and
    /// a term that is not a triple term is answered without allocating.
    pub fn for_each_variable<'a>(&'a self, mut visit: impl FnMut(&'a Variable)) {
        let mut pending = match self {
            Self::Variable(variable) => return visit(variable),
            Self::Triple(_) => vec![self],
            Self::NamedNode(_) | Self::BlankNode(_) | Self::Literal(_) => return,
        };
        while let Some(term) = pending.pop() {
            match term {
                Self::Variable(variable) => visit(variable),
                Self::Triple(triple) => {
                    if let NamedNodePattern::Variable(variable) = &triple.predicate {
                        visit(variable);
                    }
                    pending.push(&triple.object);
                    pending.push(&triple.subject);
                }
                Self::NamedNode(_) | Self::BlankNode(_) | Self::Literal(_) => {}
            }
        }
    }

    /// Add every variable this term mentions to `out`, a quoted triple term's own
    /// positions included (see [`Self::for_each_variable`]).
    pub fn collect_variables(&self, out: &mut impl Extend<Variable>) {
        self.for_each_variable(|variable| out.extend([variable.clone()]));
    }

    /// Add the name of every variable this term mentions to `out`, a quoted triple
    /// term's own positions included (see [`Self::for_each_variable`]).
    pub fn collect_variable_names(&self, out: &mut BTreeSet<String>) {
        self.for_each_variable(|variable| {
            out.insert(variable.as_str().to_owned());
        });
    }
}

impl TriplePattern {
    /// Add the name of every variable this triple pattern mentions to `out`: its
    /// subject's and object's (quoted triple terms included) and a variable
    /// predicate.
    pub fn collect_variable_names(&self, out: &mut BTreeSet<String>) {
        self.subject.collect_variable_names(out);
        if let NamedNodePattern::Variable(variable) = &self.predicate {
            out.insert(variable.as_str().to_owned());
        }
        self.object.collect_variable_names(out);
    }
}

impl GroundTerm {
    /// How many triple terms this `VALUES` cell's longest chain holds, the outermost
    /// included; see [`TermPattern::triple_term_nesting`].
    #[must_use]
    pub fn triple_term_nesting(&self) -> usize {
        triple_term_nesting(self)
    }
}

impl GraphPattern {
    /// Check the reserved non-distinguished identity contract without changing
    /// the admission rules for ordinary raw-algebra names or terms.
    ///
    /// # Errors
    /// Refuses explicit hidden projection, grouping, output or expression references.
    pub fn validate_hidden_variables(&self) -> Result<()> {
        crate::scope::validate_pattern(self).map_err(ParseError::from)
    }
}

impl Query {
    /// Check reserved non-distinguished identities at a raw query entry, including
    /// graph templates and description targets, without re-admitting ordinary terms.
    ///
    /// # Errors
    /// Refuses an explicit observation of a hidden match witness.
    pub fn validate_hidden_variables(&self) -> Result<()> {
        crate::scope::validate_query(self).map_err(ParseError::from)
    }

    /// Check the structural invariants needed to evaluate compiler-built algebra.
    ///
    /// Walks borrowed nodes without rendering or parsing SPARQL. Runtime expression
    /// errors (including ill-typed datatype lexical forms) remain runtime errors.
    /// Blank labels remain opaque identifiers, including scope-qualified labels.
    /// Registry admission belongs to the evaluator, which owns those registries.
    ///
    /// Walks the tree over a work list, so a tree of any height is checked without
    /// recursion; how tall a tree may be to be *evaluated* is the evaluator's to decide.
    ///
    /// # Errors
    /// Refuses invalid absolute IRIs, language tags, binding widths and output-name
    /// collisions, and malformed typed calls or ranges.
    pub fn validate(&self) -> Result<()> {
        self.walk::<true>(&mut BTreeSet::new())
    }

    /// Admit algebra for ordinary parameter and SHACL transformations.
    /// Contextual application has private input/output identities and is admitted
    /// only through the typed contextual preparation boundary.
    ///
    /// # Errors
    /// The structural diagnostics of [`Self::validate`], or a contextual application.
    pub fn validate_ordinary(&self) -> Result<()> {
        self.walk::<false>(&mut BTreeSet::new())
    }

    /// The IRIs of every extension function the query calls: each
    /// [`Function::Custom`] in an expression anywhere in the algebra — a `FILTER`, a
    /// `BIND`, a projection, an `ORDER BY`, a `HAVING`, an aggregate's argument, and
    /// inside `EXISTS` / `NOT EXISTS` and sub-queries — sorted and de-duplicated.
    ///
    /// A host that can say which IRIs it cannot evaluate learns, before running the
    /// query, whether the query would reach one. Custom aggregates
    /// ([`crate::AggregateFunction::Custom`]) are a separate seam and are not listed.
    ///
    /// # Errors
    /// Whatever [`Self::validate`] refuses: the walk is the same one.
    pub fn custom_function_calls(&self) -> Result<BTreeSet<String>> {
        let mut calls = BTreeSet::new();
        self.walk::<true>(&mut calls)?;
        Ok(calls.into_iter().map(ToOwned::to_owned).collect())
    }

    /// [`Self::validate`]'s walk, recording every [`Function::Custom`] IRI it passes
    /// in `calls`.
    fn walk<'a, const APPLICATION: bool>(&'a self, calls: &mut BTreeSet<&'a str>) -> Result<()> {
        let (pattern, dataset, base) = match self {
            Self::Select {
                pattern,
                dataset,
                base_iri,
                ..
            }
            | Self::Ask {
                pattern,
                dataset,
                base_iri,
                ..
            }
            | Self::Construct {
                pattern,
                dataset,
                base_iri,
                ..
            }
            | Self::Describe {
                pattern,
                dataset,
                base_iri,
                ..
            } => (pattern, dataset, base_iri),
        };
        for name in dataset.default.iter().chain(&dataset.named) {
            iri(name.as_str())?;
        }
        if let Some(base) = base {
            purrdf_iri::BaseIri::parse(base.as_str()).map_err(|e| invalid(e.to_string()))?;
        }
        crate::scope::validate_query_head(self).map_err(ParseError::from)?;
        let mut stack = vec![NodeRef::Pattern(pattern)];
        match self {
            Self::Construct { template, .. } => {
                for quad in template {
                    if let Some(graph) = &quad.graph {
                        named(graph)?;
                    }
                    stack.push(NodeRef::Triple(&quad.triple));
                }
            }
            Self::Describe { targets, .. } => {
                for target in targets {
                    named(target)?;
                }
            }
            Self::Select { .. } | Self::Ask { .. } => {}
        }
        visit_nodes(stack, |node| check::<APPLICATION>(node, calls))
    }
}

/// The shared borrowed admission walk, with one check per node and no recursion.
fn visit_nodes<'a>(
    stack: Vec<NodeRef<'a>>,
    mut check: impl FnMut(NodeRef<'a>) -> Result<()>,
) -> Result<()> {
    for root in stack.into_iter().rev() {
        crate::scope::walk_nodes(root, |node, index| {
            crate::scope::check_node(node, crate::scope::ScopeSite::pattern(index))
                .map_err(ParseError::from)?;
            check(node)
        })?;
    }
    Ok(())
}

pub(crate) use crate::scope::for_each_quad_variable;

/// The one output rule shared by query admission and update carriers.
pub(crate) fn check_quad_output(quad: &crate::QuadPattern) -> Result<()> {
    crate::scope::validate_quad(quad).map_err(ParseError::from)
}

fn invalid(message: impl Into<String>) -> ParseError {
    ParseError::syntax(message, 0)
}

/// Accept `value` iff it is a well-formed **absolute** IRI.
///
/// `purrdf_iri::is_absolute` rather than `parse(..)?.has_scheme()`: the two run the
/// same grammar and return the same errors, but `parse` owns a copy of `value` so
/// its component accessors can hand back slices, and this reads one bit and drops
/// it. That copy is a heap `String` per IRI in the query, charged on every
/// `Query::validate` — which a governed SHACL change path used to reach once per
/// focus node. The swap is an ALLOCATION change and not a validation one: what is
/// accepted and what is rejected here is unchanged, which
/// `tests::absolute_iris_are_still_accepted_and_relative_ones_still_refused` pins
/// from both sides.
fn iri(value: &str) -> Result<()> {
    if purrdf_iri::is_absolute(value).map_err(|e| invalid(e.to_string()))? {
        Ok(())
    } else {
        Err(invalid("relative IRI in query algebra"))
    }
}

fn variable(value: &Variable) -> Result<()> {
    // `VARNAME` is position-dependent, so this is a whole-string test and not a
    // per-character one: a name may CONTINUE with a combining mark but may not
    // BEGIN with one, and no position admits `'-'`.
    if !value.is_hidden() && !crate::lexer::is_varname(value.as_str()) {
        return Err(invalid("invalid query variable name"));
    }
    Ok(())
}

fn distinct_variables<'a>(values: impl IntoIterator<Item = &'a Variable>) -> Result<()> {
    let mut seen = BTreeSet::new();
    for value in values {
        variable(value)?;
        if !seen.insert(value) {
            return Err(invalid("duplicate output variable in query algebra"));
        }
    }
    Ok(())
}

fn named(value: &NamedNodePattern) -> Result<()> {
    match value {
        NamedNodePattern::NamedNode(n) => iri(n.as_str()),
        NamedNodePattern::Variable(v) => variable(v),
    }
}

fn literal(value: &Literal) -> Result<()> {
    iri(value.datatype().as_str())?;
    match value.language() {
        Some(_) => {
            let expected = purrdf_iri::vocab::language_datatype_iri(value.direction().is_some());
            if value.datatype().as_str() != expected {
                return Err(invalid(
                    "literal datatype disagrees with its language and direction",
                ));
            }
        }
        None if value.direction().is_some()
            || matches!(
                value.datatype().as_str(),
                crate::ast::RDF_LANG_STRING | crate::ast::RDF_DIR_LANG_STRING
            ) =>
        {
            return Err(invalid(
                "a language-string datatype or direction requires a language tag",
            ));
        }
        None => {}
    }
    if value
        .language()
        .is_some_and(|tag| !crate::parser::is_langtag(tag))
    {
        return Err(invalid("invalid language tag in query algebra"));
    }
    Ok(())
}

/// The checks of `node` itself; its children are checked when they are reached.
/// Every [`Function::Custom`] IRI `node` calls is recorded in `calls`.
fn check<'a, const APPLICATION: bool>(
    node: NodeRef<'a>,
    calls: &mut BTreeSet<&'a str>,
) -> Result<()> {
    match node {
        NodeRef::Pattern(pattern) => check_pattern::<APPLICATION>(pattern),
        NodeRef::Expr(expr) => check_expression(expr, calls),
        NodeRef::Path(path) => check_path(path),
        NodeRef::Triple(triple) => named(&triple.predicate),
        NodeRef::Term(term) => match term {
            TermPattern::NamedNode(n) => iri(n.as_str()),
            TermPattern::Variable(v) => variable(v),
            TermPattern::Literal(l) => literal(l),
            TermPattern::Triple(_) | TermPattern::BlankNode(_) => Ok(()),
        },
        NodeRef::Ground(term) => match term {
            GroundTerm::NamedNode(n) => iri(n.as_str()),
            GroundTerm::Literal(l) => literal(l),
            GroundTerm::BlankNode(_) => Ok(()),
            GroundTerm::Triple(t) => {
                if matches!(t.subject, GroundTerm::Literal(_) | GroundTerm::Triple(_)) {
                    return Err(invalid(
                        "a ground triple term requires an IRI or blank subject",
                    ));
                }
                iri(t.predicate.as_str())
            }
        },
        NodeRef::Order(_) => Ok(()),
        NodeRef::Aggregate(aggregate) => check_aggregate(aggregate),
    }
}

fn check_aggregate(value: &AggregateExpression) -> Result<()> {
    if let crate::AggregateFunction::Custom(name) = value.function() {
        iri(name.as_str())?;
    }
    for (_, value) in value.scalarvals() {
        literal(value)?;
    }
    Ok(())
}

fn check_pattern<const APPLICATION: bool>(pattern: &GraphPattern) -> Result<()> {
    use GraphPattern as G;
    match pattern {
        G::Apply {
            left,
            right,
            policy,
        } => {
            if !APPLICATION {
                return Err(invalid(
                    "contextual application requires typed contextual preparation",
                ));
            }
            if policy.dataset_required
                && (policy.row_pipeline
                    || policy.reduced_adjacent
                    || policy.group_domain.is_some()
                    || !policy.inputs.is_empty()
                    || policy.optional.is_some()
                    || !matches!(&**left, G::Bgp { patterns } if patterns.is_empty())
                    || !matches!(&**right, G::Graph { .. }))
            {
                return Err(invalid(
                    "dataset-required application requires an uncorrelated Graph operand and empty driver",
                ));
            }
            if policy.reduced_adjacent
                && (policy.group_domain.is_some()
                    || policy.row_pipeline
                    || !policy.inputs.is_empty()
                    || policy.optional.is_some()
                    || !matches!(&**left, G::Bgp {patterns} if patterns.is_empty())
                    || !matches!(&**right, G::Reduced { .. }))
            {
                return Err(invalid(
                    "adjacent reduction requires an uncorrelated Reduced operand and empty driver",
                ));
            }
            if policy.row_pipeline {
                if policy.group_domain.is_some()
                    || policy.reduced_adjacent
                    || policy.optional.is_some()
                {
                    return Err(invalid(
                        "scalar continuation cannot carry group, reduction or retry policies",
                    ));
                }
                let mut row = &**right;
                loop {
                    match row {
                        G::Extend { inner, .. }
                        | G::Filter { inner, .. }
                        | G::Project { inner, .. } => row = inner,
                        G::Bgp { patterns } if patterns.is_empty() => break,
                        _ => {
                            return Err(invalid(
                                "scalar continuation requires a zero-or-one-row scalar operand",
                            ));
                        }
                    }
                }
            }
            if let Some(domain) = &policy.group_domain {
                if !matches!(&**left, G::Bgp { patterns } if patterns.is_empty())
                    || !matches!(&**right, G::Group { .. })
                    || policy.row_pipeline
                    || policy.reduced_adjacent
                    || !policy.inputs.is_empty()
                    || policy.optional.is_some()
                {
                    return Err(invalid(
                        "group mapping domain requires an uncorrelated Group operand and empty driver",
                    ));
                }
                let mut seen = BTreeSet::new();
                for name in domain {
                    variable(name)?;
                    if !seen.insert(name) {
                        return Err(invalid("group mapping domain repeats a column"));
                    }
                }
            }
            let mut inputs = BTreeSet::new();
            for (input, driver) in &policy.inputs {
                variable(input)?;
                variable(driver)?;
                if !inputs.insert(input) {
                    return Err(invalid("application input is declared twice"));
                }
            }
            if let Some(optional) = &policy.optional {
                variable(&optional.forget_marker)?;
                if inputs.contains(&optional.forget_marker) {
                    return Err(invalid(
                        "optional visibility marker collides with an application input",
                    ));
                }
                let mut retry = BTreeSet::new();
                for (input, driver) in &optional.retry_inputs {
                    variable(input)?;
                    variable(driver)?;
                    if !inputs.contains(input) {
                        return Err(invalid("retry refers to an undeclared application input"));
                    }
                    if !retry.insert(input) {
                        return Err(invalid("retry application input is declared twice"));
                    }
                }
            }
        }
        G::Bgp { .. }
        | G::Path { .. }
        | G::Join { .. }
        | G::Lateral { .. }
        | G::Minus { .. }
        | G::Union { .. }
        | G::LeftJoin { .. }
        | G::Filter { .. }
        | G::OrderBy { .. }
        | G::Distinct { .. }
        | G::Reduced { .. }
        | G::Slice { .. } => {}
        G::Graph { name, .. } | G::Service { name, .. } => named(name)?,
        G::Extend {
            variable: target, ..
        } => variable(target)?,
        G::Values {
            variables,
            bindings,
        } => {
            distinct_variables(variables)?;
            if bindings.iter().any(|row| row.len() != variables.len()) {
                return Err(invalid("VALUES row width differs from its variables"));
            }
        }
        G::Project { variables, .. } => {
            // Repeated projection variables are normalized by the result schema.
            for value in variables {
                variable(value)?;
            }
        }
        G::Group {
            variables,
            aggregates,
            ..
        } => {
            let mut outputs = BTreeSet::new();
            for value in variables {
                variable(value)?;
                outputs.insert(value);
            }
            for (target, _) in aggregates {
                variable(target)?;
                if !outputs.insert(target) {
                    return Err(invalid(
                        "aggregate output collides with another group output",
                    ));
                }
            }
        }
        G::PropertyFunction(call) => iri(&call.iri)?,
        G::Unfold {
            element, companion, ..
        } => distinct_variables(std::iter::once(element).chain(companion))?,
    }
    Ok(())
}

fn check_expression<'a>(expr: &'a Expression, calls: &mut BTreeSet<&'a str>) -> Result<()> {
    use Expression as E;
    match expr {
        E::NamedNode(n) => iri(n.as_str())?,
        E::Literal(l) => literal(l)?,
        E::Variable(v) | E::Bound(v) => variable(v)?,
        E::FunctionCall(function, args) => match function {
            Function::Custom(n) => {
                iri(n.as_str())?;
                calls.insert(n.as_str());
            }
            Function::Cdt(call) => {
                if crate::CdtFn::from_iri(&call.iri) != Some(call.fn_kind)
                    || !call.fn_kind.arity().admits(args.len())
                {
                    return Err(invalid(
                        "invalid composite datatype function identity or arity",
                    ));
                }
            }
            Function::Purrdf(call) => {
                iri(&call.iri)?;
                if !call.iri.ends_with(call.local_name()) {
                    return Err(invalid("extension function IRI disagrees with its kind"));
                }
            }
            Function::Adjust if args.len() != 2 => {
                return Err(invalid("ADJUST requires two arguments"));
            }
            _ => {}
        },
        E::Or(_)
        | E::And(_)
        | E::Arithmetic(..)
        | E::Equal(..)
        | E::SameTerm(..)
        | E::Greater(..)
        | E::GreaterOrEqual(..)
        | E::Less(..)
        | E::LessOrEqual(..)
        | E::UnaryPlus(_)
        | E::UnaryMinus(_)
        | E::Not(_)
        | E::In(..)
        | E::If(..)
        | E::Coalesce(_)
        | E::Exists(_) => {}
    }
    Ok(())
}

fn check_path(path: &PropertyPathExpression) -> Result<()> {
    use PropertyPathExpression as P;
    match path {
        P::NamedNode(n) => iri(n.as_str())?,
        P::Reverse(_)
        | P::ZeroOrMore(_)
        | P::OneOrMore(_)
        | P::ZeroOrOne(_)
        | P::Sequence(_)
        | P::Alternative(_) => {}
        P::Range { min, max, .. } => {
            if max.is_some_and(|max| *min > max) {
                return Err(invalid("path range lower bound exceeds upper bound"));
            }
        }
        P::NegatedPropertySet(values) => {
            for v in values {
                iri(v.predicate.as_str())?;
            }
        }
        P::Wildcard { namespace } => {
            if let Some(n) = namespace {
                iri(n.as_str())?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::SparqlParser;
    use crate::tree::Child;
    use crate::{
        GroundTerm, GroundTriple, NamedNode, NamedNodePattern, TermPattern, TriplePattern, Variable,
    };

    fn pattern_triple(subject: TermPattern, object: TermPattern) -> TermPattern {
        TermPattern::Triple(Child::new(TriplePattern {
            subject,
            predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked(
                "http://example.org/p",
            )),
            object,
        }))
    }

    fn ground_triple(subject: GroundTerm, object: GroundTerm) -> GroundTerm {
        GroundTerm::Triple(Child::new(GroundTriple {
            subject,
            predicate: NamedNode::new_unchecked("http://example.org/p"),
            object,
        }))
    }

    #[test]
    fn pattern_nesting_counts_the_longest_chain_through_subject_or_object() {
        let var = || TermPattern::Variable(Variable::new("x"));
        assert_eq!(var().triple_term_nesting(), 0);
        assert_eq!(pattern_triple(var(), var()).triple_term_nesting(), 1);
        let object_deep = pattern_triple(var(), pattern_triple(var(), var()));
        assert_eq!(object_deep.triple_term_nesting(), 2);
        let subject_deep = pattern_triple(
            pattern_triple(pattern_triple(var(), var()), var()),
            pattern_triple(var(), var()),
        );
        assert_eq!(subject_deep.triple_term_nesting(), 3);
    }

    #[test]
    fn ground_nesting_counts_the_longest_chain_through_subject_or_object() {
        let iri = || GroundTerm::NamedNode(NamedNode::new_unchecked("http://example.org/a"));
        assert_eq!(iri().triple_term_nesting(), 0);
        assert_eq!(ground_triple(iri(), iri()).triple_term_nesting(), 1);
        let subject_deep = ground_triple(ground_triple(ground_triple(iri(), iri()), iri()), iri());
        assert_eq!(subject_deep.triple_term_nesting(), 3);
        let mut chain = iri();
        for _ in 0..10_000 {
            chain = ground_triple(iri(), chain);
        }
        assert_eq!(chain.triple_term_nesting(), 10_000);
    }

    /// Every extension-function call is listed, wherever the expression sits — a
    /// `FILTER`, a `BIND`, a projection, an aggregate's argument, and a `FILTER` inside
    /// `NOT EXISTS` — and nothing else is: the same IRIs in predicate or object position
    /// and a built-in call are not calls.
    #[test]
    fn every_extension_function_call_is_listed_and_a_predicate_iri_is_not() {
        let query = SparqlParser::new()
            .parse_query(
                "PREFIX ex: <http://example.org/ns#>
                 SELECT ?s (ex:project(?s) AS ?p) (SUM(ex:aggregated(?o)) AS ?sum)
                 WHERE {
                   ?s ex:inFilter ?o .
                   ?s ex:object ex:inFilter .
                   BIND (ex:bound(?o) AS ?b)
                   FILTER (ex:inFilter(?o) && STRLEN(STR(?o)) > 0)
                   FILTER NOT EXISTS { ?s ?q ?r FILTER (ex:inExists(?r)) }
                 }
                 GROUP BY ?s",
            )
            .expect("the query parses");
        let calls: Vec<String> = query
            .custom_function_calls()
            .expect("the query is well-formed")
            .into_iter()
            .collect();
        assert_eq!(
            calls,
            [
                "http://example.org/ns#aggregated",
                "http://example.org/ns#bound",
                "http://example.org/ns#inExists",
                "http://example.org/ns#inFilter",
                "http://example.org/ns#project",
            ]
        );
        let none = SparqlParser::new()
            .parse_query(
                "PREFIX ex: <http://example.org/ns#>
                 SELECT ?s WHERE { ?s ex:inFilter ex:bound FILTER (STRLEN(STR(?s)) > 0) }",
            )
            .expect("the query parses");
        assert!(
            none.custom_function_calls()
                .expect("the query is well-formed")
                .is_empty()
        );
    }
}

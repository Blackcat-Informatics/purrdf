// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The public parse entry point and the recursive-descent parser that turns a
//! SPARQL 1.1/1.2 query into the [`Query`] algebra.
//!
//! The parser translates *directly* into the W3C SPARQL algebra (§18.2) rather
//! than building a separate syntax tree: group graph patterns accumulate into
//! `Join`/`LeftJoin`/`Filter`/`Extend`/`Union`/`Minus`/`Graph`, solution
//! modifiers wrap the result as `Group`/`OrderBy`/`Project`/`Distinct`/`Slice`,
//! and aggregates are lifted to synthetic variables in a `Group` node (the
//! standard §18.2.4 mechanism). Anything outside the corpus-driven scope is a
//! hard [`ParseError::Unsupported`].

use std::collections::HashMap;

use crate::algebra::{
    AggregateExpression, AggregateFunction, Expression, Function, GraphPattern, GraphTarget,
    GraphUpdateOperation, NegatedPathElement, OrderExpression, PropertyPathExpression, Query,
    QueryDataset, Update, UsingClause,
};
use crate::ast::{
    BaseDirection, BlankNode, GroundTerm, GroundTriple, Literal, NamedNode, NamedNodePattern,
    QuadPattern, TermPattern, TriplePattern, Variable,
};
use crate::error::{ParseError, Result};
use crate::lexer::{tokenize, Spanned, Token};

const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";
const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";
const XSD_INTEGER: &str = "http://www.w3.org/2001/XMLSchema#integer";
const XSD_DECIMAL: &str = "http://www.w3.org/2001/XMLSchema#decimal";
const XSD_DOUBLE: &str = "http://www.w3.org/2001/XMLSchema#double";
const XSD_BOOLEAN: &str = "http://www.w3.org/2001/XMLSchema#boolean";

/// Parse-time configuration for the SPARQL front-end.
///
/// The single knob today is [`Self::extension_fn_namespaces`]: the set of IRI
/// namespaces the parser recognizes as the **extension-function seam**. An IRI
/// in call position (immediately followed by `(`) whose string starts with any
/// configured namespace is stripped to its local name and dispatched into the
/// CLOSED [`crate::algebra::PurrdfFn`] set; an *unknown* local name under a
/// configured namespace is a hard [`ParseError`] (never a silent
/// [`Function::Custom`] fallthrough).
///
/// The default is **EMPTY**: PurRDF is a library, not an ontology, and mints no
/// vocabulary IRIs of its own — with no configured namespace the extension seam
/// is off and every call-position IRI is an ordinary [`Function::Custom`] (no
/// error, no special-casing). A deployment whose queries spell the closed
/// function set under its own ontology namespace — e.g. gmeow's
/// `https://blackcatinformatics.ca/gmeow/` with `gmeow:heldIn(...)` — supplies
/// that namespace here; the local names are fixed.
///
/// Note the serializer does **not** consult this configuration: a
/// [`Function::Purrdf`] re-emits the ORIGINAL IRI it was parsed from (recorded
/// in [`crate::algebra::PurrdfCall::iri`] — see `serialize.rs`), so re-parsing
/// that output with the same options round-trips to the same algebra and no
/// namespace is ever fabricated on output.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ParserOptions {
    /// The namespaces recognized as the extension-function seam in call position.
    /// Defaults to empty (extension functions off); order is first-match-wins
    /// for prefix stripping.
    pub extension_fn_namespaces: Vec<String>,
}

/// A reusable SPARQL query parser.
///
/// Mirrors the prior oxigraph-family `SparqlParser` surface the existing
/// consumers call so the port is mechanical: `SparqlParser::new().parse_query(text)`.
/// Parse-time configuration (the extension-function namespace set) is passed per
/// call via [`SparqlParser::parse_query_with`] / [`SparqlParser::parse_update_with`];
/// the plain `parse_*` entries use [`ParserOptions::default`].
#[derive(Clone, Debug, Default)]
pub struct SparqlParser {
    base_iri: Option<String>,
}

impl SparqlParser {
    /// Construct a parser with no implicit base IRI.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set an implicit base IRI used to resolve relative IRI references that
    /// appear before any in-query `BASE` declaration.
    #[must_use]
    pub fn with_base_iri(mut self, base_iri: impl Into<String>) -> Self {
        self.base_iri = Some(base_iri.into());
        self
    }

    /// Parse a SPARQL 1.1/1.2 query into the algebra, under [`ParserOptions::default`].
    pub fn parse_query(&self, query: &str) -> Result<Query> {
        self.parse_query_with(query, &ParserOptions::default())
    }

    /// Parse a SPARQL 1.1/1.2 query into the algebra with explicit [`ParserOptions`]
    /// (e.g. an extra extension-function namespace alias).
    pub fn parse_query_with(&self, query: &str, options: &ParserOptions) -> Result<Query> {
        let mut p = self.parser_for(query, options)?;
        let q = p.parse_query()?;
        p.expect_eof()?;
        Ok(q)
    }

    /// Parse a SPARQL 1.1 Update request into the [`Update`] algebra, under
    /// [`ParserOptions::default`].
    pub fn parse_update(&self, update: &str) -> Result<Update> {
        self.parse_update_with(update, &ParserOptions::default())
    }

    /// Parse a SPARQL 1.1 Update request into the [`Update`] algebra with explicit
    /// [`ParserOptions`].
    pub fn parse_update_with(&self, update: &str, options: &ParserOptions) -> Result<Update> {
        let mut p = self.parser_for(update, options)?;
        let u = p.parse_update()?;
        p.expect_eof()?;
        Ok(u)
    }

    /// Tokenize `text` and assemble the internal recursive-descent parser state.
    fn parser_for<'o>(&self, text: &str, options: &'o ParserOptions) -> Result<Parser<'o>> {
        let tokens = tokenize(text)?;
        Ok(Parser {
            tokens,
            pos: 0,
            prefixes: HashMap::new(),
            base: self.base_iri.clone(),
            agg_counter: 0,
            anon_counter: 0,
            group_counter: 0,
            options,
        })
    }
}

struct Parser<'o> {
    tokens: Vec<Spanned>,
    pos: usize,
    prefixes: HashMap<String, String>,
    base: Option<String>,
    agg_counter: usize,
    anon_counter: usize,
    group_counter: usize,
    options: &'o ParserOptions,
}

impl Parser<'_> {
    // ── token cursor ─────────────────────────────────────────────────────────

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|s| &s.token)
    }

    fn peek2(&self) -> Option<&Token> {
        self.tokens.get(self.pos + 1).map(|s| &s.token)
    }

    fn span(&self) -> usize {
        self.tokens
            .get(self.pos)
            .map_or_else(|| self.tokens.last().map_or(0, |s| s.end), |s| s.start)
    }

    fn bump(&mut self) -> Option<Token> {
        let t = self.tokens.get(self.pos).map(|s| s.token.clone());
        if t.is_some() {
            self.pos += 1;
        }
        t
    }

    fn at(&self, t: &Token) -> bool {
        self.peek() == Some(t)
    }

    fn eat(&mut self, t: &Token) -> bool {
        if self.at(t) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, t: &Token) -> Result<()> {
        if self.eat(t) {
            Ok(())
        } else {
            Err(ParseError::syntax(
                format!("expected {t:?}, found {:?}", self.peek()),
                self.span(),
            ))
        }
    }

    /// Is the current token the keyword `kw` (case-insensitive `Word`)?
    fn peek_kw(&self, kw: &str) -> bool {
        matches!(self.peek(), Some(Token::Word(w)) if w.eq_ignore_ascii_case(kw))
    }

    fn peek2_kw(&self, kw: &str) -> bool {
        matches!(self.peek2(), Some(Token::Word(w)) if w.eq_ignore_ascii_case(kw))
    }

    fn eat_kw(&mut self, kw: &str) -> bool {
        if self.peek_kw(kw) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect_kw(&mut self, kw: &str) -> Result<()> {
        if self.eat_kw(kw) {
            Ok(())
        } else {
            Err(ParseError::syntax(
                format!("expected keyword {kw}, found {:?}", self.peek()),
                self.span(),
            ))
        }
    }

    fn expect_eof(&self) -> Result<()> {
        if self.pos >= self.tokens.len() {
            Ok(())
        } else {
            Err(ParseError::syntax(
                format!("unexpected trailing token {:?}", self.peek()),
                self.span(),
            ))
        }
    }

    // ── prologue + query form ────────────────────────────────────────────────

    fn parse_query(&mut self) -> Result<Query> {
        self.parse_prologue()?;
        let base_iri = self.base.clone().map(NamedNode::new).transpose()?;
        if self.peek_kw("SELECT") {
            self.parse_select(base_iri)
        } else if self.peek_kw("CONSTRUCT") {
            self.parse_construct(base_iri)
        } else if self.peek_kw("ASK") {
            self.parse_ask(base_iri)
        } else if self.peek_kw("DESCRIBE") {
            self.parse_describe(base_iri)
        } else {
            Err(ParseError::syntax(
                "expected SELECT, CONSTRUCT, ASK or DESCRIBE",
                self.span(),
            ))
        }
    }

    fn parse_prologue(&mut self) -> Result<()> {
        loop {
            if self.eat_kw("BASE") {
                let iri = self.expect_iriref()?;
                self.base = Some(iri);
            } else if self.eat_kw("PREFIX") {
                let (prefix, _) = self.expect_pname_ns()?;
                let iri = self.expect_iriref()?;
                self.prefixes.insert(prefix, iri);
            } else if self.eat_kw("VERSION") {
                // SPARQL 1.2 version declaration: `VERSION <string>`. Recorded and
                // otherwise inert — it declares the query's target spec version.
                match self.bump() {
                    Some(Token::StringLit(_)) => {}
                    other => {
                        return Err(ParseError::syntax(
                            format!("expected a version string after VERSION, found {other:?}"),
                            self.span(),
                        ))
                    }
                }
            } else {
                break;
            }
        }
        Ok(())
    }

    fn expect_iriref(&mut self) -> Result<String> {
        match self.bump() {
            Some(Token::Iri(s)) => self.resolve_iri(&s),
            other => Err(ParseError::syntax(
                format!("expected IRIREF, found {other:?}"),
                self.span(),
            )),
        }
    }

    /// Expect a `prefix:` namespace token (PNAME_NS), i.e. an empty local part.
    fn expect_pname_ns(&mut self) -> Result<(String, String)> {
        match self.bump() {
            Some(Token::PrefixedName(p, l)) if l.is_empty() => Ok((p, l)),
            // `PREFIX ex:local <...>` is malformed — a prologue prefix must be a
            // bare PNAME_NS (`ex:`). Reject rather than silently dropping `local`.
            Some(Token::PrefixedName(p, l)) => Err(ParseError::syntax(
                format!("PREFIX declaration must be a bare namespace, found {p}:{l}"),
                self.span(),
            )),
            other => Err(ParseError::syntax(
                format!("expected prefix declaration, found {other:?}"),
                self.span(),
            )),
        }
    }

    /// Resolve a lexical IRIREF against the in-scope `BASE` (relative refs only).
    /// Propagates a typed [`ParseError::Iri`] when the base or the resolution is
    /// malformed instead of silently falling back to the raw string.
    fn resolve_iri(&self, s: &str) -> Result<String> {
        match &self.base {
            Some(base) if !is_absolute_iri(s) => {
                let base_iri = purrdf_iri::parse(base).map_err(|e| ParseError::Iri {
                    lexical: base.clone(),
                    reason: e.to_string(),
                })?;
                let resolved =
                    purrdf_iri::Iri::resolve(&base_iri, s).map_err(|e| ParseError::Iri {
                        lexical: s.to_owned(),
                        reason: e.to_string(),
                    })?;
                Ok(resolved.as_str().to_owned())
            }
            _ => Ok(s.to_owned()),
        }
    }

    fn resolve_prefixed(&self, prefix: &str, local: &str) -> Result<NamedNode> {
        match self.prefixes.get(prefix) {
            Some(ns) => NamedNode::new(format!("{ns}{local}")),
            None => Err(ParseError::syntax(
                format!("undeclared prefix {prefix:?}"),
                self.span(),
            )),
        }
    }

    // ── query forms ──────────────────────────────────────────────────────────

    fn parse_select(&mut self, base_iri: Option<NamedNode>) -> Result<Query> {
        self.expect_kw("SELECT")?;
        let distinct = self.eat_kw("DISTINCT");
        let reduced = !distinct && self.eat_kw("REDUCED");

        // Projection: `*` or a list of Var / (Expr AS Var).
        let mut star = false;
        let mut projected: Vec<Variable> = Vec::new();
        let mut select_exprs: Vec<(Variable, Expression)> = Vec::new();
        let mut aggregates: Vec<(Variable, AggregateExpression)> = Vec::new();
        if self.eat(&Token::Star) {
            star = true;
        } else {
            loop {
                if let Some(Token::Variable(_)) = self.peek() {
                    projected.push(self.expect_var()?);
                } else if self.at(&Token::LParen) {
                    self.expect(&Token::LParen)?;
                    let expr = self.parse_expression_lifting_aggs(&mut aggregates)?;
                    self.expect_kw("AS")?;
                    let var = self.expect_var()?;
                    self.expect(&Token::RParen)?;
                    projected.push(var.clone());
                    select_exprs.push((var, expr));
                } else {
                    break;
                }
            }
            if projected.is_empty() {
                return Err(ParseError::syntax("empty SELECT projection", self.span()));
            }
        }

        // Dataset clause (FROM / FROM NAMED), §13.2.
        let dataset = self.parse_dataset_clauses()?;

        self.eat_kw("WHERE");
        let where_pat = self.parse_group_graph_pattern()?;

        let modifiers = self.parse_solution_modifiers(&mut aggregates)?;

        // §19.8: each SELECT `(expr AS ?v)` target must be fresh — not already in
        // scope. When the query aggregates (an explicit `GROUP BY` or any
        // aggregate ⇒ implicit single group), only the grouping keys and
        // group-expression targets stay visible to the projection; the raw WHERE
        // pattern variables are projected away by grouping, so re-binding one via
        // `(expr AS ?v)` is legal (e.g. `SELECT (123 AS ?z) … GROUP BY ?s`).
        if !select_exprs.is_empty() {
            let aggregating = !modifiers.group_by.is_empty()
                || !modifiers.group_extends.is_empty()
                || !aggregates.is_empty();
            let mut in_scope: std::collections::HashSet<Variable> = if aggregating {
                modifiers
                    .group_by
                    .iter()
                    .cloned()
                    .chain(modifiers.group_extends.iter().map(|(v, _)| v.clone()))
                    .collect()
            } else {
                visible_variables(&where_pat).into_iter().collect()
            };
            for (var, _) in &select_exprs {
                if !in_scope.insert(var.clone()) {
                    return Err(ParseError::syntax(
                        format!(
                            "SELECT expression target ?{} is already in scope",
                            var.as_str()
                        ),
                        self.span(),
                    ));
                }
            }
        }

        // §11.1 grammar note: the `SELECT *` shorthand is illegal in an aggregate
        // query — an explicit `GROUP BY` (keys or expression conditions) or any
        // aggregate makes the projection ill-defined, so it is a hard syntax
        // error (vendored W3C `syntax-query` `syn-bad-01`: `SELECT * … GROUP BY`).
        if star
            && (!modifiers.group_by.is_empty()
                || !modifiers.group_extends.is_empty()
                || !aggregates.is_empty())
        {
            return Err(ParseError::syntax(
                "SELECT * is not allowed in an aggregate query (GROUP BY or aggregation)",
                self.span(),
            ));
        }

        // §18.2.4.1 grouping constraint: when the query aggregates (an explicit
        // `GROUP BY`, or one or more aggregates in the SELECT clause ⇒ an implicit
        // single group), every BARE projected variable — one named directly as a
        // `Var`, not the fresh target of a `(expr AS ?v)` — must be one of the
        // `GROUP BY` keys (explicit or the synthetic var of an expression-valued
        // GROUP BY condition). A bare projected variable that is neither a group
        // key nor confined to an aggregate is a hard query error, not a silently
        // wrong answer (this is the vendored W3C `grouping/group06`/`group07`
        // negative-syntax cases: `SELECT ?s ?v { ... } GROUP BY ?s` projects the
        // ungrouped, non-aggregated `?v`). `SELECT *` is exempted here: its
        // projection is derived structurally from the (already-grouped) algebra
        // node below, so it can only ever expose grouped/aggregate variables.
        if !star {
            let is_aggregating = !modifiers.group_by.is_empty() || !aggregates.is_empty();
            if is_aggregating {
                let as_targets: std::collections::HashSet<&Variable> =
                    select_exprs.iter().map(|(v, _)| v).collect();
                let group_vars: std::collections::HashSet<&Variable> =
                    modifiers.group_by.iter().collect();
                for var in &projected {
                    if !as_targets.contains(var) && !group_vars.contains(var) {
                        return Err(ParseError::syntax(
                            format!(
                                "SELECT projects ?{}, which is neither a GROUP BY key nor \
                                 confined to an aggregate",
                                var.as_str()
                            ),
                            self.span(),
                        ));
                    }
                }
            }
        }

        // Trailing `ValuesClause` (§18.2.4.3): a `VALUES DataBlock` after the
        // solution modifiers — valid on both a top-level query and a `SubSelect`.
        // It is joined with the WHERE group graph pattern *before* grouping and
        // projection, so the inline data is visible to aggregation and `SELECT *`.
        let where_pat = if self.peek_kw("VALUES") {
            let values = self.parse_inline_data()?;
            GraphPattern::Join {
                left: Box::new(where_pat),
                right: Box::new(values),
            }
        } else {
            where_pat
        };

        // Build the algebra (§18.2.4 ordering).
        let mut p = where_pat;
        // Expression-valued GROUP BY conditions bind their synthetic/explicit
        // grouping variable BELOW the Group, so `eval_group` sees a ready column.
        for (var, expr) in modifiers.group_extends {
            p = GraphPattern::Extend {
                inner: Box::new(p),
                variable: var,
                expression: expr,
            };
        }
        let has_group = !modifiers.group_by.is_empty() || !aggregates.is_empty();
        if has_group {
            p = GraphPattern::Group {
                inner: Box::new(p),
                variables: modifiers.group_by.clone(),
                aggregates,
            };
        }
        for expr in modifiers.having {
            p = GraphPattern::Filter {
                expr,
                inner: Box::new(p),
            };
        }
        for (var, expr) in select_exprs {
            p = GraphPattern::Extend {
                inner: Box::new(p),
                variable: var,
                expression: expr,
            };
        }
        if !modifiers.order_by.is_empty() {
            p = GraphPattern::OrderBy {
                inner: Box::new(p),
                expression: modifiers.order_by,
            };
        }
        let variables = if star {
            visible_variables(&p)
        } else {
            projected
        };
        p = GraphPattern::Project {
            inner: Box::new(p),
            variables,
        };
        if distinct {
            p = GraphPattern::Distinct { inner: Box::new(p) };
        } else if reduced {
            p = GraphPattern::Reduced { inner: Box::new(p) };
        }
        if modifiers.offset.is_some() || modifiers.limit.is_some() {
            p = GraphPattern::Slice {
                inner: Box::new(p),
                start: modifiers.offset.unwrap_or(0),
                length: modifiers.limit,
            };
        }
        Ok(Query::Select {
            pattern: p,
            dataset,
            base_iri,
        })
    }

    fn parse_construct(&mut self, base_iri: Option<NamedNode>) -> Result<Query> {
        self.expect_kw("CONSTRUCT")?;
        // Short form (§16.2.1): `CONSTRUCT DatasetClause* WHERE { TriplesTemplate }`
        // with no explicit template — the template *is* the WHERE triples block.
        if !self.at(&Token::LBrace) {
            let dataset = self.parse_dataset_clauses()?;
            self.expect_kw("WHERE")?;
            self.expect(&Token::LBrace)?;
            // The short form's template *is* the WHERE triples block (§16.2.1) — but an
            // RDF 1.2 reifier/annotation (`~ id`, `{| … |}`) inside that block desugars
            // to a FRESH synthetic reifier blank at parse time (`parse_triple_annotations`
            // / `parse_triple_node`). A `.clone()` of the already-desugared triples would
            // give the WHERE match and the CONSTRUCT template the SAME reifier blank
            // identity, which conflates two independent things: the WHERE-side reifier is
            // a non-distinguished (matched-but-discarded) existential witness, while the
            // template-side reifier is minted FRESH per solution row regardless of what it
            // matched (the general CONSTRUCT template blank-node rule). Reparsing the SAME
            // token span a second time — rewinding `self.pos`, so `fresh_anon()` mints a
            // NEW counter value — gives the WHERE copy its OWN, independent synthetic
            // reifier blanks, decoupled from the template's (W3C `eval-triple-terms`
            // `construct-5`/`expr-1`: a query-supplied `~`/`{| |}` name IS a real token, so
            // re-tokenizing reproduces the SAME label there — only the auto-generated
            // synthetic blanks differ between the two parses).
            let mark = self.pos;
            let template = self.parse_construct_template()?;
            self.pos = mark;
            let where_patterns = self.parse_construct_template()?;
            self.expect(&Token::RBrace)?;
            let where_pat = GraphPattern::Bgp {
                patterns: where_patterns,
            };
            let mut aggregates = Vec::new();
            let modifiers = self.parse_solution_modifiers(&mut aggregates)?;
            if !aggregates.is_empty()
                || !modifiers.group_by.is_empty()
                || !modifiers.having.is_empty()
            {
                return Err(ParseError::unsupported("aggregation/HAVING in CONSTRUCT"));
            }
            let mut p = where_pat;
            if !modifiers.order_by.is_empty() {
                p = GraphPattern::OrderBy {
                    inner: Box::new(p),
                    expression: modifiers.order_by,
                };
            }
            if modifiers.offset.is_some() || modifiers.limit.is_some() {
                p = GraphPattern::Slice {
                    inner: Box::new(p),
                    start: modifiers.offset.unwrap_or(0),
                    length: modifiers.limit,
                };
            }
            return Ok(Query::Construct {
                template,
                pattern: p,
                dataset,
                base_iri,
            });
        }
        // Long form: CONSTRUCT { template } WHERE { ... }
        self.expect(&Token::LBrace)?;
        let template = self.parse_construct_template()?;
        self.expect(&Token::RBrace)?;
        let dataset = self.parse_dataset_clauses()?;
        self.eat_kw("WHERE");
        let where_pat = self.parse_group_graph_pattern()?;
        let mut aggregates = Vec::new();
        let modifiers = self.parse_solution_modifiers(&mut aggregates)?;
        if !aggregates.is_empty() || !modifiers.group_by.is_empty() || !modifiers.having.is_empty()
        {
            return Err(ParseError::unsupported("aggregation/HAVING in CONSTRUCT"));
        }
        let mut p = where_pat;
        if !modifiers.order_by.is_empty() {
            p = GraphPattern::OrderBy {
                inner: Box::new(p),
                expression: modifiers.order_by,
            };
        }
        if modifiers.offset.is_some() || modifiers.limit.is_some() {
            p = GraphPattern::Slice {
                inner: Box::new(p),
                start: modifiers.offset.unwrap_or(0),
                length: modifiers.limit,
            };
        }
        Ok(Query::Construct {
            template,
            pattern: p,
            dataset,
            base_iri,
        })
    }

    fn parse_ask(&mut self, base_iri: Option<NamedNode>) -> Result<Query> {
        self.expect_kw("ASK")?;
        let dataset = self.parse_dataset_clauses()?;
        self.eat_kw("WHERE");
        let pattern = self.parse_group_graph_pattern()?;
        let mut aggregates = Vec::new();
        let modifiers = self.parse_solution_modifiers(&mut aggregates)?;
        // ASK ignores solution modifiers semantically; rather than silently
        // dropping a parsed one, hard-fail (no-optionality / no silent discard).
        if !modifiers.is_empty() || !aggregates.is_empty() {
            return Err(ParseError::unsupported("solution modifiers on ASK"));
        }
        Ok(Query::Ask {
            pattern,
            dataset,
            base_iri,
        })
    }

    fn parse_describe(&mut self, base_iri: Option<NamedNode>) -> Result<Query> {
        self.expect_kw("DESCRIBE")?;
        let mut targets = Vec::new();
        if self.eat(&Token::Star) {
            // DESCRIBE * — no explicit targets.
        } else {
            loop {
                match self.peek() {
                    Some(Token::Variable(_)) => {
                        targets.push(NamedNodePattern::Variable(self.expect_var()?));
                    }
                    Some(Token::Iri(_) | Token::PrefixedName(_, _)) => {
                        targets.push(NamedNodePattern::NamedNode(self.expect_iri_node()?));
                    }
                    _ => break,
                }
            }
            if targets.is_empty() {
                return Err(ParseError::syntax("DESCRIBE needs a target", self.span()));
            }
        }
        let dataset = self.parse_dataset_clauses()?;
        let pattern = if self.eat_kw("WHERE") || self.at(&Token::LBrace) {
            self.parse_group_graph_pattern()?
        } else {
            GraphPattern::Bgp { patterns: vec![] }
        };
        let mut aggregates = Vec::new();
        let modifiers = self.parse_solution_modifiers(&mut aggregates)?;
        if !modifiers.is_empty() || !aggregates.is_empty() {
            return Err(ParseError::unsupported("solution modifiers on DESCRIBE"));
        }
        Ok(Query::Describe {
            pattern,
            targets,
            dataset,
            base_iri,
        })
    }

    /// Zero or more `FROM [NAMED] <iri>` dataset clauses (§13.2). `FROM <iri>` adds to
    /// the active default graph; `FROM NAMED <iri>` adds an addressable named graph.
    fn parse_dataset_clauses(&mut self) -> Result<QueryDataset> {
        let mut default = Vec::new();
        let mut named = Vec::new();
        while self.eat_kw("FROM") {
            if self.eat_kw("NAMED") {
                named.push(self.expect_iri_node()?);
            } else {
                default.push(self.expect_iri_node()?);
            }
        }
        Ok(QueryDataset { default, named })
    }

    fn parse_construct_template(&mut self) -> Result<Vec<TriplePattern>> {
        // A `TriplesTemplate` (§16.2 grammar) — the same triples-block grammar as
        // a group's BGP, so RDF 1.2 reifiers/annotations and triple terms desugar
        // identically. Property paths are *not* valid in a template.
        if self.at(&Token::RBrace) {
            return Ok(Vec::new());
        }
        match self.parse_triples_block()? {
            GraphPattern::Bgp { patterns } => Ok(patterns),
            _ => Err(ParseError::syntax(
                "property paths are not allowed in a CONSTRUCT template",
                self.span(),
            )),
        }
    }

    // ── SPARQL 1.1 Update (§3 + grammar §19) ─────────────────────────────────

    /// Parse a full Update request: prologue + a `;`-separated sequence of
    /// graph-update operations. A request with only a prologue (no operations)
    /// is valid, and a trailing `;` is allowed.
    fn parse_update(&mut self) -> Result<Update> {
        self.parse_prologue()?;
        let base_iri = self.base.clone().map(NamedNode::new).transpose()?;

        let mut operations = Vec::new();
        // §4.1.1 + grammar note: a blank node label in `INSERT DATA` ground data
        // is scoped to that one operation — reusing it in another `INSERT DATA`
        // of the same request denotes a fresh vs. same blank ambiguity and is a
        // hard syntax error (vendored W3C `syntax-update-1` `syntax-update-54`).
        // This applies ONLY to ground `INSERT DATA` quads: blank nodes in an
        // `INSERT { … } WHERE` template are minted fresh per solution, so the
        // same template label legitimately recurs across operations (vendored
        // W3C `basic-update` `insert-where-same-bnode`). `DELETE DATA` / DELETE
        // templates are blank-free by invariant, and anonymous blanks carry
        // process-unique ids, so only author-written `_:label`s can collide.
        let mut prior_bnode_labels: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        // Reused across iterations to avoid reallocating the set each loop.
        let mut this_op_labels: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        loop {
            if self.pos >= self.tokens.len() {
                break;
            }
            let op = self.parse_update_operation()?;
            this_op_labels.clear();
            if let GraphUpdateOperation::InsertData { data } = &op {
                collect_quad_bnode_labels(data, &mut this_op_labels);
            }
            for label in &this_op_labels {
                if prior_bnode_labels.contains(label) {
                    return Err(ParseError::syntax(
                        format!("blank node label _:{label} is reused across update operations"),
                        self.span(),
                    ));
                }
            }
            prior_bnode_labels.extend(this_op_labels.drain());
            operations.push(op);
            // An operation separator. Without it, the request is done (a stray
            // trailing token is caught by `expect_eof` at the public entry).
            if !self.eat(&Token::Semicolon) {
                break;
            }
            // A trailing `;` may be followed by more prologue (BASE/PREFIX) and
            // another operation, or by end-of-input.
            self.parse_prologue()?;
        }
        Ok(Update {
            operations,
            base_iri,
        })
    }

    fn parse_update_operation(&mut self) -> Result<GraphUpdateOperation> {
        if self.peek_kw("INSERT") {
            self.parse_insert()
        } else if self.peek_kw("DELETE") {
            self.parse_delete()
        } else if self.peek_kw("WITH") {
            self.parse_with_modify()
        } else if self.peek_kw("LOAD") {
            self.parse_load()
        } else if self.peek_kw("CLEAR") {
            self.parse_clear_or_drop(true)
        } else if self.peek_kw("DROP") {
            self.parse_clear_or_drop(false)
        } else if self.peek_kw("CREATE") {
            self.parse_create()
        } else if self.peek_kw("ADD") || self.peek_kw("MOVE") || self.peek_kw("COPY") {
            self.parse_add_move_copy()
        } else {
            Err(ParseError::syntax(
                format!(
                    "expected an update operation keyword, found {:?}",
                    self.peek()
                ),
                self.span(),
            ))
        }
    }

    /// `INSERT DATA { QuadData }` or `INSERT { QuadPattern } [USING ...] WHERE { ... }`.
    fn parse_insert(&mut self) -> Result<GraphUpdateOperation> {
        self.expect_kw("INSERT")?;
        if self.eat_kw("DATA") {
            let data = self.parse_quad_data()?;
            // INSERT DATA: no variables anywhere; blank nodes ARE allowed (§3.1.1).
            self.enforce_data_invariants(&data, false)?;
            return Ok(GraphUpdateOperation::InsertData { data });
        }
        // INSERT { template } [USING ...] WHERE { ... } — an insert-only modify.
        let insert = self.parse_quad_pattern_block(false)?;
        let using = self.parse_using_clauses()?;
        self.expect_kw("WHERE")?;
        let pattern = self.parse_group_graph_pattern()?;
        Ok(GraphUpdateOperation::DeleteInsert {
            delete: Vec::new(),
            insert,
            with: None,
            using,
            pattern: Box::new(pattern),
        })
    }

    /// `DELETE DATA { QuadData }`, `DELETE WHERE { QuadPattern }`, or
    /// `DELETE { template } [INSERT { ... }] [USING ...] WHERE { ... }`.
    fn parse_delete(&mut self) -> Result<GraphUpdateOperation> {
        self.expect_kw("DELETE")?;
        if self.eat_kw("DATA") {
            let data = self.parse_quad_data()?;
            // DELETE DATA: no variables AND no blank nodes (§3.1.2).
            self.enforce_data_invariants(&data, true)?;
            return Ok(GraphUpdateOperation::DeleteData { data });
        }
        if self.eat_kw("WHERE") {
            // DELETE WHERE { QuadPattern } — the template IS the where pattern.
            let mark = self.pos;
            let delete = self.parse_quad_pattern_block(true)?;
            // Re-parse the same braces as a group graph pattern for the WHERE.
            self.pos = mark;
            let pattern = self.parse_group_graph_pattern()?;
            return Ok(GraphUpdateOperation::DeleteInsert {
                delete,
                insert: Vec::new(),
                with: None,
                using: Vec::new(),
                pattern: Box::new(pattern),
            });
        }
        // DELETE { template } [INSERT { ... }] [USING ...] WHERE { ... }.
        let delete = self.parse_quad_pattern_block(true)?;
        let insert = if self.eat_kw("INSERT") {
            self.parse_quad_pattern_block(false)?
        } else {
            Vec::new()
        };
        let using = self.parse_using_clauses()?;
        self.expect_kw("WHERE")?;
        let pattern = self.parse_group_graph_pattern()?;
        Ok(GraphUpdateOperation::DeleteInsert {
            delete,
            insert,
            with: None,
            using,
            pattern: Box::new(pattern),
        })
    }

    /// `WITH <iri> (DELETE { ... } | INSERT { ... }) [INSERT { ... }] WHERE { ... }`.
    fn parse_with_modify(&mut self) -> Result<GraphUpdateOperation> {
        self.expect_kw("WITH")?;
        let with = Some(self.expect_iri_node()?);
        let mut delete = Vec::new();
        let mut insert = Vec::new();
        if self.eat_kw("DELETE") {
            delete = self.parse_quad_pattern_block(true)?;
            if self.eat_kw("INSERT") {
                insert = self.parse_quad_pattern_block(false)?;
            }
        } else if self.eat_kw("INSERT") {
            insert = self.parse_quad_pattern_block(false)?;
        } else {
            return Err(ParseError::syntax(
                "WITH must be followed by DELETE and/or INSERT",
                self.span(),
            ));
        }
        let using = self.parse_using_clauses()?;
        self.expect_kw("WHERE")?;
        let pattern = self.parse_group_graph_pattern()?;
        Ok(GraphUpdateOperation::DeleteInsert {
            delete,
            insert,
            with,
            using,
            pattern: Box::new(pattern),
        })
    }

    /// Zero or more `USING [NAMED] <iri>` clauses (§3.1.3). The `NAMED` modifier is
    /// preserved: `USING <iri>` folds into the active default graph, `USING NAMED
    /// <iri>` becomes an addressable named graph for the `WHERE`.
    fn parse_using_clauses(&mut self) -> Result<Vec<UsingClause>> {
        let mut using = Vec::new();
        while self.eat_kw("USING") {
            if self.eat_kw("NAMED") {
                using.push(UsingClause::Named(self.expect_iri_node()?));
            } else {
                using.push(UsingClause::Default(self.expect_iri_node()?));
            }
        }
        Ok(using)
    }

    /// `LOAD [SILENT] <iri> [INTO GRAPH <iri>]`.
    fn parse_load(&mut self) -> Result<GraphUpdateOperation> {
        self.expect_kw("LOAD")?;
        let silent = self.eat_kw("SILENT");
        let source = self.expect_iri_node()?;
        let destination = if self.eat_kw("INTO") {
            self.expect_kw("GRAPH")?;
            GraphTarget::Named(self.expect_iri_node()?)
        } else {
            GraphTarget::Default
        };
        Ok(GraphUpdateOperation::Load {
            silent,
            source,
            destination,
        })
    }

    /// `CLEAR [SILENT] <GraphRefAll>` / `DROP [SILENT] <GraphRefAll>`.
    fn parse_clear_or_drop(&mut self, is_clear: bool) -> Result<GraphUpdateOperation> {
        self.expect_kw(if is_clear { "CLEAR" } else { "DROP" })?;
        let silent = self.eat_kw("SILENT");
        let target = self.parse_graph_ref_all()?;
        Ok(if is_clear {
            GraphUpdateOperation::Clear { silent, target }
        } else {
            GraphUpdateOperation::Drop { silent, target }
        })
    }

    /// `CREATE [SILENT] GRAPH <iri>`.
    fn parse_create(&mut self) -> Result<GraphUpdateOperation> {
        self.expect_kw("CREATE")?;
        let silent = self.eat_kw("SILENT");
        self.expect_kw("GRAPH")?;
        let graph = self.expect_iri_node()?;
        Ok(GraphUpdateOperation::Create { silent, graph })
    }

    /// `ADD|MOVE|COPY [SILENT] <GraphOrDefault> TO <GraphOrDefault>`.
    fn parse_add_move_copy(&mut self) -> Result<GraphUpdateOperation> {
        let which = if self.eat_kw("ADD") {
            0u8
        } else if self.eat_kw("MOVE") {
            1
        } else {
            self.expect_kw("COPY")?;
            2
        };
        let silent = self.eat_kw("SILENT");
        let source = self.parse_graph_or_default()?;
        self.expect_kw("TO")?;
        let destination = self.parse_graph_or_default()?;
        Ok(match which {
            0 => GraphUpdateOperation::Add {
                silent,
                source,
                destination,
            },
            1 => GraphUpdateOperation::Move {
                silent,
                source,
                destination,
            },
            _ => GraphUpdateOperation::Copy {
                silent,
                source,
                destination,
            },
        })
    }

    /// `GraphRefAll`: `DEFAULT | NAMED | ALL | GRAPH <iri>`.
    fn parse_graph_ref_all(&mut self) -> Result<GraphTarget> {
        if self.eat_kw("DEFAULT") {
            Ok(GraphTarget::Default)
        } else if self.eat_kw("NAMED") {
            Ok(GraphTarget::NamedGraphs)
        } else if self.eat_kw("ALL") {
            Ok(GraphTarget::All)
        } else if self.eat_kw("GRAPH") {
            Ok(GraphTarget::Named(self.expect_iri_node()?))
        } else {
            Err(ParseError::syntax(
                "expected DEFAULT, NAMED, ALL or GRAPH <iri>",
                self.span(),
            ))
        }
    }

    /// `GraphOrDefault`: `DEFAULT | [GRAPH] <iri>` (no NAMED/ALL here).
    fn parse_graph_or_default(&mut self) -> Result<GraphTarget> {
        if self.eat_kw("DEFAULT") {
            Ok(GraphTarget::Default)
        } else {
            self.eat_kw("GRAPH");
            Ok(GraphTarget::Named(self.expect_iri_node()?))
        }
    }

    /// Parse a `{ ... }` quad block into [`QuadPattern`]s. Triple templates plus
    /// optional nested `GRAPH (<iri>|?var) { triples }` groups. When `is_delete`
    /// is set, any blank node in the templates is a hard error (DELETE templates
    /// disallow blanks per §3.1.3).
    fn parse_quad_pattern_block(&mut self, is_delete: bool) -> Result<Vec<QuadPattern>> {
        let mut quads = Vec::new();
        self.expect(&Token::LBrace)?;
        loop {
            if self.at(&Token::RBrace) {
                break;
            } else if self.eat_kw("GRAPH") {
                let graph = self.parse_var_or_iri_name()?;
                self.collect_quad_group(Some(&graph), is_delete, &mut quads)?;
            } else if self.eat(&Token::Dot) {
                // statement separator between triple blocks
            } else {
                let mut triples = Vec::new();
                self.parse_template_triple(&mut triples)?;
                self.eat(&Token::Dot);
                for triple in triples {
                    if is_delete {
                        reject_blank_in_triple_pattern(&triple, self.span())?;
                    }
                    quads.push(QuadPattern {
                        triple,
                        graph: None,
                    });
                }
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(quads)
    }

    /// Parse one subject + predicate-object list of an update template
    /// (`TriplesTemplate`), emitting the (RDF 1.2-desugared) triples into
    /// `triples`. Mirrors the subject dispatch of [`parse_triples_block`] so
    /// reifiers, annotations, triple terms, collections and blank-node property
    /// lists all desugar identically; property paths are not admissible here.
    fn parse_template_triple(&mut self, triples: &mut Vec<TriplePattern>) -> Result<()> {
        let mut paths = Vec::new();
        let (subject, standalone_ok) = if self.at(&Token::LBracket) {
            (
                self.parse_blank_node_property_list(triples, &mut paths)?,
                true,
            )
        } else if self.at(&Token::LParen) {
            (self.parse_collection(triples, &mut paths)?, false)
        } else if self.at(&Token::TripleOpen) {
            let node = self.parse_triple_node(triples, &mut paths)?;
            let standalone = !matches!(node, TermPattern::Triple(_));
            (node, standalone)
        } else {
            (self.parse_term_pattern()?, false)
        };
        let standalone = standalone_ok
            && (self.at(&Token::Dot) || self.at(&Token::RBrace) || self.at(&Token::LBrace));
        if !standalone {
            self.parse_predicate_object_list(&subject, triples, &mut paths)?;
        }
        if !paths.is_empty() {
            return Err(ParseError::syntax(
                "property paths are not allowed in an update template",
                self.span(),
            ));
        }
        Ok(())
    }

    /// Parse a nested `GRAPH g { triples }` group, scoping each parsed triple to
    /// `graph` and pushing the resulting quad patterns into `quads`.
    fn collect_quad_group(
        &mut self,
        graph: Option<&NamedNodePattern>,
        is_delete: bool,
        quads: &mut Vec<QuadPattern>,
    ) -> Result<()> {
        self.expect(&Token::LBrace)?;
        let mut triples = Vec::new();
        while !self.at(&Token::RBrace) {
            self.parse_template_triple(&mut triples)?;
            if !self.eat(&Token::Dot) {
                break;
            }
        }
        self.expect(&Token::RBrace)?;
        for triple in triples {
            if is_delete {
                reject_blank_in_triple_pattern(&triple, self.span())?;
            }
            quads.push(QuadPattern {
                triple,
                graph: graph.cloned(),
            });
        }
        Ok(())
    }

    /// Parse a `{ QuadData }` block as quad *patterns* (the same surface as
    /// `parse_quad_pattern_block`). The DATA invariants (no variables; and, for
    /// DELETE DATA, no blank nodes) are enforced separately by
    /// [`enforce_data_invariants`](Self::enforce_data_invariants) so INSERT DATA
    /// can keep its (allowed) blank nodes.
    fn parse_quad_data(&mut self) -> Result<Vec<QuadPattern>> {
        self.parse_quad_pattern_block(false)
    }

    /// Enforce the `INSERT DATA` / `DELETE DATA` invariants by walking the parsed
    /// [`QuadPattern`]s: NO variables anywhere (subject/predicate/object/graph). For
    /// DELETE DATA (`reject_blank`), NO blank nodes either (§3.1.2). INSERT DATA
    /// permits blank nodes (§3.1.1: minted fresh per request). Any violation is a
    /// hard [`ParseError::syntax`].
    fn enforce_data_invariants(&self, quads: &[QuadPattern], reject_blank: bool) -> Result<()> {
        for q in quads {
            if let Some(NamedNodePattern::Variable(_)) = &q.graph {
                return Err(ParseError::syntax(
                    "variable graph in INSERT/DELETE DATA is not allowed",
                    self.span(),
                ));
            }
            self.check_data_triple(&q.triple, reject_blank)?;
        }
        Ok(())
    }

    /// Walk one DATA triple pattern, rejecting variables (always) and blank nodes
    /// (when `reject_blank`). Descends into RDF 1.2 quoted triples.
    fn check_data_triple(&self, t: &TriplePattern, reject_blank: bool) -> Result<()> {
        if let NamedNodePattern::Variable(_) = &t.predicate {
            return Err(ParseError::syntax(
                "variable predicate in INSERT/DELETE DATA is not allowed",
                self.span(),
            ));
        }
        self.check_data_term(&t.subject, reject_blank)?;
        self.check_data_term(&t.object, reject_blank)
    }

    /// Walk one DATA term pattern, rejecting variables (always) and blank nodes
    /// (when `reject_blank`). Descends into RDF 1.2 quoted triples.
    fn check_data_term(&self, t: &TermPattern, reject_blank: bool) -> Result<()> {
        match t {
            TermPattern::NamedNode(_) | TermPattern::Literal(_) => Ok(()),
            TermPattern::Triple(tp) => self.check_data_triple(tp, reject_blank),
            TermPattern::Variable(_) => Err(ParseError::syntax(
                "variable in INSERT/DELETE DATA is not allowed",
                self.span(),
            )),
            TermPattern::BlankNode(_) => {
                if reject_blank {
                    Err(ParseError::syntax(
                        "blank node in DELETE DATA is not allowed",
                        self.span(),
                    ))
                } else {
                    // INSERT DATA blanks are allowed (minted fresh per request).
                    Ok(())
                }
            }
        }
    }

    // ── group graph pattern → algebra (§18.2.2) ──────────────────────────────

    fn parse_group_graph_pattern(&mut self) -> Result<GraphPattern> {
        self.expect(&Token::LBrace)?;

        // A sub-SELECT group: `{ SELECT ... }`.
        if self.peek_kw("SELECT") {
            let sub = self.parse_select(None)?;
            self.expect(&Token::RBrace)?;
            return match sub {
                Query::Select { pattern, .. } => Ok(pattern),
                _ => unreachable!("parse_select yields Query::Select"),
            };
        }

        let mut g = GraphPattern::Bgp { patterns: vec![] };
        let mut filters: Vec<Expression> = Vec::new();

        loop {
            if self.at(&Token::RBrace) {
                break;
            } else if self.at(&Token::LBrace) {
                let mut node = self.parse_group_graph_pattern()?;
                while self.eat_kw("UNION") {
                    let right = self.parse_group_graph_pattern()?;
                    node = GraphPattern::Union {
                        left: Box::new(node),
                        right: Box::new(right),
                    };
                }
                g = join(g, node);
            } else if self.eat_kw("OPTIONAL") {
                let inner = self.parse_group_graph_pattern()?;
                let (right, expression) = split_trailing_filter(inner);
                g = GraphPattern::LeftJoin {
                    left: Box::new(g),
                    right: Box::new(right),
                    expression,
                };
            } else if self.eat_kw("MINUS") {
                let right = self.parse_group_graph_pattern()?;
                g = GraphPattern::Minus {
                    left: Box::new(g),
                    right: Box::new(right),
                };
            } else if self.eat_kw("GRAPH") {
                let name = self.parse_var_or_iri_name()?;
                let inner = self.parse_group_graph_pattern()?;
                g = join(
                    g,
                    GraphPattern::Graph {
                        name,
                        inner: Box::new(inner),
                    },
                );
            } else if self.eat_kw("SERVICE") {
                let silent = self.eat_kw("SILENT");
                let name = self.parse_var_or_iri_name()?;
                let inner = self.parse_group_graph_pattern()?;
                let is_var_endpoint = matches!(name, NamedNodePattern::Variable(_));
                let service = GraphPattern::Service {
                    name,
                    inner: Box::new(inner),
                    silent,
                };
                // A variable endpoint (`SERVICE ?g`) is correlated with the
                // enclosing pattern — it must bind the endpoint from the
                // surrounding solution before federating — so it becomes a
                // LATERAL join. A fixed-IRI endpoint stays a plain join.
                g = if is_var_endpoint {
                    GraphPattern::Lateral {
                        left: Box::new(g),
                        right: Box::new(service),
                    }
                } else {
                    join(g, service)
                };
            } else if self.eat_kw("FILTER") {
                filters.push(self.parse_constraint()?);
            } else if self.eat_kw("BIND") {
                self.expect(&Token::LParen)?;
                let expression = self.parse_expression()?;
                self.expect_kw("AS")?;
                let variable = self.expect_var()?;
                self.expect(&Token::RParen)?;
                // §19.6: the variable introduced by BIND must not already be
                // in-scope in the group graph pattern up to this point — a
                // re-binding is a hard syntax error, not a silent shadow
                // (vendored W3C `syntax-query` `syntax-BINDscope6/7/8`).
                if visible_variables(&g).contains(&variable) {
                    return Err(ParseError::syntax(
                        format!(
                            "BIND target ?{} is already in scope in the group graph pattern",
                            variable.as_str()
                        ),
                        self.span(),
                    ));
                }
                g = GraphPattern::Extend {
                    inner: Box::new(g),
                    variable,
                    expression,
                };
            } else if self.peek_kw("VALUES") {
                let values = self.parse_inline_data()?;
                g = join(g, values);
            } else if self.eat(&Token::Dot) {
                // statement separator between blocks
            } else {
                // A triples block (BGP / path patterns).
                let block = self.parse_triples_block()?;
                g = join(g, block);
            }
        }

        self.expect(&Token::RBrace)?;
        for expr in filters {
            g = GraphPattern::Filter {
                expr,
                inner: Box::new(g),
            };
        }
        Ok(g)
    }

    /// Parse a run of triples (subject + predicate-object lists) into a BGP and
    /// any complex property-path `Path` nodes, joined together.
    fn parse_triples_block(&mut self) -> Result<GraphPattern> {
        let mut triples: Vec<TriplePattern> = Vec::new();
        let mut paths: Vec<GraphPattern> = Vec::new();
        loop {
            // The subject may be a blank-node property list `[ p o ; … ]` or an RDF
            // collection `( … )`, each of which emits its own triples and yields a
            // fresh node (the BNPL blank, or the collection's head).
            let (subject, standalone_capable) = if self.at(&Token::LBracket) {
                (
                    self.parse_blank_node_property_list(&mut triples, &mut paths)?,
                    true,
                )
            } else if self.at(&Token::LParen) {
                (self.parse_collection(&mut triples, &mut paths)?, false)
            } else if self.at(&Token::TripleOpen) {
                // A reifying triple `<< s p o >>` emits its own reifier triples, so
                // it may stand alone (`<< s p o >> .`) with no predicate-object
                // list. A *triple term* `<<( s p o )>>` is a value: it may head a
                // subject's predicate-object list but must not stand alone.
                let node = self.parse_triple_node(&mut triples, &mut paths)?;
                let standalone_ok = !matches!(node, TermPattern::Triple(_));
                (node, standalone_ok)
            } else {
                (self.parse_term_pattern()?, false)
            };
            // A standalone `[ … ] .` needs no following predicate-object list (its
            // triples are already emitted); any other subject requires one. A
            // collection always heads a predicate-object list (it is never standalone).
            let standalone = standalone_capable
                && (self.at(&Token::Dot) || self.at(&Token::RBrace) || self.block_boundary());
            if !standalone {
                self.parse_predicate_object_list(&subject, &mut triples, &mut paths)?;
            }
            if !self.eat(&Token::Dot) {
                break;
            }
            // After a `.`, stop if the block ends (`}` or a keyword/brace).
            if self.at(&Token::RBrace) || self.block_boundary() {
                break;
            }
        }
        let mut g = GraphPattern::Bgp { patterns: triples };
        for path in paths {
            g = join(g, path);
        }
        Ok(g)
    }

    /// Parse a blank-node property list `[ predicate object … ]` (RDF 1.1 §4.2,
    /// SPARQL §19.6). Mints a fresh blank node, emits the embedded triples into
    /// the current block's `triples`/`paths`, and returns the blank node as a term
    /// for use in subject or object position.
    ///
    /// An empty `[]` (SPARQL ANON) is legal and simply mints a fresh blank node
    /// without any associated predicate-object pairs.
    fn parse_blank_node_property_list(
        &mut self,
        triples: &mut Vec<TriplePattern>,
        paths: &mut Vec<GraphPattern>,
    ) -> Result<TermPattern> {
        self.expect(&Token::LBracket)?;
        let node = TermPattern::BlankNode(self.fresh_anon());
        if !self.at(&Token::RBracket) {
            self.parse_predicate_object_list(&node, triples, paths)?;
        }
        self.expect(&Token::RBracket)?;
        Ok(node)
    }

    /// Parse an RDF collection `( n1 n2 … )` (RDF 1.1 §4.3, SPARQL §19.5
    /// `Collection`). Desugars to the standard `rdf:first`/`rdf:rest` blank-node
    /// chain terminated by `rdf:nil`, emitting those triples into the current
    /// block's `triples` and returning the HEAD node as a term for use in subject
    /// or object position. An empty list `()` is `rdf:nil` itself.
    ///
    /// Each element is a `GraphNode` — a plain term, a nested blank-node property
    /// list `[ … ]`, or a nested collection `( … )` — so the recursion mirrors the
    /// `parse_blank_node_property_list` object idiom.
    fn parse_collection(
        &mut self,
        triples: &mut Vec<TriplePattern>,
        paths: &mut Vec<GraphPattern>,
    ) -> Result<TermPattern> {
        self.expect(&Token::LParen)?;
        // The SPARQL grammar requires at least one node inside the parentheses, but
        // RDF's empty collection `()` is `rdf:nil`; accept it for robustness.
        if self.eat(&Token::RParen) {
            return Ok(TermPattern::NamedNode(NamedNode::new_unchecked(RDF_NIL)));
        }
        let first_pred = NamedNodePattern::NamedNode(NamedNode::new_unchecked(RDF_FIRST));
        let rest_pred = NamedNodePattern::NamedNode(NamedNode::new_unchecked(RDF_REST));
        let nil = TermPattern::NamedNode(NamedNode::new_unchecked(RDF_NIL));

        let head = TermPattern::BlankNode(self.fresh_anon());
        let mut node = head.clone();
        loop {
            let element = self.parse_graph_node(triples, paths)?;
            triples.push(TriplePattern {
                subject: node.clone(),
                predicate: first_pred.clone(),
                object: element,
            });
            if self.at(&Token::RParen) {
                // Last element: terminate the chain with rdf:nil.
                triples.push(TriplePattern {
                    subject: node,
                    predicate: rest_pred,
                    object: nil,
                });
                break;
            }
            // Another element follows: link to a fresh tail node.
            let next = TermPattern::BlankNode(self.fresh_anon());
            triples.push(TriplePattern {
                subject: node,
                predicate: rest_pred.clone(),
                object: next.clone(),
            });
            node = next;
        }
        self.expect(&Token::RParen)?;
        Ok(head)
    }

    /// Parse one `GraphNode` (collection element / object): a nested blank-node
    /// property list, a nested collection, or a plain term.
    fn parse_graph_node(
        &mut self,
        triples: &mut Vec<TriplePattern>,
        paths: &mut Vec<GraphPattern>,
    ) -> Result<TermPattern> {
        if self.at(&Token::LBracket) {
            self.parse_blank_node_property_list(triples, paths)
        } else if self.at(&Token::LParen) {
            self.parse_collection(triples, paths)
        } else if self.at(&Token::TripleOpen) {
            self.parse_triple_node(triples, paths)
        } else {
            self.parse_term_pattern()
        }
    }

    /// Parse an RDF 1.2 triple node in a term position:
    ///
    /// * `<<( s p o )>>` — a **triple term** (a value), yielded directly; or
    /// * `<< s p o [~ reifier] >>` — a **reifying triple**, desugared to a
    ///   reifier `R` with `R rdf:reifies <<( s p o )>>` (R fresh unless given),
    ///   and `R` is the term.
    ///
    /// The inner `s`/`o` may themselves be triple nodes (nesting is supported).
    fn parse_triple_node(
        &mut self,
        triples: &mut Vec<TriplePattern>,
        paths: &mut Vec<GraphPattern>,
    ) -> Result<TermPattern> {
        self.expect(&Token::TripleOpen)?;
        let is_triple_term = self.eat(&Token::LParen);
        let inner = self.parse_inner_triple(triples, paths)?;
        if is_triple_term {
            self.expect(&Token::RParen)?;
            self.expect(&Token::TripleClose)?;
            return Ok(TermPattern::Triple(Box::new(inner)));
        }
        // Reifying triple: optional `~ reifier`, else a fresh blank reifier.
        let reifier = if self.eat(&Token::Tilde) {
            self.parse_reifier_id()?
        } else {
            TermPattern::BlankNode(self.fresh_anon())
        };
        self.expect(&Token::TripleClose)?;
        self.emit_reifies(&reifier, &inner, triples);
        Ok(reifier)
    }

    /// Parse the `s p o` inside a `<< … >>` (reifying triple) / `<<( … )>>`
    /// (triple term). Per RDF 1.2 the component productions are restricted: a
    /// **triple term**'s subject is a `Var | iri | BlankNode` (no nested triple
    /// node), while a **reifying triple**'s subject may itself be a triple node.
    /// Neither admits an RDF collection or a populated blank-node property list in
    /// any position (both would emit auxiliary triples a single triple cannot
    /// carry).
    fn parse_inner_triple(
        &mut self,
        triples: &mut Vec<TriplePattern>,
        paths: &mut Vec<GraphPattern>,
    ) -> Result<TriplePattern> {
        let subject = self.parse_triple_node_component(triples, paths)?;
        let predicate = self.parse_predicate_name()?;
        let object = self.parse_triple_node_component(triples, paths)?;
        Ok(TriplePattern {
            subject,
            predicate,
            object,
        })
    }

    /// Parse one subject/object component of a triple node. A nested `<< … >>` /
    /// `<<( … )>>` is admissible, but an RDF collection `( … )` or a populated
    /// blank-node property list `[ p o … ]` is not (each would emit auxiliary
    /// triples a single triple cannot carry); only the anonymous `[]` (a fresh
    /// blank node) is.
    fn parse_triple_node_component(
        &mut self,
        triples: &mut Vec<TriplePattern>,
        paths: &mut Vec<GraphPattern>,
    ) -> Result<TermPattern> {
        match self.peek() {
            Some(Token::TripleOpen) => self.parse_triple_node(triples, paths),
            Some(Token::LParen) => Err(ParseError::syntax(
                "an RDF collection is not allowed inside a triple term or reifying triple",
                self.span(),
            )),
            Some(Token::LBracket) => {
                self.expect(&Token::LBracket)?;
                if !self.eat(&Token::RBracket) {
                    return Err(ParseError::syntax(
                        "a populated blank-node property list is not allowed inside a \
                         triple term or reifying triple",
                        self.span(),
                    ));
                }
                Ok(TermPattern::BlankNode(self.fresh_anon()))
            }
            _ => self.parse_term_pattern(),
        }
    }

    /// Emit `reifier rdf:reifies <<( t )>>` for a reification.
    fn emit_reifies(
        &self,
        reifier: &TermPattern,
        t: &TriplePattern,
        triples: &mut Vec<TriplePattern>,
    ) {
        triples.push(TriplePattern {
            subject: reifier.clone(),
            predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked(RDF_REIFIES)),
            object: TermPattern::Triple(Box::new(t.clone())),
        });
    }

    /// A reifier id after `~` (§ `Reifier ::= '~' VarOrReifierId?`): a variable,
    /// IRI, labelled blank node `_:b`, or anonymous `[]` — or, when none is
    /// present, a fresh blank node.
    fn parse_reifier_id(&mut self) -> Result<TermPattern> {
        match self.peek() {
            Some(Token::Variable(_)) => Ok(TermPattern::Variable(self.expect_var()?)),
            Some(Token::Iri(_) | Token::PrefixedName(_, _)) => {
                Ok(TermPattern::NamedNode(self.expect_iri_node()?))
            }
            Some(Token::BlankNodeLabel(_)) => {
                let Some(Token::BlankNodeLabel(l)) = self.bump() else {
                    unreachable!()
                };
                Ok(TermPattern::BlankNode(BlankNode::new(l)))
            }
            Some(Token::Anon) => {
                self.pos += 1;
                Ok(TermPattern::BlankNode(self.fresh_anon()))
            }
            Some(Token::LBracket) => {
                self.expect(&Token::LBracket)?;
                self.expect(&Token::RBracket)?;
                Ok(TermPattern::BlankNode(self.fresh_anon()))
            }
            _ => Ok(TermPattern::BlankNode(self.fresh_anon())),
        }
    }

    /// Parse RDF 1.2 annotation syntax trailing an asserted triple `(s, pred, o)`:
    /// zero or more reifiers `~ [id]` and annotation blocks `{| predObjList |}`.
    /// Each emits a fresh (or given) reifier `R` with `R rdf:reifies <<( s p o )>>`;
    /// an annotation block additionally applies its predicate-object list to `R`.
    fn parse_triple_annotations(
        &mut self,
        subject: &TermPattern,
        pred: &NamedNodePattern,
        object: &TermPattern,
        triples: &mut Vec<TriplePattern>,
        paths: &mut Vec<GraphPattern>,
    ) -> Result<()> {
        let base = TriplePattern {
            subject: subject.clone(),
            predicate: pred.clone(),
            object: object.clone(),
        };
        // An annotation block `{| … |}` binds to the reifier of the immediately
        // preceding `~ id` if one is pending (so `~ :r {| … |}` annotates `:r`
        // rather than a fresh node — important for DELETE templates, which forbid
        // blank nodes); otherwise it mints a fresh blank reifier.
        let mut pending: Option<TermPattern> = None;
        loop {
            if self.eat(&Token::Tilde) {
                let reifier = self.parse_reifier_id()?;
                self.emit_reifies(&reifier, &base, triples);
                pending = Some(reifier);
            } else if self.eat(&Token::AnnotationOpen) {
                let reifier = match pending.take() {
                    Some(r) => r,
                    None => {
                        let r = TermPattern::BlankNode(self.fresh_anon());
                        self.emit_reifies(&r, &base, triples);
                        r
                    }
                };
                self.parse_predicate_object_list(&reifier, triples, paths)?;
                self.expect(&Token::AnnotationClose)?;
            } else {
                break;
            }
        }
        Ok(())
    }

    /// True when the next token starts a non-triples element of a group.
    fn block_boundary(&self) -> bool {
        self.at(&Token::LBrace)
            || self.peek_kw("OPTIONAL")
            || self.peek_kw("MINUS")
            || self.peek_kw("GRAPH")
            || self.peek_kw("SERVICE")
            || self.peek_kw("FILTER")
            || self.peek_kw("BIND")
            || self.peek_kw("VALUES")
    }

    fn parse_predicate_object_list(
        &mut self,
        subject: &TermPattern,
        triples: &mut Vec<TriplePattern>,
        paths: &mut Vec<GraphPattern>,
    ) -> Result<()> {
        loop {
            // Verb = VarOrIri | path. A bare variable predicate is a simple
            // triple predicate, not a property path.
            let verb = if let Some(Token::Variable(_)) = self.peek() {
                Verb::Simple(NamedNodePattern::Variable(self.expect_var()?))
            } else {
                let path = self.parse_path()?;
                match simple_predicate(&path) {
                    Some(pred) => Verb::Simple(pred),
                    None => Verb::Path(path),
                }
            };
            // object list
            loop {
                // An object may itself be a blank-node property list `[ … ]` or an
                // RDF collection `( … )` (both emit their own triples here).
                let object = self.parse_graph_node(triples, paths)?;
                match &verb {
                    Verb::Simple(pred) => {
                        triples.push(TriplePattern {
                            subject: subject.clone(),
                            predicate: pred.clone(),
                            object: object.clone(),
                        });
                        // RDF 1.2 annotation syntax (`~ reifier`, `{| … |}`) may
                        // trail the object, reifying the triple just asserted.
                        self.parse_triple_annotations(subject, pred, &object, triples, paths)?;
                    }
                    Verb::Path(path) => paths.push(GraphPattern::Path {
                        subject: subject.clone(),
                        path: path.clone(),
                        object,
                    }),
                }
                if !self.eat(&Token::Comma) {
                    break;
                }
            }
            if !self.eat(&Token::Semicolon) {
                break;
            }
            // allow a trailing `;` before `.`/`}`/`]` (the last closes a
            // blank-node property list).
            if self.at(&Token::Dot)
                || self.at(&Token::RBrace)
                || self.at(&Token::RBracket)
                || self.block_boundary()
            {
                break;
            }
        }
        Ok(())
    }

    // ── property paths (§18.1.7 / §9) ────────────────────────────────────────

    fn parse_path(&mut self) -> Result<PropertyPathExpression> {
        self.parse_path_alternative()
    }

    fn parse_path_alternative(&mut self) -> Result<PropertyPathExpression> {
        let mut left = self.parse_path_sequence()?;
        while self.eat(&Token::Pipe) {
            let right = self.parse_path_sequence()?;
            left = PropertyPathExpression::Alternative(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_path_sequence(&mut self) -> Result<PropertyPathExpression> {
        let mut left = self.parse_path_elt_or_inverse()?;
        while self.eat(&Token::Slash) {
            let right = self.parse_path_elt_or_inverse()?;
            left = PropertyPathExpression::Sequence(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_path_elt_or_inverse(&mut self) -> Result<PropertyPathExpression> {
        if self.eat(&Token::Caret) {
            Ok(PropertyPathExpression::Reverse(Box::new(
                self.parse_path_elt()?,
            )))
        } else {
            self.parse_path_elt()
        }
    }

    fn parse_path_elt(&mut self) -> Result<PropertyPathExpression> {
        let primary = self.parse_path_primary()?;
        Ok(match self.peek() {
            Some(Token::Star) => {
                self.pos += 1;
                PropertyPathExpression::ZeroOrMore(Box::new(primary))
            }
            Some(Token::Plus) => {
                self.pos += 1;
                PropertyPathExpression::OneOrMore(Box::new(primary))
            }
            Some(Token::Question) => {
                self.pos += 1;
                PropertyPathExpression::ZeroOrOne(Box::new(primary))
            }
            // `{n}` / `{n,}` / `{n,m}` / `{,m}` — bounded repetition (a PurRDF
            // extension beyond SPARQL 1.1 §9; symmetric parse for the serializer).
            Some(Token::LBrace) => self.parse_path_range(primary)?,
            _ => primary,
        })
    }

    /// Parse a bounded-repetition postfix `{n}` / `{n,}` / `{n,m}` / `{,m}` — a
    /// PurRDF extension beyond SPARQL 1.1 §9.  The opening `{` is the current token.
    /// Hard-fails (no silent degradation) on an empty `{}`, a non-integer bound,
    /// or a lower bound exceeding the upper bound.
    fn parse_path_range(
        &mut self,
        primary: PropertyPathExpression,
    ) -> Result<PropertyPathExpression> {
        self.expect(&Token::LBrace)?;
        let lower = self.eat_integer()?;
        let has_comma = self.eat(&Token::Comma);
        let upper = if has_comma { self.eat_integer()? } else { None };
        self.expect(&Token::RBrace)?;

        let (min, max) = if has_comma {
            // `{,}` — both bounds absent — is a silent-degrade to `*`; hard-fail instead.
            if lower.is_none() && upper.is_none() {
                return Err(ParseError::syntax(
                    "empty path range {,} is not allowed (use * for zero-or-more)",
                    self.span(),
                ));
            }
            // `{n,}` / `{n,m}` / `{,m}` (missing lower ⇒ 0).
            (lower.unwrap_or(0), upper)
        } else {
            // `{n}` ⇒ exactly n; an empty `{}` is invalid.
            match lower {
                Some(n) => (n, Some(n)),
                None => {
                    return Err(ParseError::syntax(
                        "empty path range {} is not allowed",
                        self.span(),
                    ))
                }
            }
        };
        if let Some(m) = max {
            if min > m {
                return Err(ParseError::syntax(
                    format!("path range lower bound {min} exceeds upper bound {m}"),
                    self.span(),
                ));
            }
        }
        Ok(PropertyPathExpression::Range {
            inner: Box::new(primary),
            min,
            max,
        })
    }

    /// Consume an `Integer` token and parse it to `u32`, returning `Ok(None)` when
    /// the current token is not an integer (so the caller can distinguish a missing
    /// bound from a present one).  An out-of-`u32`-range integer is a hard error.
    fn eat_integer(&mut self) -> Result<Option<u32>> {
        let Some(Token::Integer(lex)) = self.peek() else {
            return Ok(None);
        };
        let lex = lex.clone();
        match lex.parse::<u32>() {
            Ok(n) => {
                self.pos += 1;
                Ok(Some(n))
            }
            Err(_) => Err(ParseError::syntax(
                format!("path range bound {lex:?} is not a valid u32"),
                self.span(),
            )),
        }
    }

    fn parse_path_primary(&mut self) -> Result<PropertyPathExpression> {
        if self.peek_kw("a") && matches!(self.peek(), Some(Token::Word(w)) if w == "a") {
            self.pos += 1;
            return Ok(PropertyPathExpression::NamedNode(NamedNode::new_unchecked(
                RDF_TYPE,
            )));
        }
        match self.peek() {
            Some(Token::Iri(_) | Token::PrefixedName(_, _)) => {
                Ok(PropertyPathExpression::NamedNode(self.expect_iri_node()?))
            }
            Some(Token::LParen) => {
                self.pos += 1;
                let inner = self.parse_path()?;
                self.expect(&Token::RParen)?;
                Ok(inner)
            }
            Some(Token::Bang) => {
                self.pos += 1;
                self.parse_negated_property_set()
            }
            other => Err(ParseError::syntax(
                format!("expected a property path, found {other:?}"),
                self.span(),
            )),
        }
    }

    fn parse_negated_property_set(&mut self) -> Result<PropertyPathExpression> {
        let mut nodes = Vec::new();
        if self.eat(&Token::LParen) {
            loop {
                nodes.push(self.parse_path_one_in_set()?);
                if !self.eat(&Token::Pipe) {
                    break;
                }
            }
            self.expect(&Token::RParen)?;
        } else {
            nodes.push(self.parse_path_one_in_set()?);
        }
        Ok(PropertyPathExpression::NegatedPropertySet(nodes))
    }

    fn parse_path_one_in_set(&mut self) -> Result<NegatedPathElement> {
        // `^iri` — an inverse link inside a negated property set (SPARQL 1.1
        // §18.2 `PathOneInPropertySet`) — excludes a *reverse* hop rather than a
        // forward one; see `NegatedPathElement` and the evaluator's decomposition
        // into a forward/reverse `Alternative`.
        let inverse = self.eat(&Token::Caret);
        if matches!(self.peek(), Some(Token::Word(w)) if w == "a") {
            self.pos += 1;
            return Ok(NegatedPathElement {
                predicate: NamedNode::new_unchecked(RDF_TYPE),
                inverse,
            });
        }
        let predicate = self.expect_iri_node()?;
        Ok(NegatedPathElement { predicate, inverse })
    }

    // ── terms ────────────────────────────────────────────────────────────────

    fn parse_term_pattern(&mut self) -> Result<TermPattern> {
        match self.peek() {
            Some(Token::Variable(_)) => Ok(TermPattern::Variable(self.expect_var()?)),
            Some(Token::Iri(_) | Token::PrefixedName(_, _)) => {
                Ok(TermPattern::NamedNode(self.expect_iri_node()?))
            }
            Some(Token::BlankNodeLabel(_)) => {
                let Some(Token::BlankNodeLabel(l)) = self.bump() else {
                    unreachable!()
                };
                Ok(TermPattern::BlankNode(BlankNode::new(l)))
            }
            Some(Token::Anon) => {
                self.pos += 1;
                Ok(TermPattern::BlankNode(self.fresh_anon()))
            }
            Some(
                Token::StringLit(_)
                | Token::LongStringLit(_)
                | Token::Integer(_)
                | Token::Decimal(_)
                | Token::Double(_),
            ) => Ok(TermPattern::Literal(self.parse_literal()?)),
            Some(Token::Word(w)) if w == "true" || w == "false" => {
                let b = matches!(self.bump(), Some(Token::Word(w)) if w == "true");
                Ok(TermPattern::Literal(Literal::new_typed(
                    if b { "true" } else { "false" },
                    NamedNode::new_unchecked(XSD_BOOLEAN),
                )))
            }
            Some(Token::TripleOpen) => {
                let t = self.parse_quoted_triple()?;
                Ok(TermPattern::Triple(Box::new(t)))
            }
            other => Err(ParseError::syntax(
                format!("expected an RDF term, found {other:?}"),
                self.span(),
            )),
        }
    }

    /// `<<( s p o )>>` or `<< s p o >>` (RDF 1.2 quoted triple / triple term).
    fn parse_quoted_triple(&mut self) -> Result<TriplePattern> {
        self.expect(&Token::TripleOpen)?;
        let parens = self.eat(&Token::LParen);
        let subject = self.parse_term_pattern()?;
        let predicate = self.parse_predicate_name()?;
        let object = self.parse_term_pattern()?;
        if parens {
            self.expect(&Token::RParen)?;
        }
        self.expect(&Token::TripleClose)?;
        Ok(TriplePattern {
            subject,
            predicate,
            object,
        })
    }

    /// A predicate in a triple position: an IRI, `a`, or a variable.
    fn parse_predicate_name(&mut self) -> Result<NamedNodePattern> {
        if matches!(self.peek(), Some(Token::Word(w)) if w == "a") {
            self.pos += 1;
            return Ok(NamedNodePattern::NamedNode(NamedNode::new_unchecked(
                RDF_TYPE,
            )));
        }
        match self.peek() {
            Some(Token::Variable(_)) => Ok(NamedNodePattern::Variable(self.expect_var()?)),
            _ => Ok(NamedNodePattern::NamedNode(self.expect_iri_node()?)),
        }
    }

    fn parse_var_or_iri_name(&mut self) -> Result<NamedNodePattern> {
        match self.peek() {
            Some(Token::Variable(_)) => Ok(NamedNodePattern::Variable(self.expect_var()?)),
            _ => Ok(NamedNodePattern::NamedNode(self.expect_iri_node()?)),
        }
    }

    fn parse_literal(&mut self) -> Result<Literal> {
        match self.bump() {
            Some(Token::Integer(s)) => {
                Ok(Literal::new_typed(s, NamedNode::new_unchecked(XSD_INTEGER)))
            }
            Some(Token::Decimal(s)) => {
                Ok(Literal::new_typed(s, NamedNode::new_unchecked(XSD_DECIMAL)))
            }
            Some(Token::Double(s)) => {
                Ok(Literal::new_typed(s, NamedNode::new_unchecked(XSD_DOUBLE)))
            }
            Some(Token::StringLit(s) | Token::LongStringLit(s)) => {
                if let Some(Token::LangTag(_)) = self.peek() {
                    let Some(Token::LangTag(tag)) = self.bump() else {
                        unreachable!()
                    };
                    let (lang, dir) = split_lang_dir(&tag);
                    Ok(Literal::new_lang(s, lang, dir))
                } else if self.eat(&Token::HatHat) {
                    let dt = self.expect_iri_node()?;
                    Ok(Literal::new_typed(s, dt))
                } else {
                    Ok(Literal::new_simple(s))
                }
            }
            other => Err(ParseError::syntax(
                format!("expected a literal, found {other:?}"),
                self.span(),
            )),
        }
    }

    fn expect_var(&mut self) -> Result<Variable> {
        match self.bump() {
            Some(Token::Variable(n)) => Ok(Variable::new(n)),
            other => Err(ParseError::syntax(
                format!("expected a variable, found {other:?}"),
                self.span(),
            )),
        }
    }

    fn expect_iri_node(&mut self) -> Result<NamedNode> {
        match self.bump() {
            Some(Token::Iri(s)) => NamedNode::new(self.resolve_iri(&s)?),
            Some(Token::PrefixedName(p, l)) => self.resolve_prefixed(&p, &l),
            other => Err(ParseError::syntax(
                format!("expected an IRI, found {other:?}"),
                self.span(),
            )),
        }
    }

    // ── VALUES / inline data ─────────────────────────────────────────────────

    fn parse_inline_data(&mut self) -> Result<GraphPattern> {
        self.expect_kw("VALUES")?;
        let mut variables = Vec::new();
        let mut bindings = Vec::new();
        if self.eat(&Token::LParen) {
            // VALUES ( ?a ?b ) { ( v v ) ... }
            while let Some(Token::Variable(_)) = self.peek() {
                let v = self.expect_var()?;
                if variables.contains(&v) {
                    return Err(ParseError::syntax(
                        format!("duplicate variable ?{} in VALUES clause", v.as_str()),
                        self.span(),
                    ));
                }
                variables.push(v);
            }
            self.expect(&Token::RParen)?;
            self.expect(&Token::LBrace)?;
            while self.eat(&Token::LParen) {
                let mut row = Vec::new();
                while !self.at(&Token::RParen) {
                    row.push(self.parse_data_cell()?);
                }
                self.expect(&Token::RParen)?;
                if row.len() != variables.len() {
                    return Err(ParseError::syntax(
                        format!(
                            "VALUES row has {} cells for {} variable(s)",
                            row.len(),
                            variables.len()
                        ),
                        self.span(),
                    ));
                }
                bindings.push(row);
            }
        } else {
            // VALUES ?a { v v ... }
            variables.push(self.expect_var()?);
            self.expect(&Token::LBrace)?;
            while !self.at(&Token::RBrace) {
                bindings.push(vec![self.parse_data_cell()?]);
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(GraphPattern::Values {
            variables,
            bindings,
        })
    }

    fn parse_data_cell(&mut self) -> Result<Option<GroundTerm>> {
        if matches!(self.peek(), Some(Token::Word(w)) if w.eq_ignore_ascii_case("UNDEF")) {
            self.pos += 1;
            return Ok(None);
        }
        Ok(Some(self.parse_ground_term()?))
    }

    fn parse_ground_term(&mut self) -> Result<GroundTerm> {
        match self.peek() {
            Some(Token::Iri(_) | Token::PrefixedName(_, _)) => {
                Ok(GroundTerm::NamedNode(self.expect_iri_node()?))
            }
            Some(Token::TripleOpen) => {
                let t = self.parse_ground_triple()?;
                Ok(GroundTerm::Triple(Box::new(t)))
            }
            Some(Token::Word(w)) if w == "true" || w == "false" => {
                let b = matches!(self.bump(), Some(Token::Word(w)) if w == "true");
                Ok(GroundTerm::Literal(Literal::new_typed(
                    if b { "true" } else { "false" },
                    NamedNode::new_unchecked(XSD_BOOLEAN),
                )))
            }
            _ => Ok(GroundTerm::Literal(self.parse_literal()?)),
        }
    }

    fn parse_ground_triple(&mut self) -> Result<GroundTriple> {
        self.expect(&Token::TripleOpen)?;
        let parens = self.eat(&Token::LParen);
        let subject = self.parse_ground_term()?;
        // A ground triple term's subject is an `iri | BlankNode` — never a literal
        // or a nested triple term (only the *object* may nest).
        if matches!(subject, GroundTerm::Triple(_) | GroundTerm::Literal(_)) {
            return Err(ParseError::syntax(
                "a literal or nested triple term may not be the subject of a triple term",
                self.span(),
            ));
        }
        // The predicate is an IRI or the `a` keyword (rdf:type).
        let predicate = if matches!(self.peek(), Some(Token::Word(w)) if w == "a") {
            self.pos += 1;
            NamedNode::new_unchecked("http://www.w3.org/1999/02/22-rdf-syntax-ns#type")
        } else {
            self.expect_iri_node()?
        };
        let object = self.parse_ground_term()?;
        if parens {
            self.expect(&Token::RParen)?;
        }
        self.expect(&Token::TripleClose)?;
        Ok(GroundTriple {
            subject,
            predicate,
            object,
        })
    }

    // ── solution modifiers ───────────────────────────────────────────────────

    /// True when the cursor is at a bare (non-parenthesized) `GROUP BY`
    /// GroupCondition — a `BuiltInCall` or `FunctionCall`. The grammar's bare
    /// conditions all begin with a callee token (a builtin keyword, an IRI, or a
    /// prefixed name); the modifier-list terminators (`HAVING`/`ORDER`/`LIMIT`/
    /// `OFFSET`/`VALUES`) and boolean literals are excluded so the `GROUP BY`
    /// loop stops cleanly at the next clause.
    fn at_bare_group_condition(&self) -> bool {
        match self.peek() {
            Some(Token::Iri(_) | Token::PrefixedName(_, _)) => true,
            Some(Token::Word(w)) => !matches!(
                w.to_ascii_uppercase().as_str(),
                "HAVING" | "ORDER" | "LIMIT" | "OFFSET" | "VALUES" | "BINDINGS" | "TRUE" | "FALSE"
            ),
            _ => false,
        }
    }

    fn parse_solution_modifiers(
        &mut self,
        aggregates: &mut Vec<(Variable, AggregateExpression)>,
    ) -> Result<Modifiers> {
        let mut m = Modifiers::default();
        if self.eat_kw("GROUP") {
            self.expect_kw("BY")?;
            loop {
                if let Some(Token::Variable(_)) = self.peek() {
                    m.group_by.push(self.expect_var()?);
                } else if self.at(&Token::LParen) {
                    // `( Expr [AS ?v] )` — SPARQL 1.1 §18.2.4 GroupCondition. Lower
                    // to an Extend(?v := Expr) under the Group, then group by ?v.
                    self.expect(&Token::LParen)?;
                    // Non-lifting parse: an aggregate in a GROUP BY key is illegal
                    // and surfaces here as `Unsupported`.
                    let expr = self.parse_expression()?;
                    let var = if self.eat_kw("AS") {
                        self.expect_var()?
                    } else {
                        self.fresh_group_var()
                    };
                    self.expect(&Token::RParen)?;
                    m.group_extends.push((var.clone(), expr));
                    m.group_by.push(var);
                } else if self.at_bare_group_condition() {
                    // A bare `BuiltInCall` / `FunctionCall` GroupCondition, e.g.
                    // `GROUP BY STR(?x)` — lower to a synthetic-var Extend.
                    let expr = self.parse_expression()?;
                    let var = self.fresh_group_var();
                    m.group_extends.push((var.clone(), expr));
                    m.group_by.push(var);
                } else {
                    break;
                }
            }
        }
        if self.eat_kw("HAVING") {
            loop {
                let expr = self.parse_having_constraint(aggregates)?;
                m.having.push(expr);
                if !self.at(&Token::LParen) {
                    break;
                }
            }
        }
        if self.eat_kw("ORDER") {
            self.expect_kw("BY")?;
            loop {
                let cond = if self.eat_kw("ASC") {
                    self.expect(&Token::LParen)?;
                    let e = self.parse_expression_lifting_aggs(aggregates)?;
                    self.expect(&Token::RParen)?;
                    OrderExpression::Asc(e)
                } else if self.eat_kw("DESC") {
                    self.expect(&Token::LParen)?;
                    let e = self.parse_expression_lifting_aggs(aggregates)?;
                    self.expect(&Token::RParen)?;
                    OrderExpression::Desc(e)
                } else if self.order_key_ahead() {
                    OrderExpression::Asc(self.parse_primary_with_aggs(aggregates)?)
                } else {
                    break;
                };
                m.order_by.push(cond);
            }
        }
        // LIMIT / OFFSET in either order.
        loop {
            if self.eat_kw("LIMIT") {
                m.limit = Some(self.expect_integer()?);
            } else if self.eat_kw("OFFSET") {
                m.offset = Some(self.expect_integer()?);
            } else {
                break;
            }
        }
        Ok(m)
    }

    fn order_key_ahead(&self) -> bool {
        matches!(
            self.peek(),
            Some(Token::Variable(_) | Token::LParen | Token::Iri(_))
        ) || matches!(self.peek(), Some(Token::Word(w)) if is_builtin_function(w))
    }

    fn expect_integer(&mut self) -> Result<usize> {
        match self.bump() {
            Some(Token::Integer(s)) => s
                .parse::<usize>()
                .map_err(|_| ParseError::syntax(format!("bad integer {s:?}"), self.span())),
            other => Err(ParseError::syntax(
                format!("expected an integer, found {other:?}"),
                self.span(),
            )),
        }
    }

    // ── expressions ──────────────────────────────────────────────────────────

    /// FILTER constraint: a bracketted expression, a built-in call, or a
    /// function call (§ Constraint).
    fn parse_constraint(&mut self) -> Result<Expression> {
        if self.at(&Token::LParen) {
            self.pos += 1;
            let e = self.parse_expression()?;
            self.expect(&Token::RParen)?;
            Ok(e)
        } else {
            self.parse_primary_expression()
        }
    }

    fn parse_having_constraint(
        &mut self,
        aggregates: &mut Vec<(Variable, AggregateExpression)>,
    ) -> Result<Expression> {
        self.expect(&Token::LParen)?;
        let e = self.parse_expression_lifting_aggs(aggregates)?;
        self.expect(&Token::RParen)?;
        Ok(e)
    }

    fn parse_expression(&mut self) -> Result<Expression> {
        let mut sink = Vec::new();
        let e = self.parse_or(&mut sink)?;
        if !sink.is_empty() {
            return Err(ParseError::unsupported(
                "aggregate outside GROUP BY / SELECT / HAVING context",
            ));
        }
        Ok(e)
    }

    fn parse_expression_lifting_aggs(
        &mut self,
        aggregates: &mut Vec<(Variable, AggregateExpression)>,
    ) -> Result<Expression> {
        self.parse_or(aggregates)
    }

    fn parse_or(&mut self, aggs: &mut Vec<(Variable, AggregateExpression)>) -> Result<Expression> {
        let mut left = self.parse_and(aggs)?;
        while self.eat(&Token::Or) {
            let right = self.parse_and(aggs)?;
            left = Expression::Or(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_and(&mut self, aggs: &mut Vec<(Variable, AggregateExpression)>) -> Result<Expression> {
        let mut left = self.parse_relational(aggs)?;
        while self.eat(&Token::And) {
            let right = self.parse_relational(aggs)?;
            left = Expression::And(Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_relational(
        &mut self,
        aggs: &mut Vec<(Variable, AggregateExpression)>,
    ) -> Result<Expression> {
        let left = self.parse_additive(aggs)?;
        let op = match self.peek() {
            Some(Token::Eq) => Some("="),
            Some(Token::NotEq) => Some("!="),
            Some(Token::Lt) => Some("<"),
            Some(Token::Gt) => Some(">"),
            Some(Token::LtEq) => Some("<="),
            Some(Token::GtEq) => Some(">="),
            _ => None,
        };
        if let Some(op) = op {
            self.pos += 1;
            let right = self.parse_additive(aggs)?;
            let (l, r) = (Box::new(left), Box::new(right));
            return Ok(match op {
                "=" => Expression::Equal(l, r),
                "!=" => Expression::Not(Box::new(Expression::Equal(l, r))),
                "<" => Expression::Less(l, r),
                ">" => Expression::Greater(l, r),
                "<=" => Expression::LessOrEqual(l, r),
                _ => Expression::GreaterOrEqual(l, r),
            });
        }
        if self.peek_kw("IN") {
            self.pos += 1;
            let list = self.parse_expression_list(aggs)?;
            return Ok(Expression::In(Box::new(left), list));
        }
        if self.peek_kw("NOT") && self.peek2_kw("IN") {
            self.pos += 2;
            let list = self.parse_expression_list(aggs)?;
            return Ok(Expression::Not(Box::new(Expression::In(
                Box::new(left),
                list,
            ))));
        }
        Ok(left)
    }

    fn parse_additive(
        &mut self,
        aggs: &mut Vec<(Variable, AggregateExpression)>,
    ) -> Result<Expression> {
        let mut left = self.parse_multiplicative(aggs)?;
        loop {
            if self.eat(&Token::Plus) {
                let right = self.parse_multiplicative(aggs)?;
                left = Expression::Add(Box::new(left), Box::new(right));
            } else if self.eat(&Token::Minus) {
                let right = self.parse_multiplicative(aggs)?;
                left = Expression::Subtract(Box::new(left), Box::new(right));
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_multiplicative(
        &mut self,
        aggs: &mut Vec<(Variable, AggregateExpression)>,
    ) -> Result<Expression> {
        let mut left = self.parse_unary(aggs)?;
        loop {
            if self.eat(&Token::Star) {
                let right = self.parse_unary(aggs)?;
                left = Expression::Multiply(Box::new(left), Box::new(right));
            } else if self.eat(&Token::Slash) {
                let right = self.parse_unary(aggs)?;
                left = Expression::Divide(Box::new(left), Box::new(right));
            } else {
                break;
            }
        }
        Ok(left)
    }

    fn parse_unary(
        &mut self,
        aggs: &mut Vec<(Variable, AggregateExpression)>,
    ) -> Result<Expression> {
        if self.eat(&Token::Bang) {
            Ok(Expression::Not(Box::new(self.parse_unary(aggs)?)))
        } else if self.eat(&Token::Plus) {
            Ok(Expression::UnaryPlus(Box::new(self.parse_unary(aggs)?)))
        } else if self.eat(&Token::Minus) {
            Ok(Expression::UnaryMinus(Box::new(self.parse_unary(aggs)?)))
        } else {
            self.parse_primary_with_aggs(aggs)
        }
    }

    /// Parse an RDF 1.2 triple term `<<( s p o )>>` in *expression* position
    /// (`ExprTripleTerm`, §17.4). It denotes the same value as `TRIPLE(s, p, o)`,
    /// so it lowers to that function call. Only the triple-*term* form (`<<(`) is
    /// valid here — a reifying triple `<< … >>` is not an expression.
    fn parse_triple_term_expr(
        &mut self,
        aggs: &mut Vec<(Variable, AggregateExpression)>,
    ) -> Result<Expression> {
        self.expect(&Token::TripleOpen)?;
        if !self.eat(&Token::LParen) {
            return Err(ParseError::syntax(
                "a reifying triple `<< … >>` is not valid in expression position; \
                 use a triple term `<<( s p o )>>`",
                self.span(),
            ));
        }
        // A triple term's subject is a `Var | iri` here — never a literal or a
        // nested triple term.
        if matches!(
            self.peek(),
            Some(
                Token::TripleOpen
                    | Token::StringLit(_)
                    | Token::LongStringLit(_)
                    | Token::Integer(_)
                    | Token::Decimal(_)
                    | Token::Double(_)
            )
        ) {
            return Err(ParseError::syntax(
                "a literal or nested triple term may not be the subject of a triple term",
                self.span(),
            ));
        }
        let s = self.parse_primary_with_aggs(aggs)?;
        let p = self.parse_primary_with_aggs(aggs)?;
        let o = self.parse_primary_with_aggs(aggs)?;
        self.expect(&Token::RParen)?;
        self.expect(&Token::TripleClose)?;
        Ok(Expression::FunctionCall(Function::Triple, vec![s, p, o]))
    }

    fn parse_primary_expression(&mut self) -> Result<Expression> {
        let mut sink = Vec::new();
        let e = self.parse_primary_with_aggs(&mut sink)?;
        if !sink.is_empty() {
            return Err(ParseError::unsupported("aggregate in this position"));
        }
        Ok(e)
    }

    fn parse_primary_with_aggs(
        &mut self,
        aggs: &mut Vec<(Variable, AggregateExpression)>,
    ) -> Result<Expression> {
        match self.peek() {
            Some(Token::LParen) => {
                self.pos += 1;
                let e = self.parse_or(aggs)?;
                self.expect(&Token::RParen)?;
                Ok(e)
            }
            Some(Token::Variable(_)) => Ok(Expression::Variable(self.expect_var()?)),
            Some(Token::Iri(_) | Token::PrefixedName(_, _)) => self.parse_iri_or_function(aggs),
            Some(
                Token::StringLit(_)
                | Token::LongStringLit(_)
                | Token::Integer(_)
                | Token::Decimal(_)
                | Token::Double(_),
            ) => Ok(Expression::Literal(self.parse_literal()?)),
            Some(Token::TripleOpen) => self.parse_triple_term_expr(aggs),
            Some(Token::Word(w)) => {
                let w = w.clone();
                if w == "true" || w == "false" {
                    self.pos += 1;
                    Ok(Expression::Literal(Literal::new_typed(
                        if w == "true" { "true" } else { "false" },
                        NamedNode::new_unchecked(XSD_BOOLEAN),
                    )))
                } else {
                    self.parse_builtin_or_aggregate(&w, aggs)
                }
            }
            other => Err(ParseError::syntax(
                format!("expected an expression, found {other:?}"),
                self.span(),
            )),
        }
    }

    fn parse_iri_or_function(
        &mut self,
        aggs: &mut Vec<(Variable, AggregateExpression)>,
    ) -> Result<Expression> {
        let node = self.expect_iri_node()?;
        if self.at(&Token::LParen) {
            // An IRI in call position under ANY configured extension-function namespace
            // (default: NONE — the namespace set is caller configuration supplied via
            // ParserOptions, e.g. the gmeow namespace) dispatches to the CLOSED
            // extension-function seam, recognized here at parse time. The local-name
            // MUST resolve; an unknown <ns>foo(...) under a configured namespace is a
            // hard error (fail-fast), never a silent Function::Custom fallthrough. An
            // IRI under NO configured namespace stays Function::Custom. The original
            // IRI is recorded in the AST node so serialization round-trips exactly.
            let ext_local = self
                .options
                .extension_fn_namespaces
                .iter()
                .find_map(|ns| node.as_str().strip_prefix(ns.as_str()));
            let func = if let Some(local) = ext_local {
                match crate::algebra::PurrdfFn::from_local_name(local) {
                    Some(fn_kind) => Function::Purrdf(crate::algebra::PurrdfCall {
                        fn_kind,
                        iri: node.as_str().to_owned(),
                    }),
                    None => {
                        return Err(ParseError::syntax(
                            format!("unknown extension function <{}>", node.as_str()),
                            self.span(),
                        ));
                    }
                }
            } else {
                Function::Custom(node)
            };
            let args = self.parse_arg_list(aggs)?;
            Ok(Expression::FunctionCall(func, args))
        } else {
            Ok(Expression::NamedNode(node))
        }
    }

    fn parse_builtin_or_aggregate(
        &mut self,
        name: &str,
        aggs: &mut Vec<(Variable, AggregateExpression)>,
    ) -> Result<Expression> {
        let upper = name.to_ascii_uppercase();
        // Aggregates lift to a synthetic Group variable.
        if let Some(func) = aggregate_function(&upper) {
            return self.parse_aggregate(func, aggs);
        }
        match upper.as_str() {
            "BOUND" => {
                self.pos += 1;
                self.expect(&Token::LParen)?;
                let v = self.expect_var()?;
                self.expect(&Token::RParen)?;
                Ok(Expression::Bound(v))
            }
            "IF" => {
                self.pos += 1;
                let args = self.parse_arg_list(aggs)?;
                expect_arity(&args, 3, "IF", self.span())?;
                let mut it = args.into_iter();
                Ok(Expression::If(
                    Box::new(it.next().unwrap()),
                    Box::new(it.next().unwrap()),
                    Box::new(it.next().unwrap()),
                ))
            }
            "COALESCE" => {
                self.pos += 1;
                Ok(Expression::Coalesce(self.parse_arg_list(aggs)?))
            }
            "EXISTS" => {
                self.pos += 1;
                Ok(Expression::Exists(Box::new(
                    self.parse_group_graph_pattern()?,
                )))
            }
            "NOT" => {
                self.pos += 1;
                self.expect_kw("EXISTS")?;
                Ok(Expression::Not(Box::new(Expression::Exists(Box::new(
                    self.parse_group_graph_pattern()?,
                )))))
            }
            "SAMETERM" => {
                self.pos += 1;
                let args = self.parse_arg_list(aggs)?;
                expect_arity(&args, 2, "sameTerm", self.span())?;
                let mut it = args.into_iter();
                Ok(Expression::SameTerm(
                    Box::new(it.next().unwrap()),
                    Box::new(it.next().unwrap()),
                ))
            }
            _ => {
                if let Some(func) = builtin_function(&upper) {
                    self.pos += 1;
                    let args = self.parse_arg_list(aggs)?;
                    Ok(Expression::FunctionCall(func, args))
                } else {
                    Err(ParseError::unsupported(format!(
                        "function or keyword {name}"
                    )))
                }
            }
        }
    }

    fn parse_aggregate(
        &mut self,
        func: AggregateFunction,
        aggs: &mut Vec<(Variable, AggregateExpression)>,
    ) -> Result<Expression> {
        self.pos += 1; // function name
        self.expect(&Token::LParen)?;
        // DISTINCT precedes `*` in `COUNT(DISTINCT *)`; consume it first so the
        // star form carries the flag (the `*` arm previously hid the DISTINCT,
        // making CountStar { distinct: true } unreachable).
        let distinct = self.eat_kw("DISTINCT");
        let agg = if self.eat(&Token::Star) {
            // COUNT(*) / COUNT(DISTINCT *)
            AggregateExpression::CountStar { distinct }
        } else {
            let inner = self.parse_expression()?;
            let separator = if let AggregateFunction::GroupConcat { .. } = func {
                self.parse_optional_separator()?
            } else {
                None
            };
            let function = match func {
                AggregateFunction::GroupConcat { .. } => {
                    AggregateFunction::GroupConcat { separator }
                }
                other => other,
            };
            AggregateExpression::FunctionCall {
                function,
                expression: Box::new(inner),
                distinct,
            }
        };
        self.expect(&Token::RParen)?;
        let synth = self.fresh_agg_var();
        aggs.push((synth.clone(), agg));
        Ok(Expression::Variable(synth))
    }

    fn parse_optional_separator(&mut self) -> Result<Option<String>> {
        if self.eat(&Token::Semicolon) {
            self.expect_kw("SEPARATOR")?;
            self.expect(&Token::Eq)?;
            match self.bump() {
                Some(Token::StringLit(s) | Token::LongStringLit(s)) => Ok(Some(s)),
                other => Err(ParseError::syntax(
                    format!("expected SEPARATOR string, found {other:?}"),
                    self.span(),
                )),
            }
        } else {
            Ok(None)
        }
    }

    fn fresh_agg_var(&mut self) -> Variable {
        let v = Variable::new(format!("__purrdf_agg_{}", self.agg_counter));
        self.agg_counter += 1;
        v
    }

    /// Mint a fresh, unique grouping variable for an expression-valued
    /// `GROUP BY (Expr)` condition with no explicit `AS`. Distinct namespace from
    /// `fresh_agg_var` so the two never collide.
    fn fresh_group_var(&mut self) -> Variable {
        let v = Variable::new(format!("__purrdf_group_{}", self.group_counter));
        self.group_counter += 1;
        v
    }

    /// Mint a fresh, unique label for an anonymous blank node (`[]`). Each
    /// occurrence is a distinct existential; reusing one label (e.g. `""`) would
    /// wrongly fuse separate blank nodes into a single AST node.
    fn fresh_anon(&mut self) -> BlankNode {
        let b = BlankNode::new(format!("__purrdf_anon_{}", self.anon_counter));
        self.anon_counter += 1;
        b
    }

    fn parse_arg_list(
        &mut self,
        aggs: &mut Vec<(Variable, AggregateExpression)>,
    ) -> Result<Vec<Expression>> {
        self.expect(&Token::LParen)?;
        let mut args = Vec::new();
        if self.eat(&Token::Star) {
            // e.g. COUNT(*) handled elsewhere; a bare `*` here is invalid.
            return Err(ParseError::syntax(
                "unexpected '*' in argument list",
                self.span(),
            ));
        }
        if !self.at(&Token::RParen) {
            self.eat_kw("DISTINCT");
            loop {
                args.push(self.parse_or(aggs)?);
                if !self.eat(&Token::Comma) {
                    break;
                }
            }
        }
        self.expect(&Token::RParen)?;
        Ok(args)
    }

    fn parse_expression_list(
        &mut self,
        aggs: &mut Vec<(Variable, AggregateExpression)>,
    ) -> Result<Vec<Expression>> {
        self.expect(&Token::LParen)?;
        let mut list = Vec::new();
        if !self.at(&Token::RParen) {
            loop {
                list.push(self.parse_or(aggs)?);
                if !self.eat(&Token::Comma) {
                    break;
                }
            }
        }
        self.expect(&Token::RParen)?;
        Ok(list)
    }
}

/// A parsed predicate: a simple verb (IRI/`a`/variable) yielding a triple, or a
/// complex property path yielding a `GraphPattern::Path`.
enum Verb {
    Simple(NamedNodePattern),
    Path(PropertyPathExpression),
}

#[derive(Default)]
struct Modifiers {
    group_by: Vec<Variable>,
    /// `(Expr AS ?v)` / bare-expression `GROUP BY` conditions, lowered to
    /// `Extend(?v := Expr)` nodes inserted *under* the `Group` (SPARQL 1.1
    /// §18.2.4). Each synthetic/explicit `?v` minted here is also pushed to
    /// `group_by` as a grouping key.
    group_extends: Vec<(Variable, Expression)>,
    having: Vec<Expression>,
    order_by: Vec<OrderExpression>,
    limit: Option<usize>,
    offset: Option<usize>,
}

impl Modifiers {
    /// True when no solution modifier was parsed at all.
    fn is_empty(&self) -> bool {
        self.group_by.is_empty()
            && self.group_extends.is_empty()
            && self.having.is_empty()
            && self.order_by.is_empty()
            && self.limit.is_none()
            && self.offset.is_none()
    }
}

// ── free helpers ─────────────────────────────────────────────────────────────

/// Join two patterns, merging adjacent BGPs and absorbing the empty pattern (the
/// identity table `Z`) on either side so a group that opens with a non-triple
/// element (`UNION`, a property path, …) is not wrapped in a vacuous `Join`.
fn join(left: GraphPattern, right: GraphPattern) -> GraphPattern {
    if is_empty_bgp(&left) {
        return right;
    }
    if is_empty_bgp(&right) {
        return left;
    }
    match (left, right) {
        (GraphPattern::Bgp { mut patterns }, GraphPattern::Bgp { patterns: r }) => {
            patterns.extend(r);
            GraphPattern::Bgp { patterns }
        }
        (l, r) => GraphPattern::Join {
            left: Box::new(l),
            right: Box::new(r),
        },
    }
}

fn is_empty_bgp(p: &GraphPattern) -> bool {
    matches!(p, GraphPattern::Bgp { patterns } if patterns.is_empty())
}

/// If a property path is length-1 (a single predicate), return it as a triple
/// predicate; complex paths return `None` (they become `GraphPattern::Path`).
fn simple_predicate(path: &PropertyPathExpression) -> Option<NamedNodePattern> {
    match path {
        PropertyPathExpression::NamedNode(n) => Some(NamedNodePattern::NamedNode(n.clone())),
        _ => None,
    }
}

/// Lift a trailing `Filter` out of an `OPTIONAL` body so it becomes the
/// `LeftJoin` join condition (§18.2.2.3 "filter-in-optional").
fn split_trailing_filter(p: GraphPattern) -> (GraphPattern, Option<Expression>) {
    match p {
        GraphPattern::Filter { expr, inner } => (*inner, Some(expr)),
        other => (other, None),
    }
}

/// Collect the in-scope variables of a pattern in first-appearance order
/// (used for `SELECT *` projection).
fn visible_variables(p: &GraphPattern) -> Vec<Variable> {
    let mut out = Vec::new();
    collect_vars(p, &mut out);
    out
}

fn push_var(v: &Variable, out: &mut Vec<Variable>) {
    if !out.contains(v) {
        out.push(v.clone());
    }
}

fn collect_term_vars(t: &TermPattern, out: &mut Vec<Variable>) {
    match t {
        TermPattern::Variable(v) => push_var(v, out),
        TermPattern::Triple(tp) => {
            collect_term_vars(&tp.subject, out);
            if let NamedNodePattern::Variable(v) = &tp.predicate {
                push_var(v, out);
            }
            collect_term_vars(&tp.object, out);
        }
        _ => {}
    }
}

fn collect_triple_vars(tp: &TriplePattern, out: &mut Vec<Variable>) {
    collect_term_vars(&tp.subject, out);
    if let NamedNodePattern::Variable(v) = &tp.predicate {
        push_var(v, out);
    }
    collect_term_vars(&tp.object, out);
}

fn collect_vars(p: &GraphPattern, out: &mut Vec<Variable>) {
    match p {
        GraphPattern::Bgp { patterns } => {
            for tp in patterns {
                collect_triple_vars(tp, out);
            }
        }
        GraphPattern::Path {
            subject, object, ..
        } => {
            collect_term_vars(subject, out);
            collect_term_vars(object, out);
        }
        GraphPattern::Join { left, right }
        | GraphPattern::Union { left, right }
        | GraphPattern::Lateral { left, right } => {
            collect_vars(left, out);
            collect_vars(right, out);
        }
        // SPARQL §18.2.1: variables occurring only in the right operand of
        // MINUS are not in scope in the enclosing group graph pattern, so we
        // descend into `left` only.
        GraphPattern::Minus { left, .. } => {
            collect_vars(left, out);
        }
        GraphPattern::LeftJoin { left, right, .. } => {
            collect_vars(left, out);
            collect_vars(right, out);
        }
        GraphPattern::Filter { inner, .. }
        | GraphPattern::OrderBy { inner, .. }
        | GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. } => collect_vars(inner, out),
        GraphPattern::Graph { name, inner } | GraphPattern::Service { name, inner, .. } => {
            if let NamedNodePattern::Variable(v) = name {
                push_var(v, out);
            }
            collect_vars(inner, out);
        }
        GraphPattern::Extend {
            inner, variable, ..
        } => {
            collect_vars(inner, out);
            push_var(variable, out);
        }
        GraphPattern::Values { variables, .. } => {
            for v in variables {
                push_var(v, out);
            }
        }
        GraphPattern::Project { variables, .. } => {
            for v in variables {
                push_var(v, out);
            }
        }
        GraphPattern::Group {
            variables,
            aggregates,
            ..
        } => {
            for v in variables {
                push_var(v, out);
            }
            for (v, _) in aggregates {
                push_var(v, out);
            }
        }
    }
}

/// Collect the labels of every blank node in a run of quad patterns, descending
/// into RDF-1.2 quoted triples. Used to enforce the §19.6 rule that a blank node
/// label may not be shared across two operations of one update request.
fn collect_quad_bnode_labels(quads: &[QuadPattern], out: &mut std::collections::HashSet<String>) {
    for q in quads {
        collect_triple_bnode_labels(&q.triple, out);
    }
}

fn collect_triple_bnode_labels(t: &TriplePattern, out: &mut std::collections::HashSet<String>) {
    collect_term_bnode_labels(&t.subject, out);
    collect_term_bnode_labels(&t.object, out);
}

fn collect_term_bnode_labels(t: &TermPattern, out: &mut std::collections::HashSet<String>) {
    match t {
        TermPattern::BlankNode(b) => {
            out.insert(b.as_str().to_owned());
        }
        TermPattern::Triple(tp) => collect_triple_bnode_labels(tp, out),
        _ => {}
    }
}

/// Hard-fail if any subject/object position of a triple pattern (descending into
/// RDF 1.2 quoted triples) is a blank node. Blank nodes are disallowed in DELETE
/// templates and `DELETE WHERE` (SPARQL 1.1 Update §3.1.3 / §3.1.3.2).
fn reject_blank_in_triple_pattern(t: &TriplePattern, at: usize) -> Result<()> {
    reject_blank_in_term_pattern(&t.subject, at)?;
    reject_blank_in_term_pattern(&t.object, at)
}

fn reject_blank_in_term_pattern(t: &TermPattern, at: usize) -> Result<()> {
    match t {
        TermPattern::BlankNode(_) => Err(ParseError::syntax(
            "blank node in a DELETE template is not allowed",
            at,
        )),
        TermPattern::Triple(tp) => reject_blank_in_triple_pattern(tp, at),
        _ => Ok(()),
    }
}

fn is_absolute_iri(s: &str) -> bool {
    // A scheme followed by ':' — RFC-3986 §3.1 (cheap prefix test).
    let mut chars = s.char_indices();
    match chars.next() {
        Some((_, c)) if c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    for (_, c) in chars {
        if c == ':' {
            return true;
        }
        if !(c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.')) {
            return false;
        }
    }
    false
}

/// Split a lang tag into the language and an optional RDF 1.2 base direction
/// (`en--ltr` → (`en`, Ltr)).
fn split_lang_dir(tag: &str) -> (String, Option<BaseDirection>) {
    if let Some((lang, dir)) = tag.split_once("--") {
        let dir = match dir.to_ascii_lowercase().as_str() {
            "ltr" => Some(BaseDirection::Ltr),
            "rtl" => Some(BaseDirection::Rtl),
            _ => None,
        };
        (lang.to_owned(), dir)
    } else {
        (tag.to_owned(), None)
    }
}

fn expect_arity(args: &[Expression], n: usize, name: &str, at: usize) -> Result<()> {
    if args.len() == n {
        Ok(())
    } else {
        Err(ParseError::syntax(
            format!("{name} expects {n} arguments, got {}", args.len()),
            at,
        ))
    }
}

fn aggregate_function(upper: &str) -> Option<AggregateFunction> {
    Some(match upper {
        "COUNT" => AggregateFunction::Count,
        "SUM" => AggregateFunction::Sum,
        "AVG" => AggregateFunction::Avg,
        "MIN" => AggregateFunction::Min,
        "MAX" => AggregateFunction::Max,
        "SAMPLE" => AggregateFunction::Sample,
        "GROUP_CONCAT" => AggregateFunction::GroupConcat { separator: None },
        _ => return None,
    })
}

fn is_builtin_function(name: &str) -> bool {
    builtin_function(&name.to_ascii_uppercase()).is_some()
}

fn builtin_function(upper: &str) -> Option<Function> {
    Some(match upper {
        "STR" => Function::Str,
        "LANG" => Function::Lang,
        "LANGDIR" => Function::LangDir,
        "STRLANGDIR" => Function::StrLangDir,
        "HASLANG" => Function::HasLang,
        "HASLANGDIR" => Function::HasLangDir,
        "LANGMATCHES" => Function::LangMatches,
        "DATATYPE" => Function::Datatype,
        "IRI" => Function::Iri,
        "URI" => Function::Uri,
        "BNODE" => Function::BNode,
        "RAND" => Function::Rand,
        "ABS" => Function::Abs,
        "CEIL" => Function::Ceil,
        "FLOOR" => Function::Floor,
        "ROUND" => Function::Round,
        "CONCAT" => Function::Concat,
        "SUBSTR" => Function::SubStr,
        "STRLEN" => Function::StrLen,
        "REPLACE" => Function::Replace,
        "UCASE" => Function::UCase,
        "LCASE" => Function::LCase,
        "ENCODE_FOR_URI" => Function::EncodeForUri,
        "CONTAINS" => Function::Contains,
        "STRSTARTS" => Function::StrStarts,
        "STRENDS" => Function::StrEnds,
        "STRBEFORE" => Function::StrBefore,
        "STRAFTER" => Function::StrAfter,
        "YEAR" => Function::Year,
        "MONTH" => Function::Month,
        "DAY" => Function::Day,
        "HOURS" => Function::Hours,
        "MINUTES" => Function::Minutes,
        "SECONDS" => Function::Seconds,
        "TIMEZONE" => Function::Timezone,
        "TZ" => Function::Tz,
        "NOW" => Function::Now,
        "UUID" => Function::Uuid,
        "STRUUID" => Function::StrUuid,
        "MD5" => Function::Md5,
        "SHA1" => Function::Sha1,
        "SHA256" => Function::Sha256,
        "SHA384" => Function::Sha384,
        "SHA512" => Function::Sha512,
        "STRLANG" => Function::StrLang,
        "STRDT" => Function::StrDt,
        "ISIRI" => Function::IsIri,
        "ISURI" => Function::IsUri,
        "ISBLANK" => Function::IsBlank,
        "ISLITERAL" => Function::IsLiteral,
        "ISNUMERIC" => Function::IsNumeric,
        "REGEX" => Function::Regex,
        "TRIPLE" => Function::Triple,
        "SUBJECT" => Function::Subject,
        "PREDICATE" => Function::Predicate,
        "OBJECT" => Function::Object,
        "ISTRIPLE" => Function::IsTriple,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::algebra::{PurrdfCall, PurrdfFn};
    use pretty_assertions::assert_eq;

    const GM: &str =
        "PREFIX purrdf: <https://x/>\nPREFIX rdf: <http://r/>\nPREFIX rdfs: <http://s/>\n";

    fn parse(q: &str) -> Query {
        SparqlParser::new().parse_query(q).expect("parse")
    }

    fn select_pattern(q: &str) -> GraphPattern {
        match parse(q) {
            Query::Select { pattern, .. } => pattern,
            other => panic!("expected SELECT, got {other:?}"),
        }
    }

    /// Strip the outer `Project` wrapper to reach the WHERE algebra.
    fn unproject(p: GraphPattern) -> GraphPattern {
        match p {
            GraphPattern::Project { inner, .. } => *inner,
            other => other,
        }
    }

    #[test]
    fn inverse_in_negated_property_set_parses_with_direction() {
        // `!(^iri)` — the inverse element is preserved as a `NegatedPathElement`
        // with `inverse: true`, not silently degraded to the forward `!(iri)`.
        let q = format!("{GM}SELECT ?x WHERE {{ ?x !(^purrdf:p) ?y }}");
        let pattern = unproject(select_pattern(&q));
        let GraphPattern::Path { path, .. } = pattern else {
            panic!("expected a Path pattern, got {pattern:?}");
        };
        match path {
            PropertyPathExpression::NegatedPropertySet(elems) => {
                assert_eq!(elems.len(), 1);
                assert!(elems[0].inverse, "^purrdf:p must set inverse: true");
                assert_eq!(elems[0].predicate.as_str(), "https://x/p");
            }
            other => panic!("expected NegatedPropertySet, got {other:?}"),
        }
    }

    #[test]
    fn distinct_anonymous_blank_nodes_do_not_collapse() {
        // Two `[]` are two distinct existentials; they must not fuse into one
        // AST node (which would wrongly merge the triples that mention them).
        let q = format!("{GM}SELECT ?x WHERE {{ [] purrdf:p ?x . [] purrdf:q ?x }}");
        let GraphPattern::Bgp { patterns } = unproject(select_pattern(&q)) else {
            panic!("expected BGP");
        };
        assert_eq!(patterns.len(), 2);
        let (TermPattern::BlankNode(a), TermPattern::BlankNode(b)) =
            (&patterns[0].subject, &patterns[1].subject)
        else {
            panic!("both subjects should be blank nodes");
        };
        assert_ne!(a, b, "distinct [] must produce distinct blank nodes");
    }

    #[test]
    fn quoted_triple_with_variable_predicate() {
        // The RDF-1.2 codec shape: `?r rdf:reifies <<( ?s ?p ?o )>>`.
        let q = format!("{GM}SELECT ?r WHERE {{ ?r rdf:reifies <<( ?s ?p ?o )>> . }}");
        let where_pat = unproject(select_pattern(&q));
        let GraphPattern::Bgp { patterns } = where_pat else {
            panic!("expected BGP, got {where_pat:?}");
        };
        assert_eq!(patterns.len(), 1);
        let TermPattern::Triple(inner) = &patterns[0].object else {
            panic!(
                "object should be a quoted triple, got {:?}",
                patterns[0].object
            );
        };
        assert_eq!(
            inner.predicate,
            NamedNodePattern::Variable(Variable::new("p"))
        );
        assert_eq!(inner.subject, TermPattern::Variable(Variable::new("s")));
    }

    #[test]
    fn optional_lifts_trailing_filter_to_leftjoin() {
        let q = format!(
            "{GM}SELECT ?a WHERE {{ ?a a purrdf:T . OPTIONAL {{ ?a purrdf:p ?b . FILTER(?b != ?a) }} }}"
        );
        let where_pat = unproject(select_pattern(&q));
        let GraphPattern::LeftJoin { expression, .. } = where_pat else {
            panic!("expected LeftJoin, got {where_pat:?}");
        };
        assert!(expression.is_some(), "FILTER should lift into the LeftJoin");
    }

    #[test]
    fn union_of_two_groups() {
        let q = format!("{GM}SELECT ?a WHERE {{ {{ ?a a purrdf:X }} UNION {{ ?a a purrdf:Y }} }}");
        let where_pat = unproject(select_pattern(&q));
        assert!(
            matches!(where_pat, GraphPattern::Union { .. }),
            "got {where_pat:?}"
        );
    }

    #[test]
    fn bind_becomes_extend() {
        let q = format!("{GM}SELECT ?k WHERE {{ ?a a purrdf:T . BIND(\"x\" AS ?k) }}");
        let where_pat = unproject(select_pattern(&q));
        let GraphPattern::Extend { variable, .. } = where_pat else {
            panic!("expected Extend, got {where_pat:?}");
        };
        assert_eq!(variable, Variable::new("k"));
    }

    #[test]
    fn property_path_zero_or_more() {
        let q = format!("{GM}SELECT ?x WHERE {{ ?x rdfs:subClassOf* purrdf:C . }}");
        let where_pat = unproject(select_pattern(&q));
        let GraphPattern::Path { path, .. } = where_pat else {
            panic!("expected Path, got {where_pat:?}");
        };
        assert!(matches!(path, PropertyPathExpression::ZeroOrMore(_)));
    }

    #[test]
    fn sequence_path_with_star() {
        // `owl:members/rdf:rest*/rdf:first` — Sequence containing a ZeroOrMore.
        let q = format!("{GM}SELECT ?x WHERE {{ ?d purrdf:members/rdf:rest*/rdf:first ?x . }}");
        let where_pat = unproject(select_pattern(&q));
        assert!(
            matches!(
                where_pat,
                GraphPattern::Path {
                    path: PropertyPathExpression::Sequence(..),
                    ..
                }
            ),
            "got {where_pat:?}"
        );
    }

    #[test]
    fn rdf_collection_in_object_desugars_to_first_rest_chain() {
        // `?s purrdf:members ( purrdf:a purrdf:b purrdf:c )` desugars to the standard
        // rdf:first/rdf:rest blank-node chain (SPARQL §19.5 Collection). The members
        // predicate binds to the HEAD blank; three rdf:first edges carry the elements;
        // three rdf:rest edges link the chain and terminate it with rdf:nil.
        let q =
            format!("{GM}SELECT ?s WHERE {{ ?s purrdf:members ( purrdf:a purrdf:b purrdf:c ) }}");
        let GraphPattern::Bgp { patterns } = unproject(select_pattern(&q)) else {
            panic!("expected BGP");
        };
        // 1 members edge + 3 rdf:first + 3 rdf:rest = 7 triples. The desugaring emits
        // the REAL rdf: IRIs (not the test's mock `rdf:` prefix binding).
        assert_eq!(patterns.len(), 7, "got {patterns:?}");
        let first = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
        let rest = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
        let nil = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";
        let pred = |p: &TriplePattern| match &p.predicate {
            NamedNodePattern::NamedNode(n) => n.as_str().to_owned(),
            other @ NamedNodePattern::Variable(_) => panic!("unexpected predicate {other:?}"),
        };
        assert_eq!(patterns.iter().filter(|p| pred(p) == first).count(), 3);
        assert_eq!(patterns.iter().filter(|p| pred(p) == rest).count(), 3);
        assert_eq!(
            patterns
                .iter()
                .filter(|p| matches!(&p.object, TermPattern::NamedNode(n) if n.as_str() == nil))
                .count(),
            1,
            "exactly one rdf:nil terminator"
        );
        // The members triple's object is the chain head (a blank node).
        let members = patterns
            .iter()
            .find(|p| pred(p).ends_with("members"))
            .expect("members edge present");
        assert!(
            matches!(members.object, TermPattern::BlankNode(_)),
            "members object is the collection head blank"
        );
    }

    #[test]
    fn filter_not_exists() {
        let q = format!(
            "{GM}SELECT ?a WHERE {{ ?a a purrdf:T . FILTER NOT EXISTS {{ ?a purrdf:bad ?x }} }}"
        );
        let where_pat = unproject(select_pattern(&q));
        let GraphPattern::Filter { expr, .. } = where_pat else {
            panic!("expected Filter, got {where_pat:?}");
        };
        assert!(matches!(expr, Expression::Not(inner) if matches!(*inner, Expression::Exists(_))));
    }

    #[test]
    fn group_by_with_count_aggregate() {
        let q = format!(
            "{GM}SELECT ?m (COUNT(?c) AS ?n) WHERE {{ ?c purrdf:vantage ?m . }} GROUP BY ?m"
        );
        let where_pat = unproject(select_pattern(&q));
        // After §18.2: ... Extend(?n = synth) over Group{aggregates:[(synth, COUNT ?c)]}.
        let GraphPattern::Extend {
            inner, variable, ..
        } = where_pat
        else {
            panic!("expected Extend, got {where_pat:?}");
        };
        assert_eq!(variable, Variable::new("n"));
        let GraphPattern::Group {
            variables,
            aggregates,
            ..
        } = *inner
        else {
            panic!("expected Group under Extend");
        };
        assert_eq!(variables, vec![Variable::new("m")]);
        assert_eq!(aggregates.len(), 1);
        assert!(matches!(
            aggregates[0].1,
            AggregateExpression::FunctionCall {
                function: AggregateFunction::Count,
                ..
            }
        ));
    }

    #[test]
    fn order_by_desc_aggregate_lifts_into_group() {
        // SPARQL 1.1 §11.3: ORDER BY on an aggregate is legal inside a grouped
        // query. This was previously rejected as `Unsupported` because ORDER BY
        // used `parse_expression()` (aggregate-blind) instead of the agg-lifting
        // path. Regression guard for gap G4-A.
        let q = format!(
            "{GM}SELECT ?t (COUNT(?x) AS ?c) WHERE {{ ?x a ?t }} GROUP BY ?t ORDER BY DESC(COUNT(?x))"
        );
        let where_pat = unproject(select_pattern(&q));
        // Expected algebra (outermost to innermost, modulo ORDER BY wrapper):
        //   OrderBy { order: [Desc(...)], inner: Extend { var: ?c, inner: Group { aggs: [...] } } }
        let GraphPattern::OrderBy {
            inner,
            expression: order,
        } = where_pat
        else {
            panic!("expected OrderBy at top of unproject'd pattern, got {where_pat:?}");
        };
        // The order key must be a Desc wrapping a Variable reference to the
        // synthetic aggregate variable (lifted COUNT(?x)).
        assert_eq!(order.len(), 1);
        assert!(
            matches!(order[0], OrderExpression::Desc(_)),
            "ORDER BY DESC must produce Desc variant, got {:?}",
            order[0]
        );
        // Walk down: Extend → Group.
        let GraphPattern::Extend {
            inner: group_inner,
            variable,
            ..
        } = *inner
        else {
            panic!("expected Extend under OrderBy, got {inner:?}");
        };
        assert_eq!(variable, Variable::new("c"));
        let GraphPattern::Group { aggregates, .. } = *group_inner else {
            panic!("expected Group under Extend");
        };
        // The aggregate lifted from ORDER BY DESC(COUNT(?x)) must appear in the
        // Group's aggregate list alongside the SELECT-projected one. There must
        // be at least one COUNT aggregate (the ?c projection); the ORDER BY
        // COUNT(?x) should either reuse or add another.
        assert!(
            !aggregates.is_empty(),
            "Group must have at least one aggregate"
        );
        assert!(aggregates.iter().any(|(_, ae)| matches!(
            ae,
            AggregateExpression::FunctionCall {
                function: AggregateFunction::Count,
                ..
            }
        )));
    }

    #[test]
    fn filter_in_list() {
        let q = format!(
            "{GM}SELECT ?p WHERE {{ ?f purrdf:pol ?p . FILTER(?p IN (purrdf:a, purrdf:b)) }}"
        );
        let where_pat = unproject(select_pattern(&q));
        let GraphPattern::Filter { expr, .. } = where_pat else {
            panic!("expected Filter, got {where_pat:?}");
        };
        let Expression::In(_, list) = expr else {
            panic!("expected IN, got {expr:?}");
        };
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn construct_form() {
        let q = format!("{GM}CONSTRUCT {{ ?s a purrdf:Out }} WHERE {{ ?s a purrdf:In }}");
        let Query::Construct { template, .. } = parse(&q) else {
            panic!("expected CONSTRUCT");
        };
        assert_eq!(template.len(), 1);
    }

    #[test]
    fn distinct_and_order_by_and_slice() {
        let q = format!(
            "{GM}SELECT DISTINCT ?a WHERE {{ ?a a purrdf:T }} ORDER BY ?a LIMIT 5 OFFSET 2"
        );
        let p = select_pattern(&q);
        // Distinct wraps Project; Slice is the outermost? Order: Project → Distinct → Slice.
        let GraphPattern::Slice {
            inner,
            start,
            length,
        } = p
        else {
            panic!("expected Slice outermost, got {p:?}");
        };
        assert_eq!(start, 2);
        assert_eq!(length, Some(5));
        assert!(matches!(*inner, GraphPattern::Distinct { .. }));
    }

    #[test]
    fn select_star_collects_visible_vars() {
        let q = format!("{GM}SELECT * WHERE {{ ?a purrdf:p ?b . }}");
        let GraphPattern::Project { variables, .. } = select_pattern(&q) else {
            panic!("expected Project");
        };
        assert_eq!(variables, vec![Variable::new("a"), Variable::new("b")]);
    }

    #[test]
    fn from_clause_parses_into_query_dataset() {
        let q = format!(
            "{GM}SELECT ?a FROM <http://g/> FROM NAMED <http://n/> WHERE {{ ?a a purrdf:T }}"
        );
        let Query::Select { dataset, .. } = parse(&q) else {
            panic!("expected SELECT");
        };
        assert_eq!(dataset.default.len(), 1);
        assert_eq!(dataset.default[0].as_str(), "http://g/");
        assert_eq!(dataset.named.len(), 1);
        assert_eq!(dataset.named[0].as_str(), "http://n/");
    }

    #[test]
    fn no_dataset_clause_is_empty() {
        let q = format!("{GM}SELECT ?a WHERE {{ ?a a purrdf:T }}");
        let Query::Select { dataset, .. } = parse(&q) else {
            panic!("expected SELECT");
        };
        assert!(dataset.default.is_empty() && dataset.named.is_empty());
    }

    #[test]
    fn undeclared_prefix_is_syntax_error() {
        let err = SparqlParser::new()
            .parse_query("SELECT ?a WHERE { ?a a nope:T }")
            .unwrap_err();
        assert!(matches!(err, ParseError::Syntax { .. }), "got {err:?}");
    }

    #[test]
    fn trailing_tokens_rejected() {
        let q =
            format!("{GM}SELECT ?a WHERE {{ ?a a purrdf:T }} SELECT ?b WHERE {{ ?b a purrdf:U }}");
        assert!(SparqlParser::new().parse_query(&q).is_err());
    }

    #[test]
    fn trailing_values_clause_is_accepted() {
        // §18.2.4.3: a `VALUES DataBlock` after the WHERE / solution modifiers,
        // both at the top level and on a SubSelect.
        let q =
            format!("{GM}SELECT ?a WHERE {{ ?a a purrdf:T }} VALUES ?a {{ purrdf:x purrdf:y }}");
        assert!(
            matches!(parse(&q), Query::Select { .. }),
            "trailing top-level VALUES must parse"
        );
        let q2 = format!(
            "{GM}SELECT ?s ?o WHERE {{ {{ SELECT * WHERE {{ ?s ?p ?o }} VALUES (?o) {{ (purrdf:b) }} }} }}"
        );
        assert!(
            matches!(parse(&q2), Query::Select { .. }),
            "trailing VALUES on a sub-select must parse"
        );
    }
    #[test]
    fn custom_function_arg_aggregate_reaches_group() {
        // G3 regression: `purrdf:fn(COUNT(?x))` was discarding the COUNT into a
        // throwaway Vec rather than threading it through to the Group.  The
        // algebra must have a Group whose aggregates list is non-empty.
        let q =
            format!("{GM}SELECT ?t (purrdf:fn(COUNT(?x)) AS ?n) WHERE {{ ?x a ?t }} GROUP BY ?t");
        let where_pat = unproject(select_pattern(&q));
        // Outermost is Extend (for the AS ?n binding).
        let GraphPattern::Extend {
            inner, variable, ..
        } = where_pat
        else {
            panic!("expected Extend, got {where_pat:?}");
        };
        assert_eq!(variable, Variable::new("n"));
        // Inner is the Group node.
        let GraphPattern::Group {
            variables,
            aggregates,
            ..
        } = *inner
        else {
            panic!("expected Group under Extend, got {inner:?}");
        };
        assert_eq!(variables, vec![Variable::new("t")]);
        // The COUNT aggregate must have been collected — not discarded.
        assert_eq!(
            aggregates.len(),
            1,
            "COUNT aggregate was silently discarded (G3); aggregates = {aggregates:?}"
        );
        assert!(
            matches!(
                &aggregates[0].1,
                AggregateExpression::FunctionCall {
                    function: AggregateFunction::Count,
                    ..
                }
            ),
            "expected COUNT aggregate, got {:?}",
            aggregates[0].1
        );
    }

    #[test]
    fn aggregate_in_no_group_position_is_unsupported() {
        // An aggregate in a plain FILTER (no GROUP BY) must still be rejected.
        let q = format!("{GM}SELECT ?x WHERE {{ ?x a purrdf:T . FILTER(COUNT(?x) > 0) }}");
        let err = SparqlParser::new().parse_query(&q).unwrap_err();
        assert!(
            matches!(err, ParseError::Unsupported(_)),
            "expected Unsupported for aggregate in filter position, got {err:?}"
        );
    }

    // ── Bounded repetition {n,m} + predicate wildcard (#1010 PurRDF extensions) ──

    fn path_of(q: &str) -> PropertyPathExpression {
        match unproject(select_pattern(q)) {
            GraphPattern::Path { path, .. } => path,
            other => panic!("expected Path, got {other:?}"),
        }
    }

    #[test]
    fn property_path_bounded_range() {
        let q = format!("{GM}SELECT ?x WHERE {{ ?x purrdf:p{{1,3}} ?y . }}");
        assert!(matches!(
            path_of(&q),
            PropertyPathExpression::Range {
                min: 1,
                max: Some(3),
                ..
            }
        ));
    }

    #[test]
    fn property_path_exact_repetition() {
        let q = format!("{GM}SELECT ?x WHERE {{ ?x purrdf:p{{2}} ?y . }}");
        assert!(matches!(
            path_of(&q),
            PropertyPathExpression::Range {
                min: 2,
                max: Some(2),
                ..
            }
        ));
    }

    #[test]
    fn property_path_at_least_n() {
        let q = format!("{GM}SELECT ?x WHERE {{ ?x purrdf:p{{2,}} ?y . }}");
        assert!(matches!(
            path_of(&q),
            PropertyPathExpression::Range {
                min: 2,
                max: None,
                ..
            }
        ));
    }

    #[test]
    fn property_path_range_round_trips_through_display() {
        let q = format!("{GM}SELECT ?x WHERE {{ ?x purrdf:p{{1,3}} ?y . }}");
        let path = path_of(&q);
        assert_eq!(path.to_string(), "<https://x/p>{1,3}");
        // Re-parse the serialized surface → the same algebra node.
        let q2 = format!("{GM}SELECT ?x WHERE {{ ?x {path} ?y . }}");
        assert_eq!(path_of(&q2), path);
    }

    #[test]
    fn property_path_inverted_range_is_a_hard_error() {
        let q = format!("{GM}SELECT ?x WHERE {{ ?x purrdf:p{{2,1}} ?y . }}");
        let err = SparqlParser::new().parse_query(&q).unwrap_err();
        assert!(
            err.to_string().contains("exceeds upper bound"),
            "expected a min>max hard error, got {err}"
        );
    }

    #[test]
    fn property_path_empty_range_is_a_hard_error() {
        let q = format!("{GM}SELECT ?x WHERE {{ ?x purrdf:p{{}} ?y . }}");
        let err = SparqlParser::new().parse_query(&q).unwrap_err();
        assert!(
            err.to_string().contains("empty path range"),
            "expected an empty-range hard error, got {err}"
        );
    }

    #[test]
    fn property_path_both_bounds_absent_range_is_a_hard_error() {
        // `{,}` with BOTH bounds absent must hard-fail — it is NOT a silent `*`.
        let q = format!("{GM}SELECT ?x WHERE {{ ?x purrdf:p{{,}} ?y . }}");
        let err = SparqlParser::new().parse_query(&q).unwrap_err();
        assert!(
            err.to_string().contains("empty path range {,}"),
            "expected a {{,}} hard error, got {err}"
        );
    }

    #[test]
    fn property_path_partial_bounds_still_parse() {
        // `{n}`, `{n,}`, `{,m}`, `{n,m}` must all still succeed.
        let cases = [
            (
                format!("{GM}SELECT ?x WHERE {{ ?x purrdf:p{{2}} ?y . }}"),
                "<https://x/p>{2}",
            ),
            (
                format!("{GM}SELECT ?x WHERE {{ ?x purrdf:p{{1,}} ?y . }}"),
                "<https://x/p>{1,}",
            ),
            (
                format!("{GM}SELECT ?x WHERE {{ ?x purrdf:p{{,2}} ?y . }}"),
                "<https://x/p>{0,2}",
            ),
            (
                format!("{GM}SELECT ?x WHERE {{ ?x purrdf:p{{1,3}} ?y . }}"),
                "<https://x/p>{1,3}",
            ),
        ];
        for (q, expected_display) in &cases {
            let path = path_of(q);
            assert_eq!(
                path.to_string(),
                *expected_display,
                "path range failed to parse correctly for input: {q}"
            );
        }
    }

    #[test]
    fn property_path_unterminated_range_is_a_hard_error() {
        let q = format!("{GM}SELECT ?x WHERE {{ ?x purrdf:p{{1 ?y . }}");
        assert!(
            SparqlParser::new().parse_query(&q).is_err(),
            "an unterminated path range must hard-fail"
        );
    }

    #[test]
    fn predicate_wildcard_serializes_emit_only() {
        // The wildcard is emit-only (no parse surface), per LOGIC-PATHS.md.
        let any = PropertyPathExpression::Wildcard { namespace: None };
        assert_eq!(any.to_string(), "<any>");
        let scoped = PropertyPathExpression::Wildcard {
            namespace: Some(NamedNode::new_unchecked("https://x/org/")),
        };
        assert_eq!(scoped.to_string(), "<any:https://x/org/>");
    }

    #[test]
    fn star_over_grouped_sequence_round_trips_with_parens() {
        // Display must re-parenthesize a compound operand under a postfix operator.
        let q = format!("{GM}SELECT ?x WHERE {{ ?x (purrdf:p/purrdf:q){{1,2}} ?y . }}");
        let path = path_of(&q);
        assert_eq!(path.to_string(), "(<https://x/p>/<https://x/q>){1,2}");
        let q2 = format!("{GM}SELECT ?x WHERE {{ ?x {path} ?y . }}");
        assert_eq!(path_of(&q2), path);
    }

    // CR6: postfix quantifier over an inverse path must parenthesize the inverse
    // so that Display + re-parse preserves the original AST.
    //
    // Before the fix `ZeroOrMore(Reverse(p))` serialised as `^<p>*`, which
    // reparses as `Reverse(ZeroOrMore(p))` — the nesting is inverted.  The
    // corrected form is `(^<p>)*`.

    #[test]
    fn zero_or_more_over_inverse_round_trips_with_parens() {
        // Parse `(^purrdf:p)*`  →  ZeroOrMore(Reverse(NamedNode(p)))
        let q = format!("{GM}SELECT ?x WHERE {{ ?x (^purrdf:p)* ?y . }}");
        let path = path_of(&q);
        assert_eq!(path.to_string(), "(^<https://x/p>)*");
        // Re-parse the serialised surface — must give the identical algebra node.
        let q2 = format!("{GM}SELECT ?x WHERE {{ ?x {path} ?y . }}");
        assert_eq!(path_of(&q2), path);
    }

    #[test]
    fn one_or_more_over_inverse_round_trips_with_parens() {
        let q = format!("{GM}SELECT ?x WHERE {{ ?x (^purrdf:p)+ ?y . }}");
        let path = path_of(&q);
        assert_eq!(path.to_string(), "(^<https://x/p>)+");
        let q2 = format!("{GM}SELECT ?x WHERE {{ ?x {path} ?y . }}");
        assert_eq!(path_of(&q2), path);
    }

    #[test]
    fn zero_or_one_over_inverse_round_trips_with_parens() {
        let q = format!("{GM}SELECT ?x WHERE {{ ?x (^purrdf:p)? ?y . }}");
        let path = path_of(&q);
        assert_eq!(path.to_string(), "(^<https://x/p>)?");
        let q2 = format!("{GM}SELECT ?x WHERE {{ ?x {path} ?y . }}");
        assert_eq!(path_of(&q2), path);
    }

    #[test]
    fn range_over_inverse_round_trips_with_parens() {
        let q = format!("{GM}SELECT ?x WHERE {{ ?x (^purrdf:p){{1,2}} ?y . }}");
        let path = path_of(&q);
        assert_eq!(path.to_string(), "(^<https://x/p>){1,2}");
        let q2 = format!("{GM}SELECT ?x WHERE {{ ?x {path} ?y . }}");
        assert_eq!(path_of(&q2), path);
    }

    // ── SPARQL 1.1 Update parsing ─────────────────────────────────────────────

    fn parse_update(u: &str) -> Update {
        SparqlParser::new()
            .parse_update(&format!("{GM}{u}"))
            .expect("update parse")
    }

    fn update_err(u: &str) -> ParseError {
        SparqlParser::new()
            .parse_update(&format!("{GM}{u}"))
            .expect_err("update should fail")
    }

    #[test]
    fn update_insert_data() {
        let u = parse_update("INSERT DATA { purrdf:s purrdf:p purrdf:o }");
        assert_eq!(u.operations.len(), 1);
        let GraphUpdateOperation::InsertData { data } = &u.operations[0] else {
            panic!("expected InsertData, got {:?}", u.operations[0]);
        };
        assert_eq!(data.len(), 1);
        assert_eq!(data[0].graph, None);
    }

    #[test]
    fn update_insert_data_with_graph() {
        let u = parse_update("INSERT DATA { GRAPH purrdf:g { purrdf:s purrdf:p purrdf:o } }");
        let GraphUpdateOperation::InsertData { data } = &u.operations[0] else {
            panic!("expected InsertData");
        };
        assert_eq!(data.len(), 1);
        assert_eq!(
            data[0].graph,
            Some(NamedNodePattern::NamedNode(NamedNode::new_unchecked(
                "https://x/g"
            )))
        );
    }

    #[test]
    fn update_insert_data_quoted_triple() {
        // RDF 1.2 INSERT DATA with a quoted-triple object survives as a TermPattern.
        let u =
            parse_update("INSERT DATA { purrdf:s rdf:reifies <<( purrdf:a purrdf:b purrdf:c )>> }");
        let GraphUpdateOperation::InsertData { data } = &u.operations[0] else {
            panic!("expected InsertData");
        };
        assert_eq!(data.len(), 1);
        assert!(matches!(data[0].triple.object, TermPattern::Triple(_)));
    }

    #[test]
    fn update_insert_data_blank_node_is_allowed() {
        // Blank nodes ARE standard in INSERT DATA (§3.1.1, minted fresh per request).
        let u = parse_update("INSERT DATA { [] purrdf:p purrdf:o }");
        let GraphUpdateOperation::InsertData { data } = &u.operations[0] else {
            panic!("expected InsertData");
        };
        assert_eq!(data.len(), 1);
        assert!(matches!(data[0].triple.subject, TermPattern::BlankNode(_)));
    }

    #[test]
    fn update_reused_blank_label_across_operations_is_rejected() {
        // §19.6: a blank node label is scoped to one operation — sharing `_:b1`
        // across two INSERT DATA operations of a request is illegal (vendored
        // W3C `syntax-update-1` `syntax-update-54`).
        let err = update_err(
            "INSERT DATA { _:b1 purrdf:p purrdf:o } ; INSERT DATA { _:b1 purrdf:p purrdf:o }",
        );
        assert!(
            matches!(err, ParseError::Syntax { .. }),
            "expected Syntax for reused blank label across operations, got {err:?}"
        );
        // The same label WITHIN one operation is fine (one blank node), and a
        // fresh label per operation is fine.
        parse_update("INSERT DATA { _:b1 purrdf:p _:b1 } ; INSERT DATA { _:b2 purrdf:p purrdf:o }");
    }

    #[test]
    fn update_reused_blank_label_inside_quoted_triple_across_operations_is_rejected() {
        // §19.6 still applies when the blank label is nested inside an RDF 1.2
        // quoted triple term: reusing `_:b` across two INSERT DATA operations is
        // illegal even though the label never appears at top level. This exercises
        // the `TermPattern::Triple` descent in `collect_term_bnode_labels`.
        let err = update_err(concat!(
            "INSERT DATA { purrdf:s rdf:reifies <<( _:b purrdf:p purrdf:o )>> } ; ",
            "INSERT DATA { purrdf:s rdf:reifies <<( _:b purrdf:p purrdf:o )>> }",
        ));
        assert!(
            matches!(err, ParseError::Syntax { .. }),
            "expected Syntax for reused blank label inside quoted triple across operations, got {err:?}"
        );
    }

    #[test]
    fn update_blank_label_inside_quoted_triple_within_one_operation_is_allowed() {
        // The same blank label confined to a single operation is one blank node —
        // nesting it inside a quoted triple must not trigger a false rejection.
        parse_update(concat!(
            "INSERT DATA { purrdf:s rdf:reifies <<( _:b purrdf:p _:b )>> } ; ",
            "INSERT DATA { purrdf:s rdf:reifies <<( _:c purrdf:p purrdf:o )>> }",
        ));
    }

    #[test]
    fn update_insert_data_labeled_blank_node_is_allowed() {
        let u = parse_update("INSERT DATA { _:b purrdf:p purrdf:o }");
        let GraphUpdateOperation::InsertData { data } = &u.operations[0] else {
            panic!("expected InsertData");
        };
        assert!(matches!(data[0].triple.subject, TermPattern::BlankNode(_)));
    }

    #[test]
    fn update_blank_in_insert_data_quoted_triple_is_allowed() {
        // A blank node nested inside a quoted triple in INSERT DATA is still allowed.
        let u = parse_update("INSERT DATA { purrdf:s rdf:reifies <<( _:b purrdf:p purrdf:o )>> }");
        let GraphUpdateOperation::InsertData { data } = &u.operations[0] else {
            panic!("expected InsertData");
        };
        assert_eq!(data.len(), 1);
    }

    #[test]
    fn update_delete_data() {
        let u = parse_update("DELETE DATA { purrdf:s purrdf:p purrdf:o }");
        assert!(matches!(
            u.operations[0],
            GraphUpdateOperation::DeleteData { .. }
        ));
    }

    #[test]
    fn update_delete_where() {
        let u = parse_update("DELETE WHERE { ?s purrdf:p ?o }");
        let GraphUpdateOperation::DeleteInsert {
            delete,
            insert,
            pattern,
            ..
        } = &u.operations[0]
        else {
            panic!("expected DeleteInsert");
        };
        assert_eq!(delete.len(), 1);
        assert!(insert.is_empty());
        // The template IS the where pattern.
        assert!(matches!(**pattern, GraphPattern::Bgp { .. }));
    }

    #[test]
    fn update_delete_insert_modify() {
        let u = parse_update(
            "DELETE { ?s purrdf:p ?o } INSERT { ?s purrdf:q ?o } WHERE { ?s purrdf:p ?o }",
        );
        let GraphUpdateOperation::DeleteInsert {
            delete,
            insert,
            with,
            using,
            ..
        } = &u.operations[0]
        else {
            panic!("expected DeleteInsert");
        };
        assert_eq!(delete.len(), 1);
        assert_eq!(insert.len(), 1);
        assert!(with.is_none());
        assert!(using.is_empty());
    }

    #[test]
    fn update_insert_only_modify() {
        let u = parse_update("INSERT { ?s purrdf:q purrdf:o } WHERE { ?s a purrdf:T }");
        let GraphUpdateOperation::DeleteInsert { delete, insert, .. } = &u.operations[0] else {
            panic!("expected DeleteInsert");
        };
        assert!(delete.is_empty());
        assert_eq!(insert.len(), 1);
    }

    #[test]
    fn update_with_modify() {
        let u = parse_update(
            "WITH purrdf:g DELETE { ?s purrdf:p ?o } INSERT { ?s purrdf:q ?o } WHERE { ?s purrdf:p ?o }",
        );
        let GraphUpdateOperation::DeleteInsert { with, .. } = &u.operations[0] else {
            panic!("expected DeleteInsert");
        };
        assert_eq!(*with, Some(NamedNode::new_unchecked("https://x/g")));
    }

    #[test]
    fn update_using_clauses() {
        let u = parse_update(
            "DELETE { ?s purrdf:p ?o } USING purrdf:g1 USING NAMED purrdf:g2 WHERE { ?s purrdf:p ?o }",
        );
        let GraphUpdateOperation::DeleteInsert { using, .. } = &u.operations[0] else {
            panic!("expected DeleteInsert");
        };
        assert_eq!(using.len(), 2);
        // The NAMED modifier is preserved (USING <g1> vs USING NAMED <g2>).
        assert!(matches!(&using[0], UsingClause::Default(n) if n.as_str() == "https://x/g1"));
        assert!(matches!(&using[1], UsingClause::Named(n) if n.as_str() == "https://x/g2"));
    }

    #[test]
    fn update_load() {
        let u = parse_update("LOAD <http://src/data> INTO GRAPH purrdf:g");
        let GraphUpdateOperation::Load {
            silent,
            source,
            destination,
        } = &u.operations[0]
        else {
            panic!("expected Load");
        };
        assert!(!silent);
        assert_eq!(source.as_str(), "http://src/data");
        assert_eq!(
            *destination,
            GraphTarget::Named(NamedNode::new_unchecked("https://x/g"))
        );
    }

    #[test]
    fn update_load_silent_default_destination() {
        let u = parse_update("LOAD SILENT <http://src/data>");
        let GraphUpdateOperation::Load {
            silent,
            destination,
            ..
        } = &u.operations[0]
        else {
            panic!("expected Load");
        };
        assert!(silent);
        assert_eq!(*destination, GraphTarget::Default);
    }

    #[test]
    fn update_clear_each_target() {
        for (text, expected) in [
            ("CLEAR DEFAULT", GraphTarget::Default),
            ("CLEAR NAMED", GraphTarget::NamedGraphs),
            ("CLEAR ALL", GraphTarget::All),
            (
                "CLEAR GRAPH purrdf:g",
                GraphTarget::Named(NamedNode::new_unchecked("https://x/g")),
            ),
        ] {
            let u = parse_update(text);
            let GraphUpdateOperation::Clear { target, .. } = &u.operations[0] else {
                panic!("expected Clear for {text}");
            };
            assert_eq!(*target, expected, "target mismatch for {text}");
        }
    }

    #[test]
    fn update_drop() {
        let u = parse_update("DROP SILENT GRAPH purrdf:g");
        let GraphUpdateOperation::Drop { silent, target } = &u.operations[0] else {
            panic!("expected Drop");
        };
        assert!(silent);
        assert_eq!(
            *target,
            GraphTarget::Named(NamedNode::new_unchecked("https://x/g"))
        );
    }

    #[test]
    fn update_create() {
        let u = parse_update("CREATE GRAPH purrdf:g");
        let GraphUpdateOperation::Create { graph, .. } = &u.operations[0] else {
            panic!("expected Create");
        };
        assert_eq!(graph.as_str(), "https://x/g");
    }

    #[test]
    fn update_add_move_copy() {
        let add = parse_update("ADD DEFAULT TO GRAPH purrdf:g");
        assert!(matches!(
            add.operations[0],
            GraphUpdateOperation::Add { .. }
        ));
        let mv = parse_update("MOVE GRAPH purrdf:a TO GRAPH purrdf:b");
        assert!(matches!(
            mv.operations[0],
            GraphUpdateOperation::Move { .. }
        ));
        let cp = parse_update("COPY GRAPH purrdf:a TO DEFAULT");
        let GraphUpdateOperation::Copy {
            source,
            destination,
            ..
        } = &cp.operations[0]
        else {
            panic!("expected Copy");
        };
        assert_eq!(
            *source,
            GraphTarget::Named(NamedNode::new_unchecked("https://x/a"))
        );
        assert_eq!(*destination, GraphTarget::Default);
    }

    #[test]
    fn update_sequence_of_operations() {
        let u = parse_update("CREATE GRAPH purrdf:g ; CLEAR DEFAULT ;");
        assert_eq!(u.operations.len(), 2, "trailing ; must be allowed");
    }

    #[test]
    fn update_empty_request_is_valid() {
        let u = SparqlParser::new()
            .parse_update("PREFIX ex: <http://e/>")
            .expect("prologue-only update");
        assert!(u.operations.is_empty());
    }

    #[test]
    fn update_base_iri_resolves_prologue() {
        let u = SparqlParser::new()
            .with_base_iri("http://base/")
            .parse_update("INSERT DATA { <s> <http://base/p> <o> }")
            .expect("base-resolved update");
        let GraphUpdateOperation::InsertData { data } = &u.operations[0] else {
            panic!("expected InsertData");
        };
        assert_eq!(
            data[0].triple.subject,
            TermPattern::NamedNode(NamedNode::new_unchecked("http://base/s"))
        );
    }

    #[test]
    fn update_blank_in_delete_data_is_error() {
        let err = update_err("DELETE DATA { _:b purrdf:p purrdf:o }");
        assert!(matches!(err, ParseError::Syntax { .. }), "got {err:?}");
    }

    #[test]
    fn update_blank_in_delete_template_is_error() {
        let err = update_err("DELETE { _:b purrdf:p ?o } WHERE { ?s purrdf:p ?o }");
        assert!(matches!(err, ParseError::Syntax { .. }), "got {err:?}");
    }

    #[test]
    fn update_variable_in_insert_data_is_error() {
        let err = update_err("INSERT DATA { purrdf:s purrdf:p ?o }");
        assert!(matches!(err, ParseError::Syntax { .. }), "got {err:?}");
    }

    #[test]
    fn update_unknown_keyword_is_error() {
        let err = update_err("FROBNICATE GRAPH purrdf:g");
        assert!(matches!(err, ParseError::Syntax { .. }), "got {err:?}");
    }

    #[test]
    fn inverse_over_zero_or_more_stays_distinct_from_zero_or_more_over_inverse() {
        // `^purrdf:p*` parses as Reverse(ZeroOrMore(p)) — the star is inside.
        // Display of Reverse(ZeroOrMore(p)) must remain `^<p>*` (no extra parens
        // needed for Reverse; the inner `ZeroOrMore` is already a named-node-like
        // primary from the `^` perspective).
        let q = format!("{GM}SELECT ?x WHERE {{ ?x ^purrdf:p* ?y . }}");
        let path = path_of(&q);
        assert_eq!(path.to_string(), "^<https://x/p>*");
        let q2 = format!("{GM}SELECT ?x WHERE {{ ?x {path} ?y . }}");
        assert_eq!(path_of(&q2), path);
    }

    // ── expression-valued GROUP BY ───────────────────────────────────────────

    #[test]
    fn group_by_expr_as_lowers_to_extend_under_group() {
        // `GROUP BY (?a + ?a AS ?z)` → Extend(?z := ?a+?a) sits UNDER the Group,
        // whose grouping key is the explicit ?z (no algebra change).
        let q = format!(
            "{GM}SELECT ?z (COUNT(*) AS ?c) WHERE {{ ?r purrdf:a ?a }} GROUP BY (?a + ?a AS ?z)"
        );
        // Strip Project, then the select-expr Extend for ?c, to reach the Group.
        let group = match unproject(select_pattern(&q)) {
            GraphPattern::Extend { inner, .. } => *inner,
            other => other,
        };
        match group {
            GraphPattern::Group {
                inner, variables, ..
            } => {
                assert_eq!(variables, vec![Variable::new("z")]);
                match *inner {
                    GraphPattern::Extend { variable, .. } => {
                        assert_eq!(variable, Variable::new("z"));
                    }
                    other => panic!("expected Extend under Group, got {other:?}"),
                }
            }
            other => panic!("expected Group, got {other:?}"),
        }
    }

    #[test]
    fn group_by_bare_builtin_synthesizes_a_group_var() {
        // `GROUP BY STR(?a)` (no AS) mints a synthetic grouping variable.
        let q = format!("{GM}SELECT (COUNT(*) AS ?c) WHERE {{ ?r purrdf:a ?a }} GROUP BY STR(?a)");
        let group = match unproject(select_pattern(&q)) {
            GraphPattern::Extend { inner, .. } => *inner,
            other => other,
        };
        match group {
            GraphPattern::Group {
                inner, variables, ..
            } => {
                assert_eq!(variables.len(), 1);
                assert!(variables[0].as_str().starts_with("__purrdf_group_"));
                assert!(matches!(*inner, GraphPattern::Extend { .. }));
            }
            other => panic!("expected Group, got {other:?}"),
        }
    }

    #[test]
    fn aggregate_in_group_by_key_is_rejected() {
        // `GROUP BY (SUM(?x) AS ?z)` is illegal — an aggregate cannot be a
        // grouping key. The non-lifting expression parse surfaces it.
        let q = format!("{GM}SELECT ?z WHERE {{ ?r purrdf:a ?x }} GROUP BY (SUM(?x) AS ?z)");
        let err = SparqlParser::new().parse_query(&q).unwrap_err();
        assert!(
            matches!(err, ParseError::Unsupported(_)),
            "expected Unsupported for aggregate in GROUP BY key, got {err:?}"
        );
    }

    #[test]
    fn select_star_with_group_by_is_rejected() {
        // §11.1: `SELECT *` is illegal in an aggregate query (vendored W3C
        // `syntax-query` `syn-bad-01`). Both an explicit GROUP BY and a bare
        // aggregate must trip it.
        let q = format!("{GM}SELECT * {{ ?s ?p ?o }} GROUP BY ?s");
        let err = SparqlParser::new().parse_query(&q).unwrap_err();
        assert!(
            matches!(err, ParseError::Syntax { .. }),
            "expected Syntax for SELECT * with GROUP BY, got {err:?}"
        );
    }

    #[test]
    fn bind_target_already_in_scope_is_rejected() {
        // §19.6: re-binding an in-scope variable via BIND is a hard error
        // (vendored W3C `syntax-query` `syntax-BINDscope6/7/8`). Cover the flat
        // BGP, a preceding nested group, and a preceding UNION.
        for body in [
            "?s purrdf:p ?o . ?s purrdf:q ?o1 . BIND((1 + ?o) AS ?o1)",
            "{ ?s purrdf:p ?o . ?s purrdf:q ?o1 . } BIND((1 + ?o) AS ?o1)",
            "{ { ?s purrdf:p ?Y } UNION { ?s purrdf:p ?Z } } BIND(1 AS ?Y)",
        ] {
            let q = format!("{GM}SELECT * WHERE {{ {body} }}");
            let err = SparqlParser::new()
                .parse_query(&q)
                .expect_err("BIND over in-scope var must fail");
            assert!(
                matches!(err, ParseError::Syntax { .. }),
                "expected Syntax for BIND scope violation in {body:?}, got {err:?}"
            );
        }
        // A BIND target that is genuinely fresh still parses.
        let ok = format!("{GM}SELECT * WHERE {{ ?s purrdf:p ?o . BIND((1 + ?o) AS ?o1) }}");
        SparqlParser::new()
            .parse_query(&ok)
            .expect("fresh BIND target parses");
    }

    #[test]
    fn bind_target_only_in_minus_right_is_allowed() {
        // §18.2.1: a variable occurring only in the right operand of MINUS is
        // NOT in scope in the enclosing group, so binding it via BIND is legal.
        // `?v` appears solely inside the MINUS-right, so `BIND(1 AS ?v)` is fresh.
        let q = format!(
            "{GM}SELECT * WHERE {{ ?s purrdf:p ?o MINUS {{ ?x purrdf:q ?v }} BIND(1 AS ?v) }}"
        );
        SparqlParser::new()
            .parse_query(&q)
            .expect("BIND over a MINUS-right-only var must parse");
    }

    #[test]
    fn select_star_excludes_minus_right_only_vars() {
        // §18.2.1: `SELECT *` must not project variables that occur only in the
        // right operand of MINUS. `?v` is MINUS-right-only, so the projection is
        // exactly {?s, ?o}.
        let q = format!("{GM}SELECT * WHERE {{ ?s purrdf:p ?o MINUS {{ ?x purrdf:q ?v }} }}");
        let GraphPattern::Project { variables, .. } = select_pattern(&q) else {
            panic!("expected a Project wrapper for SELECT *");
        };
        let names: Vec<&str> = variables.iter().map(Variable::as_str).collect();
        assert!(
            names.contains(&"s") && names.contains(&"o"),
            "expected ?s and ?o in projection, got {names:?}"
        );
        assert!(
            !names.contains(&"v") && !names.contains(&"x"),
            "MINUS-right-only vars must not be projected, got {names:?}"
        );
    }

    #[test]
    fn nested_aggregate_stays_rejected() {
        // `SUM(COUNT(?x))` is illegal SPARQL 1.1 (no direct aggregate nesting) and
        // must remain a hard error — a regression guard.
        let q = format!("{GM}SELECT (SUM(COUNT(?x)) AS ?y) WHERE {{ ?r purrdf:a ?x }}");
        let err = SparqlParser::new().parse_query(&q).unwrap_err();
        assert!(
            matches!(err, ParseError::Unsupported(_)),
            "expected Unsupported for nested aggregate, got {err:?}"
        );
    }

    // ── blank-node property lists ─────────────────────────────────────────────

    /// Count the triples in a (possibly Join-wrapped) BGP-only WHERE body.
    fn bgp_triple_count(p: &GraphPattern) -> usize {
        match p {
            GraphPattern::Bgp { patterns } => patterns.len(),
            GraphPattern::Join { left, right } => bgp_triple_count(left) + bgp_triple_count(right),
            _ => 0,
        }
    }

    #[test]
    fn blank_node_property_list_in_object_position() {
        // `?o :hasItem [ rdfs:label ?l ]` → two triples: (?o :hasItem _:b) and
        // (_:b rdfs:label ?l), with a fresh blank node linking them.
        let q = format!("{GM}SELECT * WHERE {{ ?o purrdf:hasItem [ rdfs:label ?l ] }}");
        let body = unproject(select_pattern(&q));
        assert_eq!(bgp_triple_count(&body), 2, "got {body:?}");
    }

    #[test]
    fn blank_node_property_list_standalone_subject() {
        // `[ :p ?o ] .` is a valid standalone subject — one triple (_:b :p ?o).
        let q = format!("{GM}SELECT * WHERE {{ [ purrdf:p ?o ] . }}");
        let body = unproject(select_pattern(&q));
        assert_eq!(bgp_triple_count(&body), 1, "got {body:?}");
    }

    #[test]
    fn blank_node_property_list_multiple_predicates() {
        // `[ :a 1 ; :b 2 ]` emits two triples sharing the fresh blank node.
        let q = format!("{GM}SELECT * WHERE {{ ?s purrdf:has [ purrdf:a 1 ; purrdf:b 2 ] }}");
        let body = unproject(select_pattern(&q));
        // (?s :has _:b), (_:b :a 1), (_:b :b 2) = three triples.
        assert_eq!(bgp_triple_count(&body), 3, "got {body:?}");
    }

    // ── empty anonymous blank node [] ─────────────────────────────────────────

    #[test]
    fn empty_blank_node_in_subject_position_parses() {
        // `[] <p> <o>` — SPARQL ANON with no property list in subject position.
        let q = format!("{GM}ASK {{ [] purrdf:p <http://ex/o> }}");
        SparqlParser::new()
            .parse_query(&q)
            .expect("[] in subject position should parse without error");
    }

    #[test]
    fn empty_blank_node_in_object_position_parses() {
        // `<s> <p> []` — SPARQL ANON with no property list in object position.
        let q = format!("{GM}ASK {{ <http://ex/s> purrdf:p [] }}");
        SparqlParser::new()
            .parse_query(&q)
            .expect("[] in object position should parse without error");
    }

    #[test]
    fn non_empty_blank_node_property_list_still_parses() {
        // Regression guard: a non-empty `[ :p :o ]` must continue to work after
        // the empty-[] fix.
        let q = format!("{GM}ASK {{ <http://ex/s> purrdf:p [ purrdf:q <http://ex/o> ] }}");
        SparqlParser::new()
            .parse_query(&q)
            .expect("non-empty blank-node property list should still parse");
    }

    // ── extension-function seam (caller-configured; OFF by default) ───────────

    /// A caller-configured extension-function namespace for these tests (a
    /// neutral example.org name — purrdf itself mints no vocabulary IRIs).
    const EXT_NS: &str = "https://example.org/ext/";

    /// A prologue binding `g:` to the test extension namespace.
    const EXTP: &str = "PREFIX g: <https://example.org/ext/>\n";

    /// Options with only [`EXT_NS`] configured.
    fn ext_options() -> ParserOptions {
        ParserOptions {
            extension_fn_namespaces: vec![EXT_NS.to_owned()],
        }
    }

    /// Parse a SELECT with explicit options and return its root pattern.
    fn select_pattern_with(q: &str, options: &ParserOptions) -> GraphPattern {
        match SparqlParser::new()
            .parse_query_with(q, options)
            .expect("parse")
        {
            Query::Select { pattern, .. } => pattern,
            other => panic!("expected SELECT, got {other:?}"),
        }
    }

    /// Pull the single `BIND(... AS ?v)` expression out of a parsed SELECT.
    fn bound_expr_with(q: &str, options: &ParserOptions) -> Expression {
        let GraphPattern::Extend { expression, .. } = unproject(select_pattern_with(q, options))
        else {
            panic!("expected Extend");
        };
        expression
    }

    /// [`bound_expr_with`] under the default (no extension namespaces) options.
    fn bound_expr(q: &str) -> Expression {
        bound_expr_with(q, &ParserOptions::default())
    }

    /// The expected `heldIn` call node for a given namespace spelling.
    fn held_in_call(ns: &str) -> Function {
        Function::Purrdf(PurrdfCall {
            fn_kind: PurrdfFn::HeldIn,
            iri: format!("{ns}heldIn"),
        })
    }

    #[test]
    fn configured_extension_iri_dispatches_to_the_closed_fn_set() {
        let q = format!("{EXTP}SELECT ?h WHERE {{ ?r ?p ?o . BIND(g:heldIn(?r, ?s) AS ?h) }}");
        let Expression::FunctionCall(func, args) = bound_expr_with(&q, &ext_options()) else {
            panic!("expected a FunctionCall");
        };
        assert_eq!(func, held_in_call(EXT_NS));
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn extension_full_iri_dispatches() {
        // The same dispatch via a full (non-prefixed) IRI under the configured
        // namespace.
        let q =
            "SELECT ?h WHERE { ?r ?p ?o . BIND(<https://example.org/ext/heldIn>(?r, ?s) AS ?h) }";
        let Expression::FunctionCall(func, _) = bound_expr_with(q, &ext_options()) else {
            panic!("expected a FunctionCall");
        };
        assert_eq!(func, held_in_call(EXT_NS));
    }

    #[test]
    fn unknown_extension_function_is_hard_parse_error() {
        let q = format!("{EXTP}SELECT ?x WHERE {{ ?r ?p ?o . BIND(g:bogus(?r) AS ?x) }}");
        let err = SparqlParser::new()
            .parse_query_with(&q, &ext_options())
            .unwrap_err();
        assert!(
            matches!(err, ParseError::Syntax { .. }),
            "unknown g:bogus(...) under a configured namespace must be a hard parse error, got {err:?}"
        );
    }

    #[test]
    fn extension_iri_without_call_is_plain_named_node() {
        // A configured-namespace IRI NOT in call position stays an ordinary IRI term.
        let q = format!("{EXTP}SELECT ?x WHERE {{ ?x a g:heldIn }}");
        let GraphPattern::Bgp { patterns } = unproject(select_pattern_with(&q, &ext_options()))
        else {
            panic!("expected BGP");
        };
        assert_eq!(patterns.len(), 1);
        let TermPattern::NamedNode(n) = &patterns[0].object else {
            panic!("expected a NamedNode object");
        };
        assert_eq!(n.as_str(), "https://example.org/ext/heldIn");
    }

    #[test]
    fn default_options_have_no_extension_namespaces() {
        // With NO configured namespace (the default) the extension seam is OFF:
        // a call-position IRI is an ordinary custom function — no error, no
        // special-casing, regardless of its local name.
        assert!(ParserOptions::default().extension_fn_namespaces.is_empty());
        let q = format!("{EXTP}SELECT ?h WHERE {{ ?r ?p ?o . BIND(g:heldIn(?r, ?s) AS ?h) }}");
        let Expression::FunctionCall(func, _) = bound_expr(&q) else {
            panic!("expected a FunctionCall");
        };
        assert!(
            matches!(&func, Function::Custom(n) if n.as_str() == format!("{EXT_NS}heldIn")),
            "got {func:?}"
        );
    }

    #[test]
    fn non_extension_function_remains_custom() {
        // An IRI outside every configured namespace in call position is
        // Function::Custom even when a namespace IS configured.
        let q = format!("{GM}SELECT ?x WHERE {{ ?r ?p ?o . BIND(purrdf:fn(?r) AS ?x) }}");
        let Expression::FunctionCall(func, _) = bound_expr_with(&q, &ext_options()) else {
            panic!("expected a FunctionCall");
        };
        // `GM` binds `purrdf:` to `<https://x/>`, so this is an external custom IRI.
        assert!(matches!(func, Function::Custom(_)), "got {func:?}");
    }

    // ── configurable extension-function namespaces (ParserOptions) ────────────

    /// The gmeow ontology namespace — the original consumer's spelling of the
    /// same closed extension-function set.
    const GMEOW_NS: &str = "https://blackcatinformatics.ca/gmeow/";

    /// Options with the gmeow namespace configured ALONGSIDE the example one.
    fn gmeow_options() -> ParserOptions {
        ParserOptions {
            extension_fn_namespaces: vec![EXT_NS.to_owned(), GMEOW_NS.to_owned()],
        }
    }

    #[test]
    fn configured_namespace_alias_dispatches_to_purrdf_fn() {
        // gmeow:heldIn(...) dispatches to the SAME closed PurrdfFn set when the gmeow
        // namespace is supplied via ParserOptions.
        let q = format!(
            "PREFIX gmeow: <{GMEOW_NS}>\n\
             SELECT ?h WHERE {{ ?r ?p ?o . BIND(gmeow:heldIn(?r, ?s) AS ?h) }}"
        );
        let Expression::FunctionCall(func, args) = bound_expr_with(&q, &gmeow_options()) else {
            panic!("expected a FunctionCall");
        };
        assert_eq!(func, held_in_call(GMEOW_NS));
        assert_eq!(args.len(), 2);
    }

    #[test]
    fn every_configured_namespace_dispatches() {
        // Configuring several namespaces recognizes each of them.
        let q = format!("{EXTP}SELECT ?h WHERE {{ ?r ?p ?o . BIND(g:listLength(?r) AS ?h) }}");
        let Expression::FunctionCall(func, _) = bound_expr_with(&q, &gmeow_options()) else {
            panic!("expected a FunctionCall");
        };
        assert_eq!(
            func,
            Function::Purrdf(PurrdfCall {
                fn_kind: PurrdfFn::ListLength,
                iri: format!("{EXT_NS}listLength"),
            })
        );
    }

    #[test]
    fn unknown_local_under_configured_alias_is_hard_parse_error() {
        // The closed-set contract applies to EVERY configured namespace: an unknown
        // local name under the gmeow namespace hard-fails, no Custom fallthrough.
        let q = format!(
            "PREFIX gmeow: <{GMEOW_NS}>\n\
             SELECT ?x WHERE {{ ?r ?p ?o . BIND(gmeow:bogus(?r) AS ?x) }}"
        );
        let err = SparqlParser::new()
            .parse_query_with(&q, &gmeow_options())
            .unwrap_err();
        assert!(
            matches!(err, ParseError::Syntax { .. }),
            "unknown gmeow:bogus(...) must be a hard parse error, got {err:?}"
        );
    }

    #[test]
    fn unconfigured_namespace_stays_a_custom_function() {
        // WITHOUT the namespace configured (the default is empty), a gmeow IRI in
        // call position is an ordinary custom function — never an implicit
        // extension dispatch.
        let q = format!(
            "PREFIX gmeow: <{GMEOW_NS}>\n\
             SELECT ?h WHERE {{ ?r ?p ?o . BIND(gmeow:heldIn(?r, ?s) AS ?h) }}"
        );
        let Expression::FunctionCall(func, _) = bound_expr(&q) else {
            panic!("expected a FunctionCall");
        };
        assert!(
            matches!(&func, Function::Custom(n) if n.as_str() == format!("{GMEOW_NS}heldIn")),
            "got {func:?}"
        );
    }

    #[test]
    fn serialization_round_trips_the_original_iri() {
        // ROUND-TRIP: an extension call parsed under the gmeow namespace
        // re-serializes as the ORIGINAL gmeow IRI (no namespace is fabricated on
        // output), and a re-parse with the same options reproduces the same node.
        let q = format!(
            "PREFIX gmeow: <{GMEOW_NS}>\n\
             SELECT ?h WHERE {{ ?r ?p ?o . BIND(gmeow:heldIn(?r, ?s) AS ?h) }}"
        );
        let pattern = select_pattern_with(&q, &gmeow_options());
        let text = crate::serialize::pattern_to_select_query(&pattern);
        assert!(
            text.contains(&format!("<{GMEOW_NS}heldIn>")),
            "serialization must emit the original IRI; text = {text}"
        );
        assert!(
            !text.contains(EXT_NS),
            "no other configured namespace may leak into serialized output; text = {text}"
        );
        let reparsed = find_held_in(&select_pattern_with(&text, &gmeow_options()))
            .unwrap_or_else(|| panic!("re-parse lost the extension dispatch; text = {text}"));
        assert_eq!(reparsed, held_in_call(GMEOW_NS));
    }

    #[test]
    fn extension_serialize_round_trips() {
        let q = format!("{EXTP}SELECT ?h WHERE {{ ?r ?p ?o . BIND(g:heldIn(?r, ?s) AS ?h) }}");
        let pattern = select_pattern_with(&q, &ext_options());
        let text = crate::serialize::pattern_to_select_query(&pattern);
        // The serialized query must still re-parse to the same HeldIn dispatch.
        let reparsed_expr = find_held_in(&select_pattern_with(&text, &ext_options()))
            .unwrap_or_else(|| panic!("round-trip lost the extension dispatch; text = {text}"));
        assert_eq!(reparsed_expr, held_in_call(EXT_NS));
    }

    /// Walk a graph pattern for the first `FunctionCall(Function::Purrdf(_), …)`,
    /// returning its `Function`. Tolerant of the exact `Extend`/`Project` nesting the
    /// serializer round-trip produces.
    fn find_held_in(p: &GraphPattern) -> Option<Function> {
        match p {
            GraphPattern::Extend {
                inner, expression, ..
            } => {
                if let Expression::FunctionCall(f @ Function::Purrdf(_), _) = expression {
                    return Some(f.clone());
                }
                find_held_in(inner)
            }
            GraphPattern::Project { inner, .. }
            | GraphPattern::Filter { inner, .. }
            | GraphPattern::Distinct { inner }
            | GraphPattern::Slice { inner, .. }
            | GraphPattern::OrderBy { inner, .. } => find_held_in(inner),
            _ => None,
        }
    }
}

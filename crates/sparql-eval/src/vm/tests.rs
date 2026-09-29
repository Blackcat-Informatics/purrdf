// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The VM against the recursive tree walk it replaces, and the VM at depths the walk
//! could not reach.
//!
//! [`Walker`] is the tree walk, kept here as the reference: one recursive call per
//! node, evaluating operands in the order the operator's semantics names, over the same
//! operator helpers the VM calls. The differential tests run both over every expression
//! attached to a node of every query the repository's SPARQL suites parse, and over
//! generated expressions, on twin contexts, and require the same value or the same
//! error for every row, the same scratch bytes minted, the same charge calls in the
//! same order, and the same random and blank-node state afterwards.

use std::sync::Arc;

use purrdf_core::{BlankScope, DatasetView, RdfDataset, RdfDatasetBuilder, RdfLiteral, TermValue};
use purrdf_sparql_algebra::{
    ArithmeticOperator, Child, Expression, Function, GraphPattern, Literal, NamedNode,
    NamedNodePattern, NonEmpty, TermPattern, TriplePattern, Variable,
};

use super::compile::is_triple_constructor;
use super::{ExprProgram, Linked};
use crate::DetHashMap;
use crate::error::EvalError;
use crate::eval::EvalCtx;
use crate::expr as helpers;
use crate::scratch::SolutionTerm;
use crate::solution::VarSchema;

const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";
const EX: &str = "http://example.org/";

type Ctx<'d> = EvalCtx<'d, Arc<RdfDataset>>;
type Term = SolutionTerm<<Arc<RdfDataset> as DatasetView>::Id>;

/// The recursive tree walk: the reference the VM is checked against.
#[derive(Default)]
struct Walker {
    /// The walk's constant memo: one intern per constant node, keyed by its address.
    consts: DetHashMap<usize, Option<Term>>,
}

impl Walker {
    fn term(
        &mut self,
        expr: &Expression,
        row: &[Option<Term>],
        schema: &VarSchema,
        ctx: &mut Ctx<'_>,
    ) -> Result<Option<Term>, EvalError> {
        use crate::cdt_fn::CdtRelation;
        use std::cmp::Ordering;
        match expr {
            Expression::NamedNode(n) => {
                Ok(self.constant(ctx, expr, || TermValue::Iri(n.as_str().to_owned())))
            }
            Expression::Literal(l) => {
                Ok(self.constant(ctx, expr, || crate::convert::literal_to_value(l)))
            }
            Expression::Variable(v) => Ok(schema.index_of(v).and_then(|c| row[c])),
            Expression::Bound(v) => {
                let bound = schema.index_of(v).and_then(|c| row[c]).is_some();
                Ok(Some(helpers::bool_term(ctx, bound)))
            }
            Expression::Or(operands) => {
                let mut value = Some(false);
                for operand in operands {
                    value = helpers::kleene_or(value, self.ebv(operand, row, schema, ctx)?);
                }
                Ok(value.map(|b| helpers::bool_term(ctx, b)))
            }
            Expression::And(operands) => {
                let mut value = Some(true);
                for operand in operands {
                    value = helpers::kleene_and(value, self.ebv(operand, row, schema, ctx)?);
                }
                Ok(value.map(|b| helpers::bool_term(ctx, b)))
            }
            Expression::Not(a) => {
                let v = self.ebv(a, row, schema, ctx)?;
                Ok(v.map(|b| helpers::bool_term(ctx, !b)))
            }
            Expression::Equal(a, b) => {
                let ta = self.term(a, row, schema, ctx)?;
                let tb = self.term(b, row, schema, ctx)?;
                Ok(helpers::equal_terms(ctx, ta, tb))
            }
            Expression::Greater(a, b)
            | Expression::GreaterOrEqual(a, b)
            | Expression::Less(a, b)
            | Expression::LessOrEqual(a, b) => {
                let ta = self.term(a, row, schema, ctx)?;
                let tb = self.term(b, row, schema, ctx)?;
                Ok(match expr {
                    Expression::Greater(..) => {
                        helpers::compare_terms(ctx, ta, tb, CdtRelation::Greater, |c| {
                            c == Ordering::Greater
                        })
                    }
                    Expression::GreaterOrEqual(..) => {
                        helpers::compare_terms(ctx, ta, tb, CdtRelation::GreaterOrEqual, |c| {
                            c != Ordering::Less
                        })
                    }
                    Expression::Less(..) => {
                        helpers::compare_terms(ctx, ta, tb, CdtRelation::Less, |c| {
                            c == Ordering::Less
                        })
                    }
                    _ => helpers::compare_terms(ctx, ta, tb, CdtRelation::LessOrEqual, |c| {
                        c != Ordering::Greater
                    }),
                })
            }
            Expression::SameTerm(a, b) => {
                let ta = self.term(a, row, schema, ctx)?;
                let tb = self.term(b, row, schema, ctx)?;
                Ok(match (ta, tb) {
                    (Some(x), Some(y)) => Some(helpers::bool_term(ctx, x == y)),
                    _ => None,
                })
            }
            Expression::If(c, t, e) => match self.ebv(c, row, schema, ctx)? {
                Some(true) => self.term(t, row, schema, ctx),
                Some(false) => self.term(e, row, schema, ctx),
                None => Ok(None),
            },
            Expression::Coalesce(items) => {
                for item in items {
                    if let Some(term) = self.term(item, row, schema, ctx)? {
                        return Ok(Some(term));
                    }
                }
                Ok(None)
            }
            Expression::In(needle, haystack) => {
                let Some(target) = self.term(needle, row, schema, ctx)? else {
                    return Ok(None);
                };
                let tv = helpers::value_of(ctx, target);
                let mut saw_error = false;
                for item in haystack {
                    match self.term(item, row, schema, ctx)? {
                        Some(candidate) => match helpers::in_candidate(ctx, target, &tv, candidate)
                        {
                            Some(true) => return Ok(Some(helpers::bool_term(ctx, true))),
                            Some(false) => {}
                            None => saw_error = true,
                        },
                        None => saw_error = true,
                    }
                }
                Ok(if saw_error {
                    None
                } else {
                    Some(helpers::bool_term(ctx, false))
                })
            }
            Expression::Exists(pattern) => {
                let found = helpers::exists(pattern, row, schema, ctx)?;
                Ok(Some(helpers::bool_term(ctx, found)))
            }
            Expression::Arithmetic(first, steps) => {
                let mut value = self.term(first, row, schema, ctx)?;
                for (op, operand) in steps {
                    let right = self.term(operand, row, schema, ctx)?;
                    value = match (value, right) {
                        (Some(ta), Some(tb)) => helpers::arithmetic_step(ctx, *op, ta, tb),
                        _ => None,
                    };
                }
                Ok(value)
            }
            Expression::UnaryPlus(a) => {
                let ta = self.term(a, row, schema, ctx)?;
                Ok(helpers::unary_numeric_term(
                    ctx,
                    ta,
                    purrdf_xsd::numeric_unary_plus,
                ))
            }
            Expression::UnaryMinus(a) => {
                let ta = self.term(a, row, schema, ctx)?;
                Ok(helpers::unary_numeric_term(
                    ctx,
                    ta,
                    purrdf_xsd::value_unary_minus,
                ))
            }
            Expression::FunctionCall(function, args) => {
                self.function(function, args, row, schema, ctx)
            }
        }
    }

    fn ebv(
        &mut self,
        expr: &Expression,
        row: &[Option<Term>],
        schema: &VarSchema,
        ctx: &mut Ctx<'_>,
    ) -> Result<Option<bool>, EvalError> {
        match self.term(expr, row, schema, ctx)? {
            Some(term) => Ok(helpers::ebv_term(ctx, term)),
            None => Ok(None),
        }
    }

    fn constant(
        &mut self,
        ctx: &mut Ctx<'_>,
        expr: &Expression,
        build: impl FnOnce() -> TermValue,
    ) -> Option<Term> {
        if ctx.in_substituted_exists {
            return helpers::intern_leaf(ctx, build());
        }
        let key = std::ptr::from_ref::<Expression>(expr) as usize;
        if let Some(term) = self.consts.get(&key) {
            return *term;
        }
        let term = helpers::intern_leaf(ctx, build());
        self.consts.insert(key, term);
        term
    }

    fn function(
        &mut self,
        function: &Function,
        args: &[Expression],
        row: &[Option<Term>],
        schema: &VarSchema,
        ctx: &mut Ctx<'_>,
    ) -> Result<Option<Term>, EvalError> {
        match function {
            Function::Contains | Function::StrStarts | Function::StrEnds => {
                let (Some((h, _)), Some((n, _))) = (
                    self.string_arg(args.first(), row, schema, ctx)?,
                    self.string_arg(args.get(1), row, schema, ctx)?,
                ) else {
                    return Ok(None);
                };
                let holds = match function {
                    Function::Contains => h.contains(n.as_str()),
                    Function::StrStarts => h.starts_with(n.as_str()),
                    _ => h.ends_with(n.as_str()),
                };
                return Ok(Some(helpers::bool_term(ctx, holds)));
            }
            Function::Regex => {
                let text = self.string_arg(args.first(), row, schema, ctx)?;
                let pattern = self.string_arg(args.get(1), row, schema, ctx)?;
                let flags = self.string_arg(args.get(2), row, schema, ctx)?;
                let (Some((text, _)), Some((pattern, _))) = (text, pattern) else {
                    return Ok(None);
                };
                let flags = flags.map_or_default(|(f, _)| f);
                return Ok(helpers::cached_regex(ctx, &pattern, &flags)
                    .map(|re| helpers::bool_term(ctx, re.as_regex().is_match(&text))));
            }
            Function::LangMatches => {
                let (Some((tag, _)), Some((range, _))) = (
                    self.string_arg(args.first(), row, schema, ctx)?,
                    self.string_arg(args.get(1), row, schema, ctx)?,
                ) else {
                    return Ok(None);
                };
                return Ok(Some(helpers::bool_term(
                    ctx,
                    helpers::lang_matches(&tag, &range),
                )));
            }
            Function::Triple if args.len() == 3 && is_triple_constructor(&args[2]) => {
                let value = self.triple_value(args, row, schema, ctx)?;
                return Ok(value.and_then(|value| helpers::intern(ctx, value)));
            }
            _ => {}
        }
        let mut vals: Vec<Option<TermValue>> = Vec::with_capacity(args.len());
        for a in args {
            vals.push(
                self.term(a, row, schema, ctx)?
                    .map(|t| helpers::value_of(ctx, t)),
            );
        }
        helpers::apply_function(function, &vals, ctx, None)
    }

    /// The triple term a constructor over `args` builds, uninterned, its object built
    /// the same way when it is a constructor too.
    fn triple_value(
        &mut self,
        args: &[Expression],
        row: &[Option<Term>],
        schema: &VarSchema,
        ctx: &mut Ctx<'_>,
    ) -> Result<Option<TermValue>, EvalError> {
        let subject = self
            .term(&args[0], row, schema, ctx)?
            .map(|t| helpers::value_of(ctx, t));
        let predicate = self
            .term(&args[1], row, schema, ctx)?
            .map(|t| helpers::value_of(ctx, t));
        let object = match &args[2] {
            Expression::FunctionCall(Function::Triple, inner)
                if is_triple_constructor(&args[2]) =>
            {
                self.triple_value(inner, row, schema, ctx)?
            }
            object => self
                .term(object, row, schema, ctx)?
                .map(|t| helpers::value_of(ctx, t)),
        };
        Ok(helpers::triple_value(subject, predicate, object))
    }

    fn string_arg(
        &mut self,
        expr: Option<&Expression>,
        row: &[Option<Term>],
        schema: &VarSchema,
        ctx: &mut Ctx<'_>,
    ) -> Result<Option<(String, Option<String>)>, EvalError> {
        let Some(expr) = expr else {
            return Ok(None);
        };
        match expr {
            Expression::Literal(lit)
                if lit.datatype().as_str() == XSD_STRING
                    || lit.datatype().as_str() == RDF_LANG_STRING =>
            {
                Ok(Some((
                    lit.value().to_owned(),
                    lit.language().map(str::to_ascii_lowercase),
                )))
            }
            Expression::FunctionCall(Function::Str, inner) if inner.len() == 1 => {
                let lexical = match &inner[0] {
                    Expression::NamedNode(node) => Some(node.as_str().to_owned()),
                    Expression::Literal(lit) => Some(lit.value().to_owned()),
                    other => self
                        .term(other, row, schema, ctx)?
                        .and_then(|term| helpers::str_lexical_term(ctx, term)),
                };
                Ok(lexical.map(|s| (s, None)))
            }
            Expression::FunctionCall(Function::Lang, inner) if inner.len() == 1 => {
                let lexical = match &inner[0] {
                    Expression::Literal(lit) => {
                        Some(lit.language().map_or_default(str::to_ascii_lowercase))
                    }
                    other => self
                        .term(other, row, schema, ctx)?
                        .and_then(|term| helpers::lang_lexical_term(ctx, term)),
                };
                Ok(lexical.map(|s| (s, None)))
            }
            _ => {
                let Some(term) = self.term(expr, row, schema, ctx)? else {
                    return Ok(None);
                };
                Ok(helpers::string_arg_of_term(ctx, term))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// the differential harness
// ---------------------------------------------------------------------------

fn dataset() -> Arc<RdfDataset> {
    let mut b = RdfDatasetBuilder::new();
    let p = b.intern_iri(&format!("{EX}p"));
    let q = b.intern_iri(&format!("{EX}q"));
    let a = b.intern_iri(&format!("{EX}a"));
    let c = b.intern_iri(&format!("{EX}c"));
    let thirty = b.intern_literal(RdfLiteral {
        lexical_form: "30".to_owned(),
        datatype: Some(format!("{XSD}integer")),
        language: None,
        direction: None,
    });
    let name = b.intern_literal(RdfLiteral::simple("Ann"));
    b.push_quad(a, p, thirty, None);
    b.push_quad(a, q, name, None);
    b.push_quad(c, p, a, None);
    b.freeze().expect("the fixture freezes")
}

fn literal(lexical: &str, datatype: &str) -> TermValue {
    TermValue::Literal {
        lexical_form: lexical.to_owned(),
        datatype: format!("{XSD}{datatype}"),
        language: None,
        direction: None,
    }
}

/// The values a row's cells are drawn from: dataset terms, computed literals of every
/// kind the operators distinguish, and a blank node.
fn palette() -> Vec<TermValue> {
    vec![
        TermValue::Iri(format!("{EX}a")),
        literal("30", "integer"),
        literal("Ann", "string"),
        TermValue::Literal {
            lexical_form: "chat".to_owned(),
            datatype: RDF_LANG_STRING.to_owned(),
            language: Some("fr".to_owned()),
            direction: None,
        },
        literal("2.5", "decimal"),
        literal("true", "boolean"),
        literal("NaN", "double"),
        TermValue::Iri(format!("{EX}zzz")),
        TermValue::Blank {
            label: "b0".to_owned(),
            scope: BlankScope::DEFAULT,
        },
        literal("-7", "integer"),
        literal("abc", "string"),
    ]
}

/// A context in a fixed state: the same `NOW()`, the same random seed, and the palette
/// interned in the same order.
fn twin<'d>(ds: &'d Arc<RdfDataset>, now: &purrdf_xsd::XsdValue) -> (Ctx<'d>, Vec<Term>) {
    let mut ctx = EvalCtx::at(ds, now.clone(), 0x5EED_1234);
    let terms = palette()
        .into_iter()
        .map(|value| {
            ctx.scratch
                .intern_checked(ctx.dataset, value)
                .expect("the palette's tags are well formed")
        })
        .collect();
    (ctx, terms)
}

/// One evaluation's observable outcome for one row.
fn outcome(ctx: &Ctx<'_>, result: Result<Option<Term>, EvalError>) -> String {
    match result {
        Ok(Some(term)) => format!("{:?}", helpers::value_of(ctx, term)),
        Ok(None) => "unbound".to_owned(),
        Err(error) => format!("error {error:?}"),
    }
}

fn ebv_outcome(result: Result<Option<bool>, EvalError>) -> String {
    match result {
        Ok(value) => format!("{value:?}"),
        Err(error) => format!("error {error:?}"),
    }
}

/// Everything one evaluator did over the rows: each row's outcome, then the context's
/// state afterwards.
#[derive(Debug, PartialEq, Eq)]
struct Run {
    rows: Vec<String>,
    charges: Vec<String>,
    minted: u64,
    rng: u64,
    bnodes: u64,
}

const ROWS: usize = 6;

/// The rows `expr` is evaluated over: every variable it mentions, bound from the
/// palette in a pattern that differs per row and per column, two picks in thirteen
/// unbound.
fn rows_for(expr: &Expression, terms: &[Term]) -> (VarSchema, Vec<Vec<Option<Term>>>) {
    let mut vars = crate::DetHashSet::default();
    helpers::expr_vars(expr, &mut vars);
    let mut vars: Vec<Variable> = vars.into_iter().collect();
    vars.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    let rows = (0..ROWS)
        .map(|r| {
            (0..vars.len())
                .map(|c| {
                    let pick = (r * 7 + c * 3) % (terms.len() + 2);
                    terms.get(pick).copied()
                })
                .collect()
        })
        .collect();
    (VarSchema::from_vars(vars), rows)
}

fn run_vm(expr: &Expression, ebv: bool, ds: &Arc<RdfDataset>, now: &purrdf_xsd::XsdValue) -> Run {
    let (mut ctx, terms) = twin(ds, now);
    let (schema, rows) = rows_for(expr, &terms);
    let base = ctx.scratch.minted_bytes();
    let (outcomes, charges) = super::charge_trace::capture(|| {
        let program = Arc::new(ExprProgram::compile(expr));
        let mut linked = Linked::link(program, expr, &schema, &mut ctx);
        let mut outcomes = Vec::new();
        for (index, row) in rows.iter().enumerate() {
            ctx.current_row = index as u64;
            outcomes.push(if ebv {
                ebv_outcome(linked.ebv(row, &schema, &mut ctx))
            } else {
                let result = linked.term(row, &schema, &mut ctx);
                outcome(&ctx, result)
            });
        }
        outcomes
    });
    Run {
        rows: outcomes,
        charges,
        minted: ctx.scratch.minted_bytes() - base,
        rng: ctx.rng_state,
        bnodes: ctx.bnode_counter,
    }
}

fn run_walker(
    expr: &Expression,
    ebv: bool,
    ds: &Arc<RdfDataset>,
    now: &purrdf_xsd::XsdValue,
) -> Run {
    let (mut ctx, terms) = twin(ds, now);
    let (schema, rows) = rows_for(expr, &terms);
    let base = ctx.scratch.minted_bytes();
    let (outcomes, charges) = super::charge_trace::capture(|| {
        let mut walker = Walker::default();
        let mut outcomes = Vec::new();
        for (index, row) in rows.iter().enumerate() {
            ctx.current_row = index as u64;
            outcomes.push(if ebv {
                ebv_outcome(walker.ebv(expr, row, &schema, &mut ctx))
            } else {
                let result = walker.term(expr, row, &schema, &mut ctx);
                outcome(&ctx, result)
            });
        }
        outcomes
    });
    Run {
        rows: outcomes,
        charges,
        minted: ctx.scratch.minted_bytes() - base,
        rng: ctx.rng_state,
        bnodes: ctx.bnode_counter,
    }
}

/// Run both evaluators over `expr`, as a term and as an effective boolean value, and
/// require the same runs.
fn assert_same(expr: &Expression, context: &str) {
    let ds = dataset();
    let now = purrdf_xsd::XsdValue::DateTime(crate::clock::wall_clock_now());
    for ebv in [false, true] {
        let walked = run_walker(expr, ebv, &ds, &now);
        let run = run_vm(expr, ebv, &ds, &now);
        assert_eq!(run, walked, "{context} (ebv: {ebv}): {expr:?}");
    }
}

#[test]
fn the_vm_matches_the_tree_walk_over_every_suite_expression() {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let suites = manifest.join("../sparql-conformance");
    let vectors = manifest.join("../../vectors");
    let mut files = Vec::new();
    // The conformance suites and corpus, and the vendored SEP-0009 and governor
    // vectors, whose queries are the ones dense in composite-datatype function calls.
    let mut dirs = vec![
        suites.join("suite"),
        suites.join("corpus"),
        vectors.join("sparql-cdt"),
        vectors.join("sparql-governors"),
    ];
    while let Some(dir) = dirs.pop() {
        for entry in std::fs::read_dir(&dir).expect("the conformance suites are present") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rq") {
                files.push(path);
            }
        }
    }
    files.sort();
    let mut compared = 0_usize;
    for file in &files {
        let text = std::fs::read_to_string(file).expect("a query file is UTF-8");
        let Ok(query) = purrdf_sparql_algebra::SparqlParser::new().parse_query(&text) else {
            continue;
        };
        let mut exprs: Vec<&Expression> = Vec::new();
        crate::governor::soundness::walk_spine(
            crate::eval::query_pattern(&query),
            &mut |node, _context, _depth| {
                crate::governor::soundness::visit_pattern_parts(node, &mut |part| {
                    if let crate::governor::soundness::PatternPart::Expression(expr) = part {
                        exprs.push(expr);
                    }
                    false
                });
            },
        );
        for expr in exprs {
            assert_same(expr, &file.display().to_string());
            compared += 1;
        }
    }
    assert!(compared > 500, "the suites hold expressions: {compared}");
}

/// A deterministic choice source for the generator.
struct Choices(u64);

impl Choices {
    fn next(&mut self, bound: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 33) % bound as u64) as usize
    }
}

fn var(name: &str) -> Expression {
    Expression::Variable(Variable::new(name))
}

fn int(value: i64) -> Expression {
    Expression::Literal(Literal::new_typed(
        value.to_string(),
        NamedNode::new_unchecked(format!("{XSD}integer")),
    ))
}

fn boolean(value: bool) -> Expression {
    Expression::Literal(Literal::new_typed(
        value.to_string(),
        NamedNode::new_unchecked(format!("{XSD}boolean")),
    ))
}

fn call(function: Function, args: Vec<Expression>) -> Expression {
    Expression::FunctionCall(function, args.into())
}

fn leaf(choices: &mut Choices) -> Expression {
    match choices.next(12) {
        0 => var("a"),
        1 => var("b"),
        2 => var("c"),
        3 => int(i64::try_from(choices.next(5)).unwrap_or(0) - 1),
        4 => Expression::Literal(Literal::new_simple("Ann")),
        5 => Expression::Literal(Literal::new_lang("chat", "FR", None)),
        6 => Expression::NamedNode(NamedNode::new_unchecked(format!("{EX}a"))),
        7 => boolean(choices.next(2) == 0),
        8 => Expression::Literal(Literal::new_typed(
            "NaN",
            NamedNode::new_unchecked(format!("{XSD}double")),
        )),
        9 => Expression::Bound(Variable::new("b")),
        10 => Expression::Literal(Literal::new_simple("^A")),
        _ => Expression::Literal(Literal::new_typed(
            "2.5",
            NamedNode::new_unchecked(format!("{XSD}decimal")),
        )),
    }
}

fn exists_pattern(choices: &mut Choices) -> GraphPattern {
    let predicate = if choices.next(2) == 0 { "p" } else { "q" };
    GraphPattern::Bgp {
        patterns: vec![TriplePattern {
            subject: TermPattern::Variable(Variable::new("a")),
            predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked(format!(
                "{EX}{predicate}"
            ))),
            object: TermPattern::Variable(Variable::new("o")),
        }],
    }
}

/// A generated expression over `?a`, `?b`, `?c`, of at most `budget` nodes.
fn generate(choices: &mut Choices, budget: &mut usize) -> Expression {
    if *budget == 0 {
        return leaf(choices);
    }
    *budget -= 1;
    let sub = |choices: &mut Choices, budget: &mut usize| generate(choices, budget);
    match choices.next(34) {
        0..=3 => leaf(choices),
        4 => {
            let (a, b) = (sub(choices, budget), sub(choices, budget));
            Expression::or(a, b)
        }
        5 => {
            let (a, b) = (sub(choices, budget), sub(choices, budget));
            let c = sub(choices, budget);
            Expression::and(Expression::and(a, b), c)
        }
        6 => Expression::Not(Child::new(sub(choices, budget))),
        7 => Expression::Equal(
            Child::new(sub(choices, budget)),
            Child::new(sub(choices, budget)),
        ),
        8 => Expression::Less(
            Child::new(sub(choices, budget)),
            Child::new(sub(choices, budget)),
        ),
        9 => Expression::GreaterOrEqual(
            Child::new(sub(choices, budget)),
            Child::new(sub(choices, budget)),
        ),
        10 => Expression::SameTerm(
            Child::new(sub(choices, budget)),
            Child::new(sub(choices, budget)),
        ),
        11 => Expression::If(
            Child::new(sub(choices, budget)),
            Child::new(sub(choices, budget)),
            Child::new(sub(choices, budget)),
        ),
        12 => {
            let items = (0..choices.next(4))
                .map(|_| sub(choices, budget))
                .collect::<Vec<_>>();
            Expression::Coalesce(items.into())
        }
        13 => {
            let needle = sub(choices, budget);
            let items = (0..choices.next(4))
                .map(|_| sub(choices, budget))
                .collect::<Vec<_>>();
            Expression::In(Child::new(needle), items.into())
        }
        14 => {
            let operators = [
                ArithmeticOperator::Add,
                ArithmeticOperator::Subtract,
                ArithmeticOperator::Multiply,
                ArithmeticOperator::Divide,
            ];
            let first = sub(choices, budget);
            let op = operators[choices.next(4)];
            let second = sub(choices, budget);
            Expression::arithmetic(first, op, second)
        }
        15 => Expression::UnaryMinus(Child::new(sub(choices, budget))),
        16 => Expression::UnaryPlus(Child::new(sub(choices, budget))),
        17 => call(Function::Str, vec![sub(choices, budget)]),
        18 => call(Function::Lang, vec![sub(choices, budget)]),
        19 => call(
            Function::Contains,
            vec![sub(choices, budget), sub(choices, budget)],
        ),
        20 => call(
            Function::StrStarts,
            vec![
                call(Function::Str, vec![sub(choices, budget)]),
                call(Function::Lang, vec![sub(choices, budget)]),
            ],
        ),
        21 => {
            let text = sub(choices, budget);
            let pattern = if choices.next(2) == 0 {
                Expression::Literal(Literal::new_simple("^a"))
            } else {
                sub(choices, budget)
            };
            let mut args = vec![text, pattern];
            if choices.next(2) == 0 {
                args.push(Expression::Literal(Literal::new_simple("i")));
            }
            call(Function::Regex, args)
        }
        22 => call(
            Function::Replace,
            vec![
                sub(choices, budget),
                Expression::Literal(Literal::new_simple("n")),
                Expression::Literal(Literal::new_simple("N")),
            ],
        ),
        23 => call(
            Function::LangMatches,
            vec![
                call(Function::Lang, vec![sub(choices, budget)]),
                Expression::Literal(Literal::new_simple("*")),
            ],
        ),
        24 => call(
            Function::Concat,
            vec![sub(choices, budget), sub(choices, budget)],
        ),
        25 => call(Function::StrLen, vec![sub(choices, budget)]),
        26 => call(
            Function::BNode,
            vec![call(Function::Str, vec![sub(choices, budget)])],
        ),
        27 => call(Function::BNode, Vec::new()),
        28 => call(Function::Rand, Vec::new()),
        29 => call(Function::StrUuid, Vec::new()),
        30 => call(
            Function::Custom(NamedNode::new_unchecked(format!("{XSD}integer"))),
            vec![sub(choices, budget)],
        ),
        31 => call(
            Function::Custom(NamedNode::new_unchecked(format!("{EX}unregistered"))),
            vec![sub(choices, budget)],
        ),
        32 => Expression::Exists(Child::new(exists_pattern(choices))),
        _ => call(Function::IsLiteral, vec![sub(choices, budget)]),
    }
}

/// A generated chain of triple term constructors, each level's object the level below:
/// subjects and predicates that are IRIs, blank nodes, variables, literals (a type error,
/// so unbound) and minted blank nodes; an innermost object that may be a language-tagged
/// literal or unbound; and the chain read as a term, tested with `isTRIPLE`, compared, or
/// taken apart with `OBJECT`.
fn triple_chain(choices: &mut Choices) -> Expression {
    let iri = |local: &str| Expression::NamedNode(NamedNode::new_unchecked(format!("{EX}{local}")));
    let mut expr = match choices.next(4) {
        0 => var("c"),
        1 => Expression::Literal(Literal::new_lang("chat", "FR", None)),
        2 => call(Function::BNode, Vec::new()),
        _ => leaf(choices),
    };
    for _ in 0..=choices.next(5) {
        let subject = match choices.next(6) {
            0 => var("a"),
            1 => call(Function::BNode, Vec::new()),
            2 => Expression::Literal(Literal::new_simple("not a subject")),
            _ => iri("s"),
        };
        let predicate = match choices.next(5) {
            0 => var("b"),
            1 => Expression::Literal(Literal::new_simple("not a predicate")),
            _ => iri("p"),
        };
        expr = call(Function::Triple, vec![subject, predicate, expr]);
    }
    match choices.next(4) {
        0 => call(Function::IsTriple, vec![expr]),
        1 => Expression::SameTerm(Child::new(expr.clone()), Child::new(expr)),
        2 => call(Function::Object, vec![expr]),
        _ => expr,
    }
}

/// **A nested triple term constructor evaluates as the tree walk evaluates it**: the same
/// value or error, the same bytes minted — its outermost term alone — the same charges
/// and the same blank-node state.
#[test]
fn the_vm_matches_the_tree_walk_over_nested_triple_constructors() {
    let mut choices = Choices(0x7219_1E5E);
    for index in 0..500 {
        let expr = triple_chain(&mut choices);
        assert_same(&expr, &format!("triple chain {index}"));
    }
}

#[test]
fn the_vm_matches_the_tree_walk_over_generated_expressions() {
    let mut choices = Choices(0x0DD5_EED5);
    for index in 0..2_000 {
        let mut budget = 12;
        let expr = generate(&mut choices, &mut budget);
        assert_same(&expr, &format!("generated expression {index}"));
    }
}

// ---------------------------------------------------------------------------
// depth
// ---------------------------------------------------------------------------

const DEPTH: usize = 100_000;

/// Run `f` on a thread whose stack is 128 KiB: a recursion over a 100 000-deep
/// expression would need far more.
fn on_small_stack<T: Send + 'static>(f: impl FnOnce() -> T + Send + 'static) -> T {
    std::thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(f)
        .expect("the thread spawns")
        .join()
        .expect("the evaluation finishes on a 128 KiB stack")
}

/// Compile, link and run `expr` over the empty row, on a fresh context.
fn evaluate(expr: &Expression) -> Option<TermValue> {
    let ds = RdfDatasetBuilder::new().freeze().expect("an empty dataset");
    let mut ctx = EvalCtx::new(&ds);
    let schema = VarSchema::new();
    let program = Arc::new(ExprProgram::compile(expr));
    let mut linked = Linked::link(program, expr, &schema, &mut ctx);
    let term = linked.term(&[], &schema, &mut ctx).expect("no hard error");
    term.map(|term| helpers::value_of(&ctx, term))
}

#[test]
fn a_deep_arithmetic_chain_evaluates_on_a_small_stack() {
    let value = on_small_stack(|| {
        // `1 + (1 + (1 + … 1))`: the right operand nests, so the tree is DEPTH deep.
        let mut expr = int(1);
        for _ in 0..DEPTH {
            expr = Expression::Arithmetic(
                Child::new(int(1)),
                NonEmpty::new((ArithmeticOperator::Add, expr)),
            );
        }
        evaluate(&expr)
    });
    // DEPTH additions of 1 onto the innermost 1.
    let expected = (DEPTH + 1).to_string();
    assert_eq!(value, Some(literal(&expected, "integer")));
}

/// **A hundred thousand nested triple term constructors build one term, interned once.**
/// Interning every level's whole term — each up to the whole chain — cost the square of
/// the depth in time and memory; the chain is built as one value, and the scratch holds
/// its outermost term, the subject and predicate constants, and nothing else.
#[test]
fn a_deep_triple_constructor_chain_interns_its_outermost_term_once() {
    let (value, computed) = on_small_stack(|| {
        let iri =
            |local: &str| Expression::NamedNode(NamedNode::new_unchecked(format!("{EX}{local}")));
        let mut expr = int(7);
        for _ in 0..DEPTH {
            expr = call(Function::Triple, vec![iri("s"), iri("p"), expr]);
        }
        let ds = RdfDatasetBuilder::new().freeze().expect("an empty dataset");
        let mut ctx = EvalCtx::new(&ds);
        let schema = VarSchema::new();
        let program = Arc::new(ExprProgram::compile(&expr));
        let mut linked = Linked::link(program, &expr, &schema, &mut ctx);
        let term = linked.term(&[], &schema, &mut ctx).expect("no hard error");
        let value = term.map(|term| helpers::value_of(&ctx, term));
        let computed = ctx.scratch.computed_count();
        drop(linked);
        drop(expr);
        (value.map(|value| triple_depth_and_core(&value)), computed)
    });
    assert_eq!(value, Some((DEPTH, literal("7", "integer"))));
    assert_eq!(
        computed, 4,
        "the scratch holds the two IRIs, the integer and the outermost triple term"
    );
}

/// How many triple terms `value` nests, and the innermost object; the value is dropped
/// over a work list, as the term's own drop takes it apart.
fn triple_depth_and_core(value: &TermValue) -> (usize, TermValue) {
    let mut depth = 0;
    let mut at = value;
    while let TermValue::Triple { o, .. } = at {
        depth += 1;
        at = o;
    }
    (depth, at.clone())
}

#[test]
fn a_deep_if_nest_evaluates_on_a_small_stack() {
    let value = on_small_stack(|| {
        // Alternating `IF(true, e, 0)` and `IF(false, 0, e)`: every level takes the
        // branch holding the nest, so the answer is the innermost value.
        let mut expr = int(7);
        for level in 0..DEPTH {
            expr = if level.is_multiple_of(2) {
                Expression::If(
                    Child::new(boolean(true)),
                    Child::new(expr),
                    Child::new(int(0)),
                )
            } else {
                Expression::If(
                    Child::new(boolean(false)),
                    Child::new(int(0)),
                    Child::new(expr),
                )
            };
        }
        evaluate(&expr)
    });
    assert_eq!(value, Some(literal("7", "integer")));
}

#[test]
fn a_deep_coalesce_nest_evaluates_on_a_small_stack() {
    let value = on_small_stack(|| {
        // `COALESCE(?unbound, COALESCE(?unbound, … 5))`: every level's first item is
        // unbound, so every level falls through to the nest and the answer is 5.
        let mut expr = int(5);
        for _ in 0..DEPTH {
            expr = Expression::Coalesce(vec![var("unbound"), expr].into());
        }
        evaluate(&expr)
    });
    assert_eq!(value, Some(literal("5", "integer")));
}

#[test]
fn a_deep_not_nest_evaluates_on_a_small_stack() {
    let value = on_small_stack(|| {
        let mut expr = boolean(true);
        for _ in 0..DEPTH {
            expr = Expression::Not(Child::new(expr));
        }
        evaluate(&expr)
    });
    // An even number of negations of `true` is `true`, an odd number `false`.
    let expected = if DEPTH.is_multiple_of(2) {
        "true"
    } else {
        "false"
    };
    assert_eq!(value, Some(literal(expected, "boolean")));
}

// ---------------------------------------------------------------------------
// linking
// ---------------------------------------------------------------------------

#[test]
fn a_link_resolves_each_variable_once_not_once_per_row() {
    let ds = dataset();
    let mut ctx = EvalCtx::new(&ds);
    let expr = Expression::and(
        Expression::Less(Child::new(var("a")), Child::new(var("b"))),
        Expression::Bound(Variable::new("a")),
    );
    let schema = VarSchema::from_vars([Variable::new("a"), Variable::new("b")]);
    let one = ctx
        .scratch
        .intern_checked(ctx.dataset, literal("1", "integer"));
    let two = ctx
        .scratch
        .intern_checked(ctx.dataset, literal("2", "integer"));
    let program = Arc::new(ExprProgram::compile(&expr));
    VarSchema::reset_index_of_calls();
    let mut linked = Linked::link(program, &expr, &schema, &mut ctx);
    let linked_calls = VarSchema::index_of_call_count();
    for _ in 0..100 {
        assert_eq!(
            linked.ebv(&[one, two], &schema, &mut ctx).expect("ebv"),
            Some(true)
        );
    }
    assert_eq!(linked_calls, 2, "one resolution per distinct variable");
    assert_eq!(
        VarSchema::index_of_call_count(),
        linked_calls,
        "none per row"
    );
}

#[test]
fn a_plan_site_compiles_once_and_every_call_shares_it() {
    let filter = GraphPattern::Filter {
        expr: Expression::Equal(Child::new(var("a")), Child::new(int(1))),
        inner: Child::new(GraphPattern::Bgp {
            patterns: Vec::new(),
        }),
    };
    let GraphPattern::Filter { expr, .. } = &filter else {
        unreachable!("built as a filter");
    };
    let ds = dataset();
    let tree = crate::plan::Tree::build(&filter);
    let mut ctx = EvalCtx::new(&ds);
    ctx.install_plan(&tree);
    let first = super::program_at(&ctx, &filter, expr);
    let second = super::program_at(&ctx, &filter, expr);
    assert!(
        Arc::ptr_eq(&first, &second),
        "the site's program is kept on the plan"
    );

    // A structurally equal expression that is not the node's own is compiled apart.
    let copy = expr.clone();
    let apart = super::program_at(&ctx, &filter, &copy);
    assert!(!Arc::ptr_eq(&first, &apart));
}

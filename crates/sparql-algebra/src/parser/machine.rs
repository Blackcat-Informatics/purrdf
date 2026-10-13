// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The pushdown machine that reads group graph patterns, sub-`SELECT`s, solution
//! modifiers and expressions.
//!
//! These four productions reach one another in a cycle — a `FILTER` holds an
//! expression, an expression holds an `EXISTS` group, a group holds a sub-`SELECT`,
//! whose projection and modifiers hold expressions again — so they share one stack of
//! suspended productions, [`Ctl`], and one loop, [`Parser::run`], drives them all. A
//! production that needs a nested one pushes what it has read so far and hands over; the
//! nested one's value comes back through [`Step::Return`] to whatever is on top. Nothing
//! here calls itself: the depth of the machine's own frames is fixed, and a request's
//! nesting lives in the heap-allocated stacks of [`Machine`], so it is bounded by memory
//! alone and is the same on every host.
//!
//! Expressions are read by operator precedence: one [`Ctl::Act`] marks where an
//! expression's operators begin, every pending prefix or infix operator sits above it
//! with its left operand, and an operator is applied as soon as one binding no tighter
//! arrives. The operators build exactly the nodes the grammar's precedence levels
//! build, through the same constructors, so a chain of `||`, `&&`, `+`/`-` or `*`/`/`
//! is one node however long it is written.
//!
//! The larger states — an open group, a `SELECT` and a solution-modifier list — live in
//! typed stacks beside [`Ctl`], so an operator frame stays small. All of them are
//! [`Parser`] fields reused from one parse of a request to the next, so a shallow query
//! grows each stack once and allocates nothing further for them.

use crate::algebra::{
    AggregateExpression, AggregateFunction, ArithmeticOperator, CdtCall, CdtFn, Expression,
    Function, GraphPattern, OrderExpression, PurrdfCall, PurrdfFn, Query, QueryDataset,
};
use crate::ast::{NamedNode, NamedNodePattern, Variable};
use crate::error::{ParseError, Result};
use crate::lexer::Token;
use crate::tree::Child;

use super::{
    ExistsScopeBasis, Modifiers, Parser, PendingExistsScopeCheck, ScopeConstruct, SelectPosition,
    VarScope, aggregate_function, builtin_function, collect_vars, empty_modifier_clause,
    expect_arity, find_scope_conflict, repeated_bound_clause, split_trailing_filters, stray_dot,
    visible_variables_with_memory,
};

#[cfg(debug_assertions)]
use super::compute_lateral_left_scope;

/// The refusal of an aggregate written where an expression may hold none.
const AGGREGATE_OUTSIDE: &str = "aggregate outside GROUP BY / SELECT / HAVING context";

/// The refusal of an aggregate written as a bare constraint or sort key that may hold
/// none.
const AGGREGATE_HERE: &str = "aggregate in this position";

/// One aggregate call lifted out of an expression: the synthetic variable the call is
/// replaced by, and the aggregation it stands for.
type Lifted = (Variable, AggregateExpression);

/// The first variable `expr` reads, outside any `EXISTS` body, that `readable`
/// rejects — the grouping constraint's witness for a SELECT expression of an
/// aggregate query — or `None` when every variable it reads is readable.
fn first_projection_read<'a, S: purrdf_lex::allocation::Admission + ?Sized>(
    expression: &'a Expression,
    readable: impl Fn(&Variable) -> bool,
    memory: &mut purrdf_lex::allocation::Memory<'_, S>,
) -> std::result::Result<Option<&'a Variable>, purrdf_lex::allocation::StorageError> {
    use crate::walk::NodeRef;
    let mut pending = Vec::new();
    memory.push(&mut pending, NodeRef::Expr(expression))?;
    let mut found = None;
    while let Some(node) = pending.pop() {
        if let NodeRef::Expr(Expression::Variable(variable) | Expression::Bound(variable)) = node
            && !readable(variable)
        {
            found = Some(variable);
            break;
        }
        if !matches!(node, NodeRef::Expr(Expression::Exists(_))) {
            let mut failure = None;
            node.for_each_child(|child| {
                if failure.is_none()
                    && let Err(error) = memory.push(&mut pending, child)
                {
                    failure = Some(error);
                }
            });
            if let Some(failure) = failure {
                return Err(failure);
            }
        }
    }
    memory.release_vec(pending)?;
    Ok(found)
}

/// How much of the expression grammar an activation reads.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Reach {
    /// A whole `Expression`: operators and all.
    Full,
    /// One `PrimaryExpression` and nothing after it: a bare constraint, a bare sort key,
    /// a component of an expression triple term.
    Primary,
}

/// Where the aggregate calls an activation reads go.
#[derive(Clone, Copy)]
enum Sink {
    /// Into the list of the activation it is nested in: a bracket, an argument, an `IN`
    /// list element.
    Shared,
    /// Into a list of its own, which must be empty once the expression ends; otherwise
    /// the expression is refused as [`ParseError::Unsupported`] with this reason.
    Refuse(&'static str),
    /// Into a list of its own, handed with the value to the production that asked for
    /// it (a projection, a `HAVING` constraint, an `ORDER BY` key), which lifts them
    /// into its query's aggregation.
    Lift,
}

/// The base of one expression's operators.
#[derive(Clone, Copy)]
struct Activation {
    reach: Reach,
    sink: Sink,
}

/// A prefix operator: `!`, unary `+`, unary `-`.
#[derive(Clone, Copy)]
enum Prefix {
    Not,
    Plus,
    Minus,
}

impl Prefix {
    fn apply(
        self,
        operand: Expression,
        memory: &mut purrdf_lex::allocation::Memory<'_, dyn super::ParserAdmission + '_>,
    ) -> Result<Expression> {
        let operand = Child::try_new(operand, memory)?;
        Ok(match self {
            Self::Not => Expression::Not(operand),
            Self::Plus => Expression::UnaryPlus(operand),
            Self::Minus => Expression::UnaryMinus(operand),
        })
    }
}

/// An infix operator, pending with its left operand.
#[derive(Clone, Copy)]
enum Infix {
    Or,
    And,
    Equal,
    NotEqual,
    Less,
    Greater,
    LessOrEqual,
    GreaterOrEqual,
    Arithmetic(ArithmeticOperator),
}

/// How tightly a prefix operator binds: tighter than every infix one.
const PREFIX_POWER: u8 = 6;

/// How tightly a relational operator binds.
const RELATIONAL_POWER: u8 = 3;

impl Infix {
    /// How tightly the operator binds: `||` loosest, then `&&`, the relational
    /// operators, `+`/`-`, and `*`/`/` tightest.
    const fn power(self) -> u8 {
        match self {
            Self::Or => 1,
            Self::And => 2,
            Self::Equal
            | Self::NotEqual
            | Self::Less
            | Self::Greater
            | Self::LessOrEqual
            | Self::GreaterOrEqual => RELATIONAL_POWER,
            Self::Arithmetic(op) => {
                if op.is_multiplicative() {
                    5
                } else {
                    4
                }
            }
        }
    }

    const fn is_relational(self) -> bool {
        self.power() == RELATIONAL_POWER
    }

    fn apply(
        self,
        left: Expression,
        right: Expression,
        memory: &mut purrdf_lex::allocation::Memory<'_, dyn super::ParserAdmission + '_>,
    ) -> Result<Expression> {
        let (l, r) = match self {
            Self::Or => return Ok(Expression::or_with_memory(left, right, memory)?),
            Self::And => return Ok(Expression::and_with_memory(left, right, memory)?),
            Self::Arithmetic(op) => {
                return Ok(Expression::arithmetic_with_memory(left, op, right, memory)?);
            }
            _ => (
                Child::try_new(left, memory)?,
                Child::try_new(right, memory)?,
            ),
        };
        Ok(match self {
            Self::Equal => Expression::Equal(l, r),
            // `!=` builds two nodes: `Not(Equal(l, r))`.
            Self::NotEqual => Expression::Not(Child::try_new(Expression::Equal(l, r), memory)?),
            Self::Less => Expression::Less(l, r),
            Self::Greater => Expression::Greater(l, r),
            Self::LessOrEqual => Expression::LessOrEqual(l, r),
            Self::GreaterOrEqual => Expression::GreaterOrEqual(l, r),
            Self::Or | Self::And | Self::Arithmetic(_) => {
                unreachable!("a chain operator is applied above")
            }
        })
    }
}

/// What a call's argument list builds once its `)` is read.
enum ArgsKind {
    /// A SEP-0009 composite-datatype function, arity-checked against its signature.
    Cdt {
        fn_kind: CdtFn,
        iri: String,
        at: usize,
    },
    /// An IRI-named function: an extension function or a custom one.
    Call(Function),
    /// `IF(c, a, b)`.
    If,
    /// `COALESCE(…)`.
    Coalesce,
    /// `sameTerm(a, b)`.
    SameTerm,
    /// A keyword-named built-in.
    Builtin(Function),
}

/// Where a `FOLD(…)` call is.
#[derive(Clone, Copy)]
enum FoldStage {
    /// Reading its first expression.
    First,
    /// Reading its second (`cdt:Map` key/value) expression.
    Second,
    /// Reading an `ASC( … )` sort key.
    Ascending,
    /// Reading a `DESC( … )` sort key.
    Descending,
    /// Reading a bare sort key.
    Bare,
}

/// A `FOLD(…)` call in progress.
struct Fold {
    stage: FoldStage,
    distinct: bool,
    /// [`Parser::in_aggregate_argument`] as it was before the call, restored once its
    /// arguments are read.
    saved: bool,
    args: Vec<Expression>,
    order_by: Vec<OrderExpression>,
}

/// A group element whose operand is being read.
enum Element {
    /// A braced group, or the latest arm of a `UNION` chain.
    Union,
    /// `OPTIONAL { … }`.
    Optional,
    /// `LATERAL { … }`, with the offset of its right-hand side's `{`.
    Lateral { at: usize },
    /// `MINUS { … }`.
    Minus,
    /// `GRAPH name { … }`.
    Graph(NamedNodePattern),
    /// `SERVICE [SILENT] name { … }`.
    Service {
        silent: bool,
        name: NamedNodePattern,
    },
    /// `FILTER constraint`.
    Filter,
    /// `BIND( expression …`.
    Bind,
    /// `UNFOLD( expression …`.
    Unfold,
}

/// Where a `SELECT` is.
#[derive(Clone, Copy)]
enum SelectStage {
    /// Reading a projection's `( expression AS ?v )`.
    Projection,
    /// Reading its `WHERE` group.
    Where,
    /// Reading its solution modifiers.
    Modifiers,
}

/// Where a solution-modifier list is.
#[derive(Clone, Copy)]
enum ModStage {
    /// Reading a `GROUP BY ( expression [AS ?v] )` condition.
    GroupBracketed,
    /// Reading a bare `GROUP BY` call condition.
    GroupBare,
    /// Reading a bracketted `HAVING` constraint.
    HavingBracketed,
    /// Reading a bare `HAVING` constraint.
    HavingBare,
    /// Reading an `ORDER BY ASC( … )` key.
    OrderAscending,
    /// Reading an `ORDER BY DESC( … )` key.
    OrderDescending,
    /// Reading a bare `ORDER BY` key.
    OrderBare,
}

/// One suspended production, waiting for the value of the production it handed over to
/// (or, for [`Self::Act`], [`Self::Prefix`] and [`Self::Infix`], for the operand that
/// completes it).
enum Ctl {
    /// The base of an expression's operators.
    Act(Activation),
    /// A pending prefix operator.
    Prefix(Prefix),
    /// A pending infix operator and its left operand.
    Infix(Infix, Expression),
    /// `( expression )` as an operand: its `)`.
    Bracket,
    /// A call's argument list, with the arguments read so far.
    Args(ArgsKind, Vec<Expression>),
    /// `left [NOT] IN ( … )`, with the elements read so far.
    InList {
        left: Expression,
        negated: bool,
        items: Vec<Expression>,
    },
    /// An expression triple term `<<( s p o )>>`, with the components read so far.
    TripleTerm(Vec<Expression>),
    /// A built-in aggregate's argument.
    Aggregate {
        func: AggregateFunction,
        distinct: bool,
        saved: bool,
    },
    /// `AGG(<iri>, …)`, with the arguments read so far.
    CustomAggregate {
        iri: NamedNode,
        distinct: bool,
        saved: bool,
        args: Vec<Expression>,
    },
    /// `FOLD(…)`.
    Fold(Box<Fold>),
    /// `[NOT] EXISTS`, waiting for its group: the offset of its `{`. The variables in
    /// scope on the row it tests are the top in-scope-set frame's once the body's own
    /// frame is closed, since only the top frame is ever written.
    Exists { at: usize, negated: bool },
    /// A bracketted constraint's `)`.
    Constraint,
    /// `{ SELECT … }`, waiting for the sub-`SELECT`.
    SubSelectGroup,
    /// A group element, waiting for its operand; the group is the top of
    /// [`Machine::groups`].
    Element(Element),
    /// A `SELECT`; its state is the top of [`Machine::selects`].
    Select(SelectStage),
    /// A solution-modifier list; its state is the top of [`Machine::modifiers`].
    Modifiers(ModStage),
}

/// A finished group graph pattern: the pattern, the variables it puts in scope in the
/// group around it, in first-appearance order (what [`collect_vars`] would note walking
/// it), and whether it holds a fresh binding [`find_scope_conflict`] could report.
pub(super) struct GroupValue {
    pub(super) pattern: GraphPattern,
    pub(super) scope: VarScope,
    pub(super) intro: bool,
    /// Filters belonging to this group, rather than a nested group whose
    /// empty-BGP join was simplified. OPTIONAL must lift only its own filters.
    pub(super) filter_count: usize,
}

/// What a finished production hands to the one it was nested in.
#[expect(
    clippy::large_enum_variant,
    reason = "production values stay inline; native buffers admit the exact Val layout before growth without allocating a separate box per value"
)]
enum Val {
    /// An expression, with the aggregates it lifted (empty unless its sink lifts).
    Expr(Expression, Vec<Lifted>),
    /// A group graph pattern.
    Group(GroupValue),
    /// A `SELECT`, and whether it holds a fresh binding [`find_scope_conflict`] could
    /// report.
    Query(Box<Query>, bool),
    /// A solution-modifier list and the query's aggregates, its own appended.
    Modifiers(Modifiers, Vec<Lifted>),
}

impl Val {
    fn expr(self) -> (Expression, Vec<Lifted>) {
        match self {
            Self::Expr(expr, lifted) => (expr, lifted),
            _ => unreachable!("an expression production is resumed with an expression"),
        }
    }

    fn group(self) -> GroupValue {
        match self {
            Self::Group(group) => group,
            _ => unreachable!("a group-bodied production is resumed with a group"),
        }
    }
}

/// What the machine does next.
#[expect(
    clippy::large_enum_variant,
    reason = "the next machine step is an inline value; boxing Return would add a separate allocation outside the native frame-buffer layout"
)]
enum Step {
    /// Read an operand of the innermost expression.
    Operand,
    /// An operand of the innermost expression is complete; read the operator after it.
    /// `true` once a relational level closed with an `IN` list, after which only `&&`
    /// and `||` continue the expression.
    Operator(Expression, bool),
    /// A production finished: hand its value to the top of [`Ctl`].
    Return(Val),
    /// Read the next element of the innermost group.
    Elements,
    /// Read a group graph pattern at its `{` ([`Parser::start_group`]). A `SELECT`'s
    /// `WHERE` group is read through this step rather than by a direct call, because a
    /// group can itself be a sub-`SELECT` whose `WHERE` group is the next one: read by
    /// direct calls, `{ SELECT * WHERE { SELECT * WHERE … } }` took a stack frame per
    /// level.
    Group,
}

/// An open group graph pattern.
struct GroupState {
    /// The pattern the elements read so far build.
    g: GraphPattern,
    /// The `FILTER`s, applied over the whole group once it closes.
    filters: Vec<Expression>,
    /// The variables `g` puts in scope, kept in step with it.
    scope: VarScope,
    /// The basic graph pattern the next triples block extends, while one is open: a
    /// `FILTER` or a `.` leaves it open, every other element closes it.
    open_bgp: Option<usize>,
    /// Whether `g` holds a fresh binding [`find_scope_conflict`] could report.
    intro: bool,
    /// The `UNION` chain being read: its pattern, its scope and its `intro`.
    union: Option<GroupValue>,
    /// Whether a `.` may come next: only straight after a `GraphPatternNotTriples`
    /// element, once (`GroupGraphPatternSub ::= TriplesBlock? ( GraphPatternNotTriples
    /// '.'? TriplesBlock? )*`). A triples block reads its own separating dots.
    dot_ok: bool,
}

impl GroupState {
    fn new() -> Self {
        Self {
            g: GraphPattern::Bgp { patterns: vec![] },
            filters: Vec::new(),
            scope: VarScope::default(),
            open_bgp: None,
            intro: false,
            union: None,
            dot_ok: false,
        }
    }
}

/// A `SELECT` or sub-`SELECT` being read.
struct SelectState {
    position: SelectPosition,
    base_iri: Option<NamedNode>,
    dedup: Dedup,
    /// The enclosing parse's deferred-`EXISTS` window, restored once this `SELECT`
    /// ends (see [`Parser::projection_scope_pending`]).
    saved_pending: bool,
    saved_in_aggregate: bool,
    saved_seen: Vec<Variable>,
    saved_checks: Vec<PendingExistsScopeCheck>,
    star: bool,
    projected: Vec<Variable>,
    select_exprs: Vec<(Variable, Expression)>,
    aggregates: Vec<Lifted>,
    dataset: QueryDataset,
    /// The `WHERE` pattern and its `intro`, once read.
    where_pattern: Option<(GraphPattern, bool)>,
}

/// A `SELECT`'s `DISTINCT` or `REDUCED`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Dedup {
    None,
    Distinct,
    Reduced,
}

/// A solution-modifier list being read, with its query's aggregates.
struct ModState {
    m: Modifiers,
    aggregates: Vec<Lifted>,
}

/// The machine's stacks, kept between parses of one request so they are grown once.
#[derive(Default)]
pub(super) struct Machine {
    ctl: Vec<Ctl>,
    groups: Vec<GroupState>,
    selects: Vec<SelectState>,
    modifiers: Vec<ModState>,
    /// One aggregate list per expression whose sink is not [`Sink::Shared`].
    sinks: Vec<Vec<Lifted>>,
}

impl Machine {
    pub(super) fn release<S: purrdf_lex::allocation::Admission + ?Sized>(
        self,
        memory: &mut purrdf_lex::allocation::Memory<'_, S>,
    ) -> std::result::Result<(), purrdf_lex::allocation::StorageError> {
        debug_assert!(
            self.ctl.is_empty()
                && self.groups.is_empty()
                && self.selects.is_empty()
                && self.modifiers.is_empty()
                && self.sinks.is_empty()
        );
        memory.release_vec(self.ctl)?;
        memory.release_vec(self.groups)?;
        memory.release_vec(self.selects)?;
        memory.release_vec(self.modifiers)?;
        memory.release_vec(self.sinks)
    }
}

impl<const RDFLIB: bool> Parser<'_, '_, '_, '_, RDFLIB> {
    // ── entry points ─────────────────────────────────────────────────────────

    /// Read a braced group graph pattern.
    pub(super) fn parse_group(&mut self) -> Result<GroupValue> {
        let step = self.start_group()?;
        Ok(self.run(step)?.group())
    }

    /// Read a group graph pattern, discarding what it puts in scope.
    pub(super) fn parse_group_graph_pattern(&mut self) -> Result<GraphPattern> {
        let group = self.parse_group()?;
        group.scope.release(self.memory)?;
        Ok(group.pattern)
    }

    /// Read a whole query's `SELECT`.
    pub(super) fn parse_select(&mut self, base_iri: Option<NamedNode>) -> Result<Query> {
        let step = self.start_select(base_iri, SelectPosition::Query)?;
        match self.run(step)? {
            Val::Query(query, _) => {
                let query = *query;
                self.memory.release_bytes(size_of::<Query>())?;
                Ok(query)
            }
            _ => unreachable!("a SELECT returns a query"),
        }
    }

    /// Read a solution-modifier list, appending the aggregates it lifts to
    /// `aggregates`.
    pub(super) fn parse_solution_modifiers(
        &mut self,
        aggregates: &mut Vec<Lifted>,
    ) -> Result<Modifiers> {
        let step = self.start_modifiers(std::mem::take(aggregates))?;
        match self.run(step)? {
            Val::Modifiers(modifiers, lifted) => {
                *aggregates = lifted;
                Ok(modifiers)
            }
            _ => unreachable!("a solution-modifier list returns modifiers"),
        }
    }

    /// Drive the machine from `step` until the production at the bottom of the stack
    /// returns its value.
    fn run(&mut self, mut step: Step) -> Result<Val> {
        loop {
            step = match step {
                Step::Operand => self.operand()?,
                Step::Operator(value, closed) => self.operator(value, closed)?,
                Step::Elements => self.elements()?,
                Step::Group => self.start_group()?,
                Step::Return(val) => match self.machine.ctl.pop() {
                    None => return Ok(val),
                    Some(ctl) => self.resume(ctl, val)?,
                },
            };
        }
    }

    /// Hand `val` to the production `ctl` suspended.
    fn resume(&mut self, ctl: Ctl, val: Val) -> Result<Step> {
        match ctl {
            Ctl::Bracket => {
                let (expr, _) = val.expr();
                self.expect(&Token::RParen)?;
                Ok(Step::Operator(expr, false))
            }
            Ctl::Args(kind, mut args) => {
                {
                    let native_value = val.expr().0;
                    self.memory.push(&mut args, native_value)?;
                };
                if self.eat(&Token::Comma) {
                    {
                        let native_value = Ctl::Args(kind, args);
                        self.memory.push(&mut self.machine.ctl, native_value)?;
                    };
                    return self.activate(Reach::Full, Sink::Shared);
                }
                self.expect(&Token::RParen)?;
                self.finish_call(kind, args)
            }
            Ctl::InList {
                left,
                negated,
                mut items,
            } => {
                {
                    let native_value = val.expr().0;
                    self.memory.push(&mut items, native_value)?;
                };
                if self.eat(&Token::Comma) {
                    {
                        let native_value = Ctl::InList {
                            left,
                            negated,
                            items,
                        };
                        self.memory.push(&mut self.machine.ctl, native_value)?;
                    };
                    return self.activate(Reach::Full, Sink::Shared);
                }
                self.expect(&Token::RParen)?;
                self.finish_in(left, negated, items)
            }
            Ctl::TripleTerm(mut parts) => {
                {
                    let native_value = val.expr().0;
                    self.memory.push(&mut parts, native_value)?;
                };
                if parts.len() < 3 {
                    {
                        let native_value = Ctl::TripleTerm(parts);
                        self.memory.push(&mut self.machine.ctl, native_value)?;
                    };
                    return self.activate(Reach::Primary, Sink::Shared);
                }
                self.expect(&Token::RParen)?;
                self.expect(&Token::TripleClose)?;
                Ok(Step::Operator(
                    Expression::FunctionCall(Function::Triple, parts.into()),
                    false,
                ))
            }
            Ctl::Aggregate {
                func,
                distinct,
                saved,
            } => {
                self.in_aggregate_argument = saved;
                let inner = val.expr().0;
                let mut scalarvals = Vec::new();
                if matches!(func, AggregateFunction::GroupConcat)
                    && let Some(sep) = self.parse_optional_separator()?
                {
                    {
                        let name = self.memory.string("separator")?;
                        let datatype = self.named_node(purrdf_xsd::datatype::XSD_STRING)?;
                        let literal = self.literal(&sep, datatype, None, None)?;
                        self.memory.release_string(sep)?;
                        let native_value = (name, literal);
                        self.memory.push(&mut scalarvals, native_value)?;
                    };
                }
                let agg = AggregateExpression::new(
                    func,
                    self.memory.collect([inner])?,
                    scalarvals,
                    Vec::new(),
                    distinct,
                )
                .expect("a one-element args list is always a valid AggregateExpression");
                self.finish_aggregate(agg)
            }
            Ctl::CustomAggregate {
                iri,
                distinct,
                saved,
                mut args,
            } => {
                {
                    let native_value = val.expr().0;
                    self.memory.push(&mut args, native_value)?;
                };
                if self.eat(&Token::Comma) {
                    {
                        let native_value = Ctl::CustomAggregate {
                            iri,
                            distinct,
                            saved,
                            args,
                        };
                        self.memory.push(&mut self.machine.ctl, native_value)?;
                    };
                    return self.activate(Reach::Full, Sink::Refuse(AGGREGATE_OUTSIDE));
                }
                self.in_aggregate_argument = saved;
                let scalarvals = self.parse_agg_scalarvals()?;
                let agg = AggregateExpression::new(
                    AggregateFunction::Custom(iri),
                    args,
                    scalarvals,
                    Vec::new(),
                    distinct,
                )
                .expect("a custom aggregate call holds at least one argument");
                self.finish_aggregate(agg)
            }
            Ctl::Fold(fold) => self.resume_fold(fold, val.expr().0),
            Ctl::Exists { at, negated } => {
                let body = val.group();
                self.pop_exists_scope_boundary();
                let body = self.check_exists_body(at, body)?;
                let exists = Expression::Exists(Child::try_new(body, self.memory)?);
                Ok(Step::Operator(
                    if negated {
                        Expression::Not(Child::try_new(exists, self.memory)?)
                    } else {
                        exists
                    },
                    false,
                ))
            }
            Ctl::Constraint => {
                let (expr, lifted) = val.expr();
                self.expect(&Token::RParen)?;
                Ok(Step::Return(Val::Expr(expr, lifted)))
            }
            Ctl::SubSelectGroup => {
                let Val::Query(sub, intro) = val else {
                    unreachable!("a sub-SELECT returns a query")
                };
                self.expect(&Token::RBrace)?;
                Ok(Step::Return(Val::Group({
                    let sub = *sub;
                    self.memory.release_bytes(size_of::<Query>())?;
                    self.sub_select_group(sub, intro)?
                })))
            }
            Ctl::Element(element) => self.resume_element(element, val),
            Ctl::Select(stage) => self.resume_select(stage, val),
            Ctl::Modifiers(stage) => self.resume_modifiers(stage, val),
            Ctl::Act(_) | Ctl::Prefix(_) | Ctl::Infix(..) => {
                unreachable!("an operator is completed by an operand, never by a returned value")
            }
        }
    }

    // ── expressions ──────────────────────────────────────────────────────────

    /// Open an expression activation and read its first operand.
    fn activate(&mut self, reach: Reach, sink: Sink) -> Result<Step> {
        if !matches!(sink, Sink::Shared) {
            {
                let native_value = Vec::new();
                self.memory.push(&mut self.machine.sinks, native_value)?;
            };
        }
        {
            let native_value = Ctl::Act(Activation { reach, sink });
            self.memory.push(&mut self.machine.ctl, native_value)?;
        };
        Ok(Step::Operand)
    }

    /// Read an operand: any prefix operators before it (in a whole expression), then a
    /// primary expression.
    fn operand(&mut self) -> Result<Step> {
        let primary_only = matches!(
            self.machine.ctl.last(),
            Some(Ctl::Act(Activation {
                reach: Reach::Primary,
                ..
            }))
        );
        if !primary_only {
            loop {
                let prefix = match self
                    .tokens
                    .get(self.pos)
                    .and_then(Option::as_ref)
                    .map(|token| &token.token)
                {
                    Some(Token::Bang) => Prefix::Not,
                    Some(Token::Plus) => Prefix::Plus,
                    Some(Token::Minus) => Prefix::Minus,
                    _ => break,
                };
                self.pos += 1;
                {
                    let native_value = Ctl::Prefix(prefix);
                    self.memory.push(&mut self.machine.ctl, native_value)?;
                };
            }
        }
        self.primary()
    }

    /// Read a primary expression: a leaf is an operand at once; a construct that holds
    /// expressions is suspended and its first one is read.
    fn primary(&mut self) -> Result<Step> {
        match self
            .tokens
            .get(self.pos)
            .and_then(Option::as_ref)
            .map(|token| &token.token)
        {
            Some(Token::LParen) => {
                self.pos += 1;
                {
                    let native_value = Ctl::Bracket;
                    self.memory.push(&mut self.machine.ctl, native_value)?;
                };
                self.activate(Reach::Full, Sink::Shared)
            }
            Some(Token::Variable(_)) => Ok(Step::Operator(
                Expression::Variable(self.expect_var()?),
                false,
            )),
            Some(Token::Iri(_) | Token::PrefixedName(_, _)) => self.iri_or_function(),
            Some(
                Token::StringLit(_)
                | Token::LongStringLit(_)
                | Token::Integer(_)
                | Token::Decimal(_)
                | Token::Double(_),
            ) => Ok(Step::Operator(
                Expression::Literal(self.parse_literal()?),
                false,
            )),
            Some(Token::TripleOpen) => self.triple_term_expr(),
            Some(Token::Word(w)) => {
                let w = *w;
                if super::boolean_keyword(w).is_some() {
                    // No sign precedes a bare boolean word here: a leading `+`/`-` is read
                    // as a prefix operator before a primary is reached, so `parse_literal`
                    // observes none and takes its boolean arm.
                    Ok(Step::Operator(
                        Expression::Literal(self.parse_literal()?),
                        false,
                    ))
                } else {
                    self.builtin_or_aggregate(w)
                }
            }
            other => Err(super::native_syntax(
                &format_args!("expected an expression, found {other:?}"),
                self.span(),
                self.memory,
            )),
        }
    }

    /// The operand `value` is complete: read the operator after it, applying every
    /// pending operator that binds at least as tightly first, or end the expression.
    fn operator(&mut self, value: Expression, closed: bool) -> Result<Step> {
        if matches!(
            self.machine.ctl.last(),
            Some(Ctl::Act(Activation {
                reach: Reach::Primary,
                ..
            }))
        ) {
            return self.finish_activation(value);
        }
        let infix = match self
            .tokens
            .get(self.pos)
            .and_then(Option::as_ref)
            .map(|token| &token.token)
        {
            Some(Token::Or) => Some(Infix::Or),
            Some(Token::And) => Some(Infix::And),
            _ if closed => None,
            Some(Token::Eq) => Some(Infix::Equal),
            Some(Token::NotEq) => Some(Infix::NotEqual),
            Some(Token::Lt) => Some(Infix::Less),
            Some(Token::Gt) => Some(Infix::Greater),
            Some(Token::LtEq) => Some(Infix::LessOrEqual),
            Some(Token::GtEq) => Some(Infix::GreaterOrEqual),
            Some(Token::Plus) => Some(Infix::Arithmetic(ArithmeticOperator::Add)),
            Some(Token::Minus) => Some(Infix::Arithmetic(ArithmeticOperator::Subtract)),
            Some(Token::Star) => Some(Infix::Arithmetic(ArithmeticOperator::Multiply)),
            Some(Token::Slash) => Some(Infix::Arithmetic(ArithmeticOperator::Divide)),
            _ => None,
        };
        if let Some(op) = infix {
            if op.is_relational() {
                let left = self.reduce(value, RELATIONAL_POWER + 1)?;
                // One relational operator per relational level: a second ends the
                // expression, and whatever reads it next reports the token.
                if self.relational_pending() {
                    return self.finish_activation(left);
                }
                self.pos += 1;
                {
                    let native_value = Ctl::Infix(op, left);
                    self.memory.push(&mut self.machine.ctl, native_value)?;
                };
                return Ok(Step::Operand);
            }
            let left = self.reduce(value, op.power())?;
            self.pos += 1;
            {
                let native_value = Ctl::Infix(op, left);
                self.memory.push(&mut self.machine.ctl, native_value)?;
            };
            return Ok(Step::Operand);
        }
        if !closed {
            let negated = if self.peek_kw("IN") {
                Some(false)
            } else if self.peek_kw("NOT") && self.peek2_kw("IN") {
                Some(true)
            } else {
                None
            };
            if let Some(negated) = negated {
                let left = self.reduce(value, RELATIONAL_POWER + 1)?;
                if self.relational_pending() {
                    return self.finish_activation(left);
                }
                self.pos += if negated { 2 } else { 1 };
                return self.in_list(left, negated);
            }
        }
        self.finish_activation(value)
    }

    /// Whether the innermost pending operator is relational.
    fn relational_pending(&self) -> bool {
        matches!(self.machine.ctl.last(), Some(Ctl::Infix(op, _)) if op.is_relational())
    }

    /// Apply every pending operator of the innermost expression that binds at least
    /// `min` tightly to `value`, innermost first.
    fn reduce(&mut self, mut value: Expression, min: u8) -> Result<Expression> {
        loop {
            let binds = match self.machine.ctl.last() {
                Some(Ctl::Prefix(_)) => PREFIX_POWER >= min,
                Some(Ctl::Infix(op, _)) => op.power() >= min,
                _ => false,
            };
            if !binds {
                return Ok(value);
            }
            value = match self.machine.ctl.pop() {
                Some(Ctl::Prefix(prefix)) => prefix.apply(value, self.memory)?,
                Some(Ctl::Infix(op, left)) => op.apply(left, value, self.memory)?,
                _ => unreachable!("only a pending operator binds"),
            };
        }
    }

    /// End the innermost expression with `value`: apply its pending operators, close its
    /// activation, and settle the aggregates it read.
    fn finish_activation(&mut self, value: Expression) -> Result<Step> {
        let value = self.reduce(value, 0)?;
        let Some(Ctl::Act(activation)) = self.machine.ctl.pop() else {
            unreachable!("an expression's operators rest on its activation")
        };
        let lifted = match activation.sink {
            Sink::Shared => Vec::new(),
            Sink::Refuse(reason) => {
                let aggregates = self
                    .machine
                    .sinks
                    .pop()
                    .expect("an activation with its own sink pushed it");
                if !aggregates.is_empty() {
                    return Err(self.unsupported(&reason));
                }
                aggregates
            }
            Sink::Lift => self
                .machine
                .sinks
                .pop()
                .expect("an activation with its own sink pushed it"),
        };
        Ok(Step::Return(Val::Expr(value, lifted)))
    }

    /// `left [NOT] IN`, read: its bracketted list.
    fn in_list(&mut self, left: Expression, negated: bool) -> Result<Step> {
        self.expect(&Token::LParen)?;
        if self.at(&Token::RParen) {
            self.expect(&Token::RParen)?;
            return self.finish_in(left, negated, Vec::new());
        }
        {
            let native_value = Ctl::InList {
                left,
                negated,
                items: Vec::new(),
            };
            self.memory.push(&mut self.machine.ctl, native_value)?;
        };
        self.activate(Reach::Full, Sink::Shared)
    }

    /// `left [NOT] IN ( items )`, whose `)` was read: it closes its relational level.
    fn finish_in(
        &mut self,
        left: Expression,
        negated: bool,
        items: Vec<Expression>,
    ) -> Result<Step> {
        let expr = Expression::In(Child::try_new(left, self.memory)?, items.into());
        Ok(Step::Operator(
            if negated {
                Expression::Not(Child::try_new(expr, self.memory)?)
            } else {
                expr
            },
            true,
        ))
    }

    /// Human-readable spelling of a SEP-0009 signature, for
    /// [`ParseError::CdtArity`].
    fn describe_cdt_arity(&mut self, arity: crate::algebra::CdtArity) -> Result<String> {
        use crate::algebra::CdtArity;
        Ok(match arity {
            CdtArity::Fixed(1) => self.memory.string("exactly 1 argument")?,
            CdtArity::Fixed(n) => self.memory.format(&format_args!("exactly {n} arguments"))?,
            CdtArity::Range { min, max } => self
                .memory
                .format(&format_args!("{min} to {max} arguments"))?,
            CdtArity::AtLeast(0) => self.memory.string("any number of arguments")?,
            CdtArity::AtLeast(min) => self
                .memory
                .format(&format_args!("at least {min} arguments"))?,
            CdtArity::Pairs => self
                .memory
                .string("an even number of arguments (key/value pairs)")?,
        })
    }

    /// An IRI in expression position: a function call when `(` follows, the IRI
    /// otherwise.
    fn iri_or_function(&mut self) -> Result<Step> {
        let node = self.expect_iri_node()?;
        if !self.at(&Token::LParen) {
            return Ok(Step::Operator(Expression::NamedNode(node), false));
        }
        // A SEP-0009 composite-datatype function, by EXACT IRI match against the closed
        // `CdtFn` registry. Checked FIRST and UNCONDITIONALLY: the spec fixes both the
        // namespace and the local names, so there is no `ParserOptions` seam here and a
        // configured extension namespace can never shadow one of these. Recognizing a
        // spec-defined third-party IRI is not minting it — see `CdtCall`'s own docs.
        if let Some(fn_kind) = CdtFn::from_iri(node.as_str()) {
            let at = self.span();
            let iri = self.memory.string(node.as_str())?;
            return self.call_args(ArgsKind::Cdt { fn_kind, iri, at });
        }
        // An IRI in call position under ANY configured extension-function namespace
        // (default: NONE — the namespace set is caller configuration supplied via
        // ParserOptions) dispatches to the CLOSED extension-function seam, recognized
        // here at parse time. The local-name MUST resolve; an unknown <ns>foo(...) under
        // a configured namespace is a hard error (fail-fast), never a silent
        // Function::Custom fallthrough. An IRI under NO configured namespace stays
        // Function::Custom. The original IRI is recorded in the AST node so
        // serialization round-trips exactly.
        let ext_local = self
            .options
            .extension_fn_namespaces
            .iter()
            .find_map(|ns| node.as_str().strip_prefix(ns.as_str()));
        let func = if let Some(local) = ext_local {
            match PurrdfFn::from_local_name(local) {
                Some(fn_kind) => Function::Purrdf(PurrdfCall {
                    fn_kind,
                    iri: self.memory.string(node.as_str())?,
                }),
                None => {
                    return Err(super::native_syntax(
                        &format_args!("unknown extension function <{}>", node.as_str()),
                        self.span(),
                        self.memory,
                    ));
                }
            }
        } else {
            Function::Custom(node)
        };
        self.call_args(ArgsKind::Call(func))
    }

    /// A call's `( … )` argument list: its first argument, or the call itself when the
    /// list is empty.
    fn call_args(&mut self, kind: ArgsKind) -> Result<Step> {
        self.expect(&Token::LParen)?;
        if self.eat(&Token::Star) {
            // `COUNT(*)` is read by the aggregate production; a bare `*` here is invalid.
            return Err(super::native_syntax(
                &"unexpected '*' in argument list",
                self.span(),
                self.memory,
            ));
        }
        if self.at(&Token::RParen) {
            self.expect(&Token::RParen)?;
            return self.finish_call(kind, Vec::new());
        }
        self.eat_kw("DISTINCT");
        {
            let native_value = Ctl::Args(kind, Vec::new());
            self.memory.push(&mut self.machine.ctl, native_value)?;
        };
        self.activate(Reach::Full, Sink::Shared)
    }

    /// Build the call whose argument list's `)` was just read.
    fn finish_call(&mut self, kind: ArgsKind, args: Vec<Expression>) -> Result<Step> {
        let expr = match kind {
            ArgsKind::Cdt { fn_kind, iri, at } => {
                // SPARQL has no overloading on argument count, so a wrong-arity call can
                // never evaluate to anything and is refused here rather than silently
                // becoming an expression error at runtime.
                if !fn_kind.arity().admits(args.len()) {
                    return Err(ParseError::CdtArity {
                        iri,
                        expected: self.describe_cdt_arity(fn_kind.arity())?,
                        found: args.len(),
                        at,
                    });
                }
                Expression::FunctionCall(Function::Cdt(CdtCall { fn_kind, iri }), args.into())
            }
            ArgsKind::Call(func) => Expression::FunctionCall(func, args.into()),
            ArgsKind::If => {
                expect_arity(&args, 3, "IF", self.span(), self.memory)?;
                let mut it = args.into_iter();
                Expression::If(
                    Child::try_new(it.next().expect("three arguments"), self.memory)?,
                    Child::try_new(it.next().expect("three arguments"), self.memory)?,
                    Child::try_new(it.next().expect("three arguments"), self.memory)?,
                )
            }
            ArgsKind::Coalesce => Expression::Coalesce(args.into()),
            ArgsKind::SameTerm => {
                expect_arity(&args, 2, "sameTerm", self.span(), self.memory)?;
                let mut it = args.into_iter();
                Expression::SameTerm(
                    Child::try_new(it.next().expect("two arguments"), self.memory)?,
                    Child::try_new(it.next().expect("two arguments"), self.memory)?,
                )
            }
            ArgsKind::Builtin(func) => {
                // The generic built-in dispatch does not arity-check; ADJUST(value,
                // timezone) is fixed at 2 (SEP-0002's sole documented signature — see the
                // `Function::Adjust` rustdoc).
                if func == Function::Adjust {
                    expect_arity(&args, 2, "ADJUST", self.span(), self.memory)?;
                }
                Expression::FunctionCall(func, args.into())
            }
        };
        Ok(Step::Operator(expr, false))
    }

    /// An RDF 1.2 triple term `<<( s p o )>>` in *expression* position
    /// (`ExprTripleTerm`, §17.4). It denotes the same value as `TRIPLE(s, p, o)`, so it
    /// lowers to that function call. Only the triple-*term* form (`<<(`) is valid here —
    /// a reifying triple `<< … >>` is not an expression.
    fn triple_term_expr(&mut self) -> Result<Step> {
        self.expect(&Token::TripleOpen)?;
        if !self.eat(&Token::LParen) {
            return Err(super::native_syntax(
                &"a reifying triple `<< … >>` is not valid in expression position; \
                 use a triple term `<<( s p o )>>`",
                self.span(),
                self.memory,
            ));
        }
        // A triple term's subject is a `Var | iri` here — never a literal or a nested
        // triple term.
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
            return Err(super::native_syntax(
                &"a literal or nested triple term may not be the subject of a triple term",
                self.span(),
                self.memory,
            ));
        }
        {
            let mut terms = Vec::new();
            self.memory.reserve(&mut terms, 3)?;
            let native_value = Ctl::TripleTerm(terms);
            self.memory.push(&mut self.machine.ctl, native_value)?;
        };
        self.activate(Reach::Primary, Sink::Shared)
    }

    /// A keyword in expression position: an aggregate, a built-in call, `EXISTS`.
    fn builtin_or_aggregate(&mut self, name: &str) -> Result<Step> {
        let mut upper = self.memory.string(name)?;
        upper.make_ascii_uppercase();
        let result = (|| {
            // Aggregates lift to a synthetic Group variable.
            if let Some(func) = aggregate_function(&upper) {
                return self.aggregate(func, &upper);
            }
            // `AGG(<iri>, [DISTINCT] arg, arg, …)` — the custom-aggregate surface; also lifts
            // to a synthetic Group variable, exactly like a named built-in aggregate. Checked
            // here (rather than added to `aggregate_function`) because it does not follow the
            // `NAME(...)` dispatch table shape: its first token inside the parens is an IRI,
            // not an expression.
            if upper == "AGG" {
                return self.custom_aggregate();
            }
            match upper.as_str() {
                "BOUND" => {
                    self.pos += 1;
                    self.expect(&Token::LParen)?;
                    let v = self.expect_var()?;
                    self.expect(&Token::RParen)?;
                    Ok(Step::Operator(Expression::Bound(v), false))
                }
                "IF" => {
                    self.pos += 1;
                    self.call_args(ArgsKind::If)
                }
                "COALESCE" => {
                    self.pos += 1;
                    self.call_args(ArgsKind::Coalesce)
                }
                "EXISTS" => {
                    self.pos += 1;
                    self.exists(false)
                }
                "NOT" => {
                    self.pos += 1;
                    self.expect_kw("EXISTS")?;
                    self.exists(true)
                }
                "SAMETERM" => {
                    self.pos += 1;
                    self.call_args(ArgsKind::SameTerm)
                }
                _ => {
                    if let Some(func) = builtin_function(&upper) {
                        self.pos += 1;
                        self.call_args(ArgsKind::Builtin(func))
                    } else {
                        Err(self.unsupported(&format_args!("function or keyword {name}")))
                    }
                }
            }
        })();
        self.memory.release_string(upper)?;
        result
    }

    /// A built-in aggregate call, its keyword at the cursor.
    fn aggregate(&mut self, func: AggregateFunction, name: &str) -> Result<Step> {
        self.pos += 1; // function name
        self.expect(&Token::LParen)?;
        // DISTINCT precedes `*` in `COUNT(DISTINCT *)`; consume it first so the star
        // form carries the flag.
        let distinct = self.eat_kw("DISTINCT");
        if self.eat(&Token::Star) {
            // `*` is the spec's empty exprlist, and the grammar admits it in exactly one
            // production: `Count` (SPARQL 1.1 §18.5.1 / SPARQL 1.2 §19.8).
            // `SUM(*)`/`AVG(*)`/`MIN(*)`/`MAX(*)`/`SAMPLE(*)`/`GROUP_CONCAT(*)` — and,
            // symmetrically, a zero-arity custom aggregate — are hard syntax errors,
            // never a silent row count.
            if func != AggregateFunction::Count {
                return Err(super::native_syntax(
                    &format_args!(
                        "`*` is only valid inside COUNT(...); {name} does not accept an empty \
                         exprlist"
                    ),
                    self.span(),
                    self.memory,
                ));
            }
            let agg = AggregateExpression::new(func, Vec::new(), Vec::new(), Vec::new(), distinct)
                .expect("COUNT accepts an empty exprlist");
            return self.finish_aggregate(agg);
        }
        // Marks any `EXISTS` reached while reading the arguments as
        // `ExistsScopeBasis::AggregateArgument` if it is deferred under
        // `Parser::projection_scope_pending` (a SELECT-list aggregate, e.g.
        // `SUM(IF(EXISTS { ... }, 1, 0))`) — irrelevant, and harmless, outside that
        // window. Restored once the arguments are read: aggregates cannot themselves
        // nest, but the `EXISTS` body an argument may contain can embed a sub-`SELECT`
        // with its own, unrelated aggregate arguments.
        let saved = std::mem::replace(&mut self.in_aggregate_argument, true);
        if matches!(func, AggregateFunction::Fold) {
            {
                let native_value = Ctl::Fold(self.boxed(Fold {
                    stage: FoldStage::First,
                    distinct,
                    saved,
                    args: Vec::new(),
                    order_by: Vec::new(),
                })?);
                self.memory.push(&mut self.machine.ctl, native_value)?;
            };
        } else {
            {
                let native_value = Ctl::Aggregate {
                    func,
                    distinct,
                    saved,
                };
                self.memory.push(&mut self.machine.ctl, native_value)?;
            };
        }
        self.activate(Reach::Full, Sink::Refuse(AGGREGATE_OUTSIDE))
    }

    /// `FOLD(…)`, resumed with the expression its stage was reading.
    ///
    /// ```text
    /// 'FOLD' '(' 'DISTINCT'? Expression ( ',' Expression )? ( 'ORDER' 'BY' OrderCondition+ )? ')'
    /// ```
    ///
    /// One expression is the `cdt:List` form, two the `cdt:Map` form (first is the key).
    /// The optional `ORDER BY` follows the LAST expression with NO separating comma — a
    /// comma there would name a third exprlist entry, which [`AggregateExpression::new`]
    /// refuses. The sort keys are the AGGREGATION's own (see
    /// [`AggregateFunction::Fold`]); an aggregate inside one of them would be a nested
    /// aggregate, refused like any other.
    fn resume_fold(&mut self, mut fold: Box<Fold>, expr: Expression) -> Result<Step> {
        match fold.stage {
            FoldStage::First => {
                {
                    let native_value = expr;
                    self.memory.push(&mut fold.args, native_value)?;
                };
                if self.eat(&Token::Comma) {
                    fold.stage = FoldStage::Second;
                    {
                        let native_value = Ctl::Fold(fold);
                        self.memory.push(&mut self.machine.ctl, native_value)?;
                    };
                    return self.activate(Reach::Full, Sink::Refuse(AGGREGATE_OUTSIDE));
                }
                self.fold_order_clause(fold)
            }
            FoldStage::Second => {
                {
                    let native_value = expr;
                    self.memory.push(&mut fold.args, native_value)?;
                };
                self.fold_order_clause(fold)
            }
            FoldStage::Ascending => {
                self.expect(&Token::RParen)?;
                {
                    let native_value = OrderExpression::Asc(expr);
                    self.memory.push(&mut fold.order_by, native_value)?;
                };
                self.fold_order_keys(fold)
            }
            FoldStage::Descending => {
                self.expect(&Token::RParen)?;
                {
                    let native_value = OrderExpression::Desc(expr);
                    self.memory.push(&mut fold.order_by, native_value)?;
                };
                self.fold_order_keys(fold)
            }
            FoldStage::Bare => {
                {
                    let native_value = OrderExpression::Asc(expr);
                    self.memory.push(&mut fold.order_by, native_value)?;
                };
                self.fold_order_keys(fold)
            }
        }
    }

    /// `FOLD`'s optional `ORDER BY` clause, after its expressions.
    fn fold_order_clause(&mut self, fold: Box<Fold>) -> Result<Step> {
        if self.eat_kw("ORDER") {
            self.expect_kw("BY")?;
            return self.fold_order_keys(fold);
        }
        self.finish_fold(*fold)
    }

    /// The next of `FOLD`'s `ORDER BY` keys, or the call's end.
    fn fold_order_keys(&mut self, mut fold: Box<Fold>) -> Result<Step> {
        let (stage, reach, sink) = if self.eat_kw("ASC") {
            self.expect(&Token::LParen)?;
            (
                FoldStage::Ascending,
                Reach::Full,
                Sink::Refuse(AGGREGATE_OUTSIDE),
            )
        } else if self.eat_kw("DESC") {
            self.expect(&Token::LParen)?;
            (
                FoldStage::Descending,
                Reach::Full,
                Sink::Refuse(AGGREGATE_OUTSIDE),
            )
        } else if self.order_key_ahead() {
            (
                FoldStage::Bare,
                Reach::Primary,
                Sink::Refuse(AGGREGATE_HERE),
            )
        } else {
            if fold.order_by.is_empty() {
                return Err(super::native_syntax(
                    &"FOLD's ORDER BY requires at least one sort condition",
                    self.span(),
                    self.memory,
                ));
            }
            return self.finish_fold(*fold);
        };
        fold.stage = stage;
        {
            let native_value = Ctl::Fold(fold);
            self.memory.push(&mut self.machine.ctl, native_value)?;
        };
        self.activate(reach, sink)
    }

    /// `FOLD(…)` whose arguments are read: build it, before its `)`.
    fn finish_fold(&mut self, fold: Fold) -> Result<Step> {
        self.in_aggregate_argument = fold.saved;
        let agg = AggregateExpression::new(
            AggregateFunction::Fold,
            fold.args,
            Vec::new(),
            fold.order_by,
            fold.distinct,
        )
        .map_err(|error| super::native_syntax(&error, self.span(), self.memory))?;
        self.finish_aggregate(agg)
    }

    /// Parse the `AGG(<iri>, [DISTINCT] arg, arg, … [; NAME=value]*)` custom-aggregate
    /// surface (the normative spelling for a custom-aggregate call — no `ParserOptions`
    /// gate, since it introduces no ambiguity with any other production). `<iri>` may be
    /// any IRI, including a prefixed name, resolved and retained byte-exact via
    /// [`Self::expect_iri_node`]. `DISTINCT`, if present, precedes the first positional
    /// argument. At least one positional argument is required — an empty argument list
    /// is a hard syntax error: there is no `AGG(<iri>)` zero-arity form.
    ///
    /// After the positional arguments, zero or more trailing `; NAME=value` scalarval
    /// clauses are admitted — see [`Self::parse_agg_scalarvals`]. This is a purely
    /// STRUCTURAL parse: whether a given custom aggregate accepts a given name (and
    /// whether its value's type is right) is validated at prepare time by the
    /// evaluator, against the registered aggregate's own declaration — never by this
    /// parser.
    fn custom_aggregate(&mut self) -> Result<Step> {
        self.pos += 1; // `AGG`
        self.expect(&Token::LParen)?;
        let iri = self.expect_iri_node()?;
        self.expect(&Token::Comma)?;
        let distinct = self.eat_kw("DISTINCT");
        // See `Self::aggregate`: marks any `EXISTS` reached while reading these
        // positional arguments as `ExistsScopeBasis::AggregateArgument`, restored once
        // the whole argument list is read.
        let saved = std::mem::replace(&mut self.in_aggregate_argument, true);
        {
            let native_value = Ctl::CustomAggregate {
                iri,
                distinct,
                saved,
                args: Vec::new(),
            };
            self.memory.push(&mut self.machine.ctl, native_value)?;
        };
        self.activate(Reach::Full, Sink::Refuse(AGGREGATE_OUTSIDE))
    }

    /// An aggregate call whose arguments are read: its `)`, and the synthetic variable
    /// that replaces it in the expression.
    fn finish_aggregate(&mut self, agg: AggregateExpression) -> Result<Step> {
        self.expect(&Token::RParen)?;
        let synth = self.fresh_agg_var()?;
        self.memory.push(
            self.machine
                .sinks
                .last_mut()
                .expect("active aggregate sink"),
            (synth.clone(), agg),
        )?;
        Ok(Step::Operator(Expression::Variable(synth), false))
    }

    /// `EXISTS`/`NOT EXISTS`, its keywords read: its group.
    fn exists(&mut self, negated: bool) -> Result<Step> {
        // Anchor the error at the body's own opening brace rather than wherever the
        // cursor lands after parsing it, mirroring `LATERAL`'s own `at` capture.
        let at = self.span();
        self.push_exists_scope_isolated()?;
        {
            let native_value = Ctl::Exists { at, negated };
            self.memory.push(&mut self.machine.ctl, native_value)?;
        };
        self.start_group()
    }

    /// Enforce SEP-0007 Part 3 on an `EXISTS`/`NOT EXISTS` group graph pattern (both
    /// spellings are the SAME production and share the SAME check): neither `BIND`, a
    /// sub-`SELECT`'s `(expr AS ?v)`, a `GROUP BY (expr AS ?v)` target NOR a `VALUES`
    /// variable inside it may rebind a variable already in scope on the row this
    /// `EXISTS` is testing.
    ///
    /// # The in-scope set consulted
    ///
    /// The top in-scope-set frame once the body's own frame is closed — which is the one
    /// the `EXISTS` keyword was read in, unchanged while its body was parsed (see
    /// `Parser::exists_scope_stack`'s doc): the transitively scope-transparent
    /// accumulation (through a plain nested group, `OPTIONAL`, `UNION`, `GRAPH`,
    /// `SERVICE`, either side of `LATERAL`) of every variable introduced, left-to-right,
    /// since the nearest enclosing TRUE scope boundary — a sub-`SELECT`'s own `WHERE`,
    /// or the query/update operation's own top-level one. For an `EXISTS` reached while
    /// parsing a `FILTER`'s constraint or a `BIND`'s value expression mid-group, that is
    /// exactly the elements of the SAME enclosing group parsed so far; for a
    /// solution-modifier expression (`GROUP BY`/`HAVING`/`ORDER BY`), it is the complete
    /// `WHERE` clause's scope (parsed in full before modifiers run).
    ///
    /// For a `SELECT`-list `(expr AS ?v)` target — or an aggregate argument lifted out of
    /// one — parsed BEFORE `WHERE` is even read, that set is necessarily still empty: the
    /// row this `EXISTS` will actually be tested against cannot be known yet. Rather than
    /// skip the check (SEP-0007 Part 3 is a SEMANTIC rule about the row at EVALUATION
    /// time, not a textual-order one), `Parser::projection_scope_pending` marks this
    /// window, and the check is DEFERRED into `Parser::pending_exists_scope_checks` —
    /// resolved once the enclosing `SELECT`'s `WHERE` is read. See
    /// [`PendingExistsScopeCheck`] for why a single `local_scope ∪ root_scope` union,
    /// computed once the root is known, suffices at ANY nesting depth reached during
    /// this window.
    ///
    /// A NESTED `EXISTS` is checked at its OWN `EXISTS` keyword, with the in-scope set
    /// THAT nesting level sees — which, because its body is parsed inside a freshly
    /// SEEDED (not merged-back) frame (see `Parser::push_exists_scope_isolated`), already
    /// includes everything visible to the outer `EXISTS` plus whatever this body's own
    /// elements have introduced so far, without this body's OWN introductions ever
    /// leaking to what the OUTER `EXISTS`'s LATER siblings see.
    ///
    /// A body that holds no fresh binding at all (`intro` false) can collide with
    /// nothing, and is neither walked nor deferred.
    fn check_exists_body(&mut self, at: usize, body: GroupValue) -> Result<GraphPattern> {
        let GroupValue {
            pattern: body,
            intro,
            scope,
            ..
        } = body;
        scope.release(self.memory)?;
        if !intro || RDFLIB {
            return Ok(body);
        }
        if self.projection_scope_pending {
            let mut local_scope = self
                .memory
                .collect(self.exists_scope_stack.top().iter().cloned())?;
            let basis = if self.in_aggregate_argument {
                ExistsScopeBasis::AggregateArgument
            } else {
                // A later SELECT-list target sees every earlier one already bound (the
                // `Extend` chain a SELECT builds from its projection nests that way) —
                // see `Parser::projection_seen_targets`'s doc.
                self.memory.extend(
                    &mut local_scope,
                    self.projection_seen_targets.iter().cloned(),
                )?;
                ExistsScopeBasis::Projection
            };
            {
                let before = self.memory.admitted_bytes();
                let cloned_body = body.clone_with_memory(self.memory)?;
                let body_bytes = self
                    .memory
                    .admitted_bytes()
                    .checked_sub(before)
                    .ok_or(purrdf_lex::allocation::StorageError::SizeOverflow)?;
                let native_value = PendingExistsScopeCheck {
                    local_scope,
                    body: cloned_body,
                    body_bytes,
                    at,
                    basis,
                };
                self.memory
                    .push(&mut self.pending_exists_scope_checks, native_value)?;
            };
            return Ok(body);
        }
        if let Some((var, intro)) =
            find_scope_conflict(self.exists_scope_stack.top(), &body, self.memory)?
        {
            return Err(super::native_syntax(
                &format_args!(
                    "{} ?{} inside {} is already in scope on {}",
                    intro.as_str(),
                    var.as_str(),
                    ScopeConstruct::Exists.keyword(),
                    ScopeConstruct::Exists.already_in_scope_clause(),
                ),
                at,
                self.memory,
            ));
        }
        Ok(body)
    }

    /// A constraint (§ Constraint: a bracketted expression, a built-in call or a
    /// function call), with no aggregate allowed in it.
    ///
    /// A bare variable or literal (`FILTER ?x`, `FILTER true`) is not a `Constraint`
    /// and is refused rather than read as a primary expression (the W3C
    /// `filter-missing-parens` negative syntax test).
    fn constraint(&mut self) -> Result<Step> {
        if self.at(&Token::LParen) {
            self.pos += 1;
            {
                let native_value = Ctl::Constraint;
                self.memory.push(&mut self.machine.ctl, native_value)?;
            };
            self.activate(Reach::Full, Sink::Refuse(AGGREGATE_OUTSIDE))
        } else if self.at_bare_constraint() {
            self.activate(Reach::Primary, Sink::Refuse(AGGREGATE_HERE))
        } else {
            Err(super::native_syntax(
                &format_args!(
                    "FILTER expects a Constraint (a bracketed expression, a built-in call \
                     or a function call), found {:?}",
                    self.tokens
                        .get(self.pos)
                        .and_then(Option::as_ref)
                        .map(|token| &token.token)
                ),
                self.span(),
                self.memory,
            ))
        }
    }

    // ── group graph patterns (§18.2.2) ───────────────────────────────────────

    /// A group graph pattern, at its `{`: a sub-`SELECT`, or a group whose elements are
    /// read next.
    fn start_group(&mut self) -> Result<Step> {
        self.expect(&Token::LBrace)?;
        if self.peek_kw("SELECT") {
            {
                let native_value = Ctl::SubSelectGroup;
                self.memory.push(&mut self.machine.ctl, native_value)?;
            };
            return self.start_select(None, SelectPosition::SubSelect);
        }
        {
            let native_value = GroupState::new();
            self.memory.push(&mut self.machine.groups, native_value)?;
        };
        Ok(Step::Elements)
    }

    /// `{ SELECT … }`, read: its pattern, and the variables its projection puts in scope.
    fn sub_select_group(&mut self, sub: Query, intro: bool) -> Result<GroupValue> {
        // Destructured field by field rather than through `..`: a sub-select keeps only
        // the pattern, and the other three have to be accounted for HERE, where the
        // `Query` is discarded, or a field added to `Query::Select` later would start
        // being dropped silently.
        match sub {
            Query::Select {
                pattern,
                dataset,
                base_iri,
                version,
            } => {
                // Refused by the SELECT itself, at the clause's own offset, so it is
                // empty here.
                debug_assert!(
                    dataset.default.is_empty() && dataset.named.is_empty(),
                    "a sub-SELECT's dataset clause is refused where it is read"
                );
                // `BASE` is prologue, parsed once at the top, and a sub-select's algebra
                // carries no base of its own.
                debug_assert!(
                    base_iri.is_none(),
                    "a sub-SELECT is parsed with no base of its own"
                );
                // NOT an absence: `VERSION` is prologue too, and every `SELECT` this
                // parse builds copies the one the prologue declared. The enclosing form
                // carries the same value, so the copy discarded here states nothing the
                // whole query does not already state.
                debug_assert_eq!(
                    version, self.version,
                    "a sub-SELECT copies the request's one prologue VERSION"
                );
                self.memory.release_vec(dataset.default)?;
                self.memory.release_vec(dataset.named)?;
                if let Some(crate::SparqlVersion::Other(text)) = version {
                    self.memory.release_string(text)?;
                }
                let mut scope = VarScope::default();
                collect_vars(&pattern, &mut scope, self.memory)?;
                Ok(GroupValue {
                    pattern,
                    scope,
                    intro,
                    filter_count: 0,
                })
            }
            _ => unreachable!("a sub-SELECT is a Query::Select"),
        }
    }

    /// Read the innermost group's elements until one needs a nested production, or the
    /// group closes.
    ///
    /// The group keeps its in-scope set in step with the pattern it builds, one element
    /// at a time: a nested group hands back the set it built, so no element is walked
    /// again by the group around it, and a `BIND`'s §19.6 check is one lookup.
    fn elements(&mut self) -> Result<Step> {
        let mut group = self
            .machine
            .groups
            .pop()
            .expect("the elements read belong to an open group");
        loop {
            if self.at(&Token::RBrace) {
                self.expect(&Token::RBrace)?;
                let GroupState {
                    mut g,
                    filters,
                    scope,
                    intro,
                    ..
                } = group;
                // RDFLib evaluates this syntactic group's filter expressions
                // together. Fold before nested empty-group joins can erase group
                // ownership; the ordinary parser retains its existing wrappers.
                let filters = if RDFLIB && filters.len() > 1 {
                    let capacity = filters.capacity();
                    let mut iterator = filters.into_iter();
                    let mut expression = iterator.next().expect("multiple collected filters");
                    for operand in iterator {
                        expression = Expression::and_with_memory(expression, operand, self.memory)?;
                    }
                    self.memory.release_bytes(
                        core::alloc::Layout::array::<Expression>(capacity)
                            .map_err(|_| purrdf_lex::allocation::StorageError::SizeOverflow)?
                            .size(),
                    )?;
                    self.memory.collect([expression])?
                } else {
                    filters
                };
                let filter_count = filters.len();
                for expr in filters {
                    g = GraphPattern::Filter {
                        expr,
                        inner: Child::try_new(g, self.memory)?,
                    };
                }
                return Ok(Step::Return(Val::Group(GroupValue {
                    pattern: g,
                    scope,
                    intro,
                    filter_count,
                })));
            }
            if self.block_boundary() && !self.peek_kw("FILTER") {
                group.open_bgp = None;
            }
            let element = if self.at(&Token::LBrace) {
                Element::Union
            } else if self.eat_kw("OPTIONAL") {
                Element::Optional
            } else if self.eat_kw("LATERAL") {
                // The error position is the start of the right-hand side's block rather
                // than wherever the cursor lands after parsing it.
                Element::Lateral { at: self.span() }
            } else if self.eat_kw("MINUS") {
                // A `MINUS` right operand contributes NOTHING to the enclosing group's
                // EXISTS in-scope set (§18.2.1) — but an `EXISTS` INSIDE it must still
                // see whatever the row being tested already has bound, so the frame is
                // SEEDED, not fresh; it is popped and discarded, never merged back.
                self.push_exists_scope_isolated()?;
                Element::Minus
            } else if self.eat_kw("GRAPH") {
                Element::Graph(self.parse_var_or_iri_name()?)
            } else if self.eat_kw("SERVICE") {
                let silent = self.eat_kw("SILENT");
                let name = self.parse_var_or_iri_name()?;
                Element::Service { silent, name }
            } else if self.eat_kw("FILTER") {
                {
                    let native_value = group;
                    self.memory.push(&mut self.machine.groups, native_value)?;
                };
                {
                    let native_value = Ctl::Element(Element::Filter);
                    self.memory.push(&mut self.machine.ctl, native_value)?;
                };
                return self.constraint();
            } else if self.eat_kw("BIND") {
                self.expect(&Token::LParen)?;
                {
                    let native_value = group;
                    self.memory.push(&mut self.machine.groups, native_value)?;
                };
                {
                    let native_value = Ctl::Element(Element::Bind);
                    self.memory.push(&mut self.machine.ctl, native_value)?;
                };
                return self.activate(Reach::Full, Sink::Refuse(AGGREGATE_OUTSIDE));
            } else if self.eat_kw("UNFOLD") {
                // `[174] Unfold ::= 'UNFOLD' '(' Expression 'AS' Var ( ',' Var )? ')'` —
                // the SEP-0009 row expander — is `BIND`'s twin: one expression, stacked
                // ABOVE the pattern parsed so far so it can read what that pattern bound.
                self.expect(&Token::LParen)?;
                {
                    let native_value = group;
                    self.memory.push(&mut self.machine.groups, native_value)?;
                };
                {
                    let native_value = Ctl::Element(Element::Unfold);
                    self.memory.push(&mut self.machine.ctl, native_value)?;
                };
                return self.activate(Reach::Full, Sink::Refuse(AGGREGATE_OUTSIDE));
            } else if self.peek_kw("VALUES") {
                let values = self.parse_inline_data()?;
                collect_vars(&values, &mut group.scope, self.memory)?;
                self.note_exists_scope(&values)?;
                group.g = self.group_join(group.g, values)?;
                group.intro = true;
                group.dot_ok = true;
                continue;
            } else if self.at(&Token::Dot) {
                // The one optional separator after a `GraphPatternNotTriples` element; a
                // dot anywhere else (`{ . }`, `{ . ?s ?p ?o }`, a second dot) is not in
                // the grammar.
                if !group.dot_ok {
                    return Err(stray_dot(self.span(), self.memory));
                }
                self.pos += 1;
                group.dot_ok = false;
                continue;
            } else {
                // A triples block (BGP / path patterns).
                let bgp = *group.open_bgp.get_or_insert_with(|| {
                    self.bgp_counter += 1;
                    self.bgp_counter
                });
                let enclosing = self.bgp_scope.replace(bgp);
                let block = self.parse_triples_block(super::TripleContext::Pattern);
                self.bgp_scope = enclosing;
                let block = block?;
                collect_vars(&block, &mut group.scope, self.memory)?;
                self.note_exists_scope(&block)?;
                group.g = self.group_join(group.g, block)?;
                group.dot_ok = false;
                continue;
            };
            {
                let native_value = group;
                self.memory.push(&mut self.machine.groups, native_value)?;
            };
            {
                let native_value = Ctl::Element(element);
                self.memory.push(&mut self.machine.ctl, native_value)?;
            };
            return self.start_group();
        }
    }

    /// Hand a group element the operand it was waiting for, and continue the group.
    fn resume_element(&mut self, element: Element, val: Val) -> Result<Step> {
        let mut group = self
            .machine
            .groups
            .pop()
            .expect("an element belongs to an open group");
        match element {
            Element::Union => {
                // Every arm joins the ONE `Union` node the chain builds
                // (`GraphPattern::union`), so a chain of any length is one combinator
                // above its arms, and the variables it puts in scope are its arms', in
                // order.
                let arm = val.group();
                let node = match group.union.take() {
                    None => arm,
                    Some(mut chain) => {
                        for v in arm.scope.as_slice() {
                            chain.scope.note(v, self.memory)?;
                        }
                        arm.scope.release(self.memory)?;
                        GroupValue {
                            pattern: GraphPattern::union_with_memory(
                                chain.pattern,
                                arm.pattern,
                                self.memory,
                            )?,
                            scope: chain.scope,
                            intro: chain.intro || arm.intro,
                            filter_count: 0,
                        }
                    }
                };
                if self.eat_kw("UNION") {
                    group.union = Some(node);
                    {
                        let native_value = group;
                        self.memory.push(&mut self.machine.groups, native_value)?;
                    };
                    {
                        let native_value = Ctl::Element(Element::Union);
                        self.memory.push(&mut self.machine.ctl, native_value)?;
                    };
                    return self.start_group();
                }
                // A bracketed sub-group (possibly a `{ SELECT ... }`, whose contribution
                // is its OWN projection, not its inner WHERE pattern) or a chain of
                // `UNION` arms.
                self.note_element_vars(&mut group.scope, node.scope)?;
                group.g = self.group_join(group.g, node.pattern)?;
                group.intro |= node.intro;
            }
            Element::Optional => {
                let inner = val.group();
                let (right, expression) =
                    split_trailing_filters(inner.pattern, inner.filter_count, self.memory)?;
                self.note_element_vars(&mut group.scope, inner.scope)?;
                group.g = GraphPattern::LeftJoin {
                    left: Child::try_new(group.g, self.memory)?,
                    right: Child::try_new(right, self.memory)?,
                    expression,
                };
                group.intro |= inner.intro;
            }
            Element::Lateral { at } => {
                let right = val.group();
                // A genuine PRODUCTION consultation (once per `LATERAL` keyword, never
                // per element inside `right`): the incremental set built so far — `g`'s
                // vars, NOT yet `right`'s — is the left-hand side's scope. Verified
                // against a fresh walk (the non-counting entry point) under
                // `debug_assertions` only.
                self.note_scope_consultation();
                #[cfg(debug_assertions)]
                {
                    let checked = compute_lateral_left_scope(&group.g, self.memory)?;
                    debug_assert_eq!(
                        group.scope.as_slice(),
                        checked.as_slice(),
                        "the incremental LATERAL left-scope drifted from a fresh visible_variables walk"
                    );
                    self.memory.release_vec(checked)?;
                }
                if right.intro
                    && let Some((var, intro)) =
                        find_scope_conflict(group.scope.as_slice(), &right.pattern, self.memory)?
                {
                    return Err(super::native_syntax(
                        &format_args!(
                            "{} ?{} inside {} is already in scope on {}",
                            intro.as_str(),
                            var.as_str(),
                            ScopeConstruct::Lateral.keyword(),
                            ScopeConstruct::Lateral.already_in_scope_clause(),
                        ),
                        at,
                        self.memory,
                    ));
                }
                self.note_element_vars(&mut group.scope, right.scope)?;
                group.g = GraphPattern::Lateral {
                    left: Child::try_new(group.g, self.memory)?,
                    right: Child::try_new(right.pattern, self.memory)?,
                };
                group.intro |= right.intro;
            }
            Element::Minus => {
                let right = val.group();
                right.scope.release(self.memory)?;
                self.pop_exists_scope_boundary();
                // SPARQL §18.2.1: `MINUS`'s right operand contributes NOTHING to the
                // enclosing group's scope, and `find_scope_conflict` never walks it.
                group.g = GraphPattern::Minus {
                    left: Child::try_new(group.g, self.memory)?,
                    right: Child::try_new(right.pattern, self.memory)?,
                };
            }
            Element::Graph(name) => {
                let inner = val.group();
                if let NamedNodePattern::Variable(v) = &name {
                    group.scope.note(v, self.memory)?;
                    self.note_exists_scope_var(v)?;
                }
                self.note_element_vars(&mut group.scope, inner.scope)?;
                let graph = GraphPattern::Graph {
                    name,
                    inner: Child::try_new(inner.pattern, self.memory)?,
                };
                group.g = self.group_join(group.g, graph)?;
                group.intro |= inner.intro;
            }
            Element::Service { silent, name } => {
                let inner = val.group();
                let is_var_endpoint = matches!(name, NamedNodePattern::Variable(_));
                if let NamedNodePattern::Variable(v) = &name {
                    group.scope.note(v, self.memory)?;
                    self.note_exists_scope_var(v)?;
                }
                self.note_element_vars(&mut group.scope, inner.scope)?;
                let service = GraphPattern::Service {
                    name,
                    inner: Child::try_new(inner.pattern, self.memory)?,
                    silent,
                };
                // A variable endpoint (`SERVICE ?g`) is correlated with the enclosing
                // pattern — it must bind the endpoint from the surrounding solution
                // before federating — so it becomes a LATERAL join. A fixed-IRI endpoint
                // stays a plain join.
                group.g = if is_var_endpoint {
                    GraphPattern::Lateral {
                        left: Child::try_new(group.g, self.memory)?,
                        right: Child::try_new(service, self.memory)?,
                    }
                } else {
                    self.group_join(group.g, service)?
                };
                group.intro |= inner.intro;
            }
            Element::Filter => {
                let native_value = val.expr().0;
                self.memory.push(&mut group.filters, native_value)?;
            }
            Element::Bind => {
                let expression = val.expr().0;
                self.expect_kw("AS")?;
                let variable = self.expect_var()?;
                self.expect(&Token::RParen)?;
                // §19.6: the variable introduced by BIND must not already be in-scope in
                // the group graph pattern up to this point — a re-binding is a hard
                // syntax error, not a silent shadow (vendored W3C `syntax-query`
                // `syntax-BINDscope6/7/8`). The incremental set answers this in O(log n)
                // — NOT a production "consultation" (`note_scope_consultation` is NOT
                // called here: it fires once per `BIND` and must not scale the count
                // with the group's element count). The equivalence check still runs,
                // through the free-function (non-counting) `visible_variables`.
                #[cfg(debug_assertions)]
                {
                    let checked = visible_variables_with_memory(&group.g, self.memory)?;
                    debug_assert_eq!(
                        group.scope.contains(&variable),
                        checked.contains(&variable),
                        "the incremental BIND-scope check drifted from a fresh visible_variables walk"
                    );
                    self.memory.release_vec(checked)?;
                }
                if !RDFLIB && group.scope.contains(&variable) {
                    return Err(super::native_syntax(
                        &format_args!(
                            "BIND target ?{} is already in scope in the group graph pattern",
                            variable.as_str()
                        ),
                        self.span(),
                        self.memory,
                    ));
                }
                group.scope.note(&variable, self.memory)?;
                self.note_exists_scope_var(&variable)?;
                group.g = GraphPattern::Extend {
                    inner: Child::try_new(group.g, self.memory)?,
                    variable,
                    expression,
                };
                group.intro = true;
            }
            Element::Unfold => {
                let expression = val.expr().0;
                self.expect_kw("AS")?;
                let element = self.expect_var()?;
                let companion = if self.eat(&Token::Comma) {
                    Some(self.expect_var()?)
                } else {
                    None
                };
                self.expect(&Token::RParen)?;
                // `BIND`'s §19.6 scope rule, verbatim, on BOTH targets.
                for variable in std::iter::once(&element).chain(companion.as_ref()) {
                    #[cfg(debug_assertions)]
                    {
                        let checked = visible_variables_with_memory(&group.g, self.memory)?;
                        debug_assert_eq!(
                            group.scope.contains(variable),
                            checked.contains(variable),
                            "the incremental UNFOLD-scope check drifted from a fresh \
                             visible_variables walk"
                        );
                        self.memory.release_vec(checked)?;
                    }
                    if group.scope.contains(variable) {
                        return Err(super::native_syntax(
                            &format_args!(
                                "UNFOLD target ?{} is already in scope in the group graph pattern",
                                variable.as_str()
                            ),
                            self.span(),
                            self.memory,
                        ));
                    }
                }
                // The two targets bind two DIFFERENT positions of one element (see
                // `GraphPattern::Unfold`), so one variable in both slots would have to
                // hold two values in one row. Refused here rather than resolved by a
                // precedence rule nobody could guess.
                if companion.as_ref() == Some(&element) {
                    return Err(super::native_syntax(
                        &format_args!(
                            "UNFOLD binds ?{} twice; its two targets must be distinct variables",
                            element.as_str()
                        ),
                        self.span(),
                        self.memory,
                    ));
                }
                for variable in std::iter::once(&element).chain(companion.as_ref()) {
                    group.scope.note(variable, self.memory)?;
                    self.note_exists_scope_var(variable)?;
                }
                group.g = GraphPattern::Unfold {
                    inner: Child::try_new(group.g, self.memory)?,
                    expression,
                    element,
                    companion,
                };
                group.intro = true;
            }
        }
        group.dot_ok = true;
        {
            let native_value = group;
            self.memory.push(&mut self.machine.groups, native_value)?;
        };
        Ok(Step::Elements)
    }

    /// Note the variables a finished element puts in scope — its own set, in its order —
    /// into the group's set and into the innermost `EXISTS` in-scope frame, exactly as a
    /// walk of the element's pattern would.
    fn note_element_vars(&mut self, scope: &mut VarScope, element: VarScope) -> Result<()> {
        for variable in element.as_slice() {
            scope.note(variable, self.memory)?;
            self.note_exists_scope_var(variable)?;
        }
        element.release(self.memory)?;
        Ok(())
    }

    // ── SELECT / sub-SELECT ──────────────────────────────────────────────────

    /// A `SELECT` at its keyword: its projection is read next.
    fn start_select(
        &mut self,
        base_iri: Option<NamedNode>,
        position: SelectPosition,
    ) -> Result<Step> {
        self.expect_kw("SELECT")?;
        let dedup = if self.eat_kw("DISTINCT") {
            Dedup::Distinct
        } else if self.eat_kw("REDUCED") {
            Dedup::Reduced
        } else {
            Dedup::None
        };

        // A fresh, EMPTY EXISTS in-scope-set frame for this SELECT/sub-SELECT — opened
        // before the projection list is even read, so an `EXISTS` inside a `(expr AS
        // ?v)` SELECT-list target sees no ambient scope leaked in from whatever query
        // this one is nested inside (a sub-SELECT is not correlated with its outer
        // query). Stays open through `WHERE` and the solution modifiers, and is popped
        // once the SELECT is built.
        self.push_exists_scope_boundary()?;

        // This SELECT's OWN deferred-EXISTS-scope window (SEP-0007 Part 3's
        // projection-list position — see `Parser::projection_scope_pending`'s doc):
        // save whatever the ENCLOSING parse had (this may itself be a sub-SELECT reached
        // mid-projection-list of an outer one, via `EXISTS { SELECT ... }`), open a
        // fresh one for the projection list about to be read, and restore the
        // enclosing state once this SELECT is built. On a parse error the whole request
        // aborts, and this `Parser` is never consulted again.
        let state = SelectState {
            position,
            base_iri,
            dedup,
            saved_pending: self.projection_scope_pending,
            saved_in_aggregate: self.in_aggregate_argument,
            saved_seen: std::mem::take(&mut self.projection_seen_targets),
            saved_checks: std::mem::take(&mut self.pending_exists_scope_checks),
            star: false,
            projected: Vec::new(),
            select_exprs: Vec::new(),
            aggregates: Vec::new(),
            dataset: QueryDataset::default(),
            where_pattern: None,
        };
        self.projection_scope_pending = true;
        self.in_aggregate_argument = false;
        {
            let native_value = state;
            self.memory.push(&mut self.machine.selects, native_value)?;
        };

        // Projection: `*` or a list of Var / (Expr AS Var).
        if self.eat(&Token::Star) {
            self.machine.selects.last_mut().expect("active SELECT").star = true;
            return self.select_where();
        }
        self.projection()
    }

    /// The rest of a projection list: plain variables until a `( expression AS ?v )`
    /// hands over to its expression, or the list ends.
    fn projection(&mut self) -> Result<Step> {
        loop {
            if let Some(Token::Variable(_)) = self.peek() {
                let v = self.expect_var()?;
                {
                    let native_value = v;
                    self.memory.push(
                        &mut self
                            .machine
                            .selects
                            .last_mut()
                            .expect("active SELECT")
                            .projected,
                        native_value,
                    )?;
                };
            } else if self.at(&Token::LParen) {
                self.expect(&Token::LParen)?;
                {
                    let native_value = Ctl::Select(SelectStage::Projection);
                    self.memory.push(&mut self.machine.ctl, native_value)?;
                };
                return self.activate(Reach::Full, Sink::Lift);
            } else {
                break;
            }
        }
        if self
            .machine
            .selects
            .last_mut()
            .expect("active SELECT")
            .projected
            .is_empty()
        {
            return Err(super::native_syntax(
                &"empty SELECT projection",
                self.span(),
                self.memory,
            ));
        }
        self.select_where()
    }

    /// A `SELECT` whose projection is read: its dataset clause, then its `WHERE` group.
    fn select_where(&mut self) -> Result<Step> {
        // The projection list is fully parsed — leave the deferred-EXISTS-scope window.
        // Every `EXISTS`/`NOT EXISTS` reached from here on (`WHERE`, `GROUP
        // BY`/`HAVING`/`ORDER BY`) already has a correct `Parser::exists_scope` to check
        // against immediately.
        self.projection_scope_pending = false;

        // Dataset clause (FROM / FROM NAMED), §13.2. Taken before the run is read, so
        // the refusal below points at the caller's own `FROM` keyword rather than at
        // wherever the parse happened to stop afterwards.
        let dataset_at = self.span();
        let enclosing_slot = self.dataset_slot.clone();
        let dataset = self.parse_dataset_clauses()?;
        let position = self
            .machine
            .selects
            .last_mut()
            .expect("active SELECT")
            .position;
        if position == SelectPosition::SubSelect {
            // A sub-select's (necessarily empty) run is not the whole query's slot.
            self.dataset_slot = enclosing_slot;
        }
        // §18 `SubSelect ::= SelectClause WhereClause SolutionModifier ValuesClause` —
        // there is no `DatasetClause` in it, and a dataset clause scopes a whole query
        // rather than one group of one. Reading the run here and then dropping it at the
        // sub-select site is the one outcome that cannot be right: the clause a caller
        // wrote would decide nothing while the query still answered, which is a wrong
        // answer under no diagnostic at all.
        if position == SelectPosition::SubSelect
            && !(dataset.default.is_empty() && dataset.named.is_empty())
        {
            return Err(super::native_syntax(
                &"a sub-SELECT carries no dataset clause: FROM and FROM NAMED are \
                 written on a whole query, which is the scope they apply to",
                dataset_at,
                self.memory,
            ));
        }
        self.machine
            .selects
            .last_mut()
            .expect("active SELECT")
            .dataset = dataset;
        self.eat_kw("WHERE");
        {
            let native_value = Ctl::Select(SelectStage::Where);
            self.memory.push(&mut self.machine.ctl, native_value)?;
        };
        Ok(Step::Group)
    }

    /// Hand a `SELECT` the value its stage was reading.
    fn resume_select(&mut self, stage: SelectStage, val: Val) -> Result<Step> {
        match stage {
            SelectStage::Projection => {
                let (expr, lifted) = val.expr();
                self.memory.append(
                    &mut self
                        .machine
                        .selects
                        .last_mut()
                        .expect("active production")
                        .aggregates,
                    lifted,
                )?;
                self.expect_kw("AS")?;
                let var = self.expect_var()?;
                self.expect(&Token::RParen)?;
                // Recorded so a LATER projection-list `EXISTS` deferred under
                // `projection_scope_pending` sees this target as already bound — see
                // `Parser::projection_seen_targets`'s doc.
                {
                    let native_value = var.clone();
                    self.memory
                        .push(&mut self.projection_seen_targets, native_value)?;
                };
                let state = self.machine.selects.last_mut().expect("active SELECT");
                {
                    let native_value = var.clone();
                    self.memory.push(&mut state.projected, native_value)?;
                };
                // A long `SELECT (e1 AS ?v1) … (eN AS ?vN)` list lowers to a chain of N
                // `Extend` nodes wrapped around the WHERE pattern once it is built.
                {
                    let native_value = (var, expr);
                    self.memory.push(&mut state.select_exprs, native_value)?;
                };
                self.projection()
            }
            SelectStage::Where => {
                let GroupValue {
                    pattern,
                    scope,
                    intro,
                    ..
                } = val.group();
                // The WHERE pattern's scope mirrored into this SELECT's own frame, so the
                // solution modifiers see it whatever built it — a group's elements, or a
                // WHERE clause that IS just `{ SELECT ... }`.
                for v in scope.as_slice() {
                    self.note_exists_scope_var(v)?;
                }
                scope.release(self.memory)?;
                let state = self.machine.selects.last_mut().expect("active SELECT");
                state.where_pattern = Some((pattern, intro));
                let aggregates = std::mem::take(&mut state.aggregates);
                {
                    let native_value = Ctl::Select(SelectStage::Modifiers);
                    self.memory.push(&mut self.machine.ctl, native_value)?;
                };
                self.start_modifiers(aggregates)
            }
            SelectStage::Modifiers => {
                let Val::Modifiers(modifiers, aggregates) = val else {
                    unreachable!("a solution-modifier list returns modifiers")
                };
                let state = self
                    .machine
                    .selects
                    .pop()
                    .expect("the modifiers belong to an open SELECT");
                self.finish_select(state, modifiers, aggregates)
            }
        }
    }

    /// A `SELECT` whose solution modifiers are read: check it, read its trailing
    /// `VALUES`, and build its algebra (§18.2.4 ordering).
    fn finish_select(
        &mut self,
        state: SelectState,
        mut modifiers: Modifiers,
        mut aggregates: Vec<Lifted>,
    ) -> Result<Step> {
        let SelectState {
            base_iri,
            dedup,
            saved_pending,
            saved_in_aggregate,
            saved_seen,
            saved_checks,
            star,
            projected,
            mut select_exprs,
            dataset,
            where_pattern,
            ..
        } = state;
        let (where_pat, where_intro) =
            where_pattern.expect("a SELECT's modifiers follow its WHERE group");

        // A `GROUP BY (expr AS ?v)` target must be fresh too — not in scope in the
        // `WHERE` clause, and not an earlier condition's target. §18.2.1 makes `?v` in
        // scope by that very form and requires it not to be in scope already at the
        // point of an `(expr AS ?v)`, and the condition lowers to an `Extend`
        // (§18.2.4.1), which §18.5 leaves undefined for a variable the solution already
        // binds. A synthetic target (`GROUP BY (expr)`) is minted outside every name a
        // query can write and never collides.
        if !RDFLIB && !modifiers.group_extends.is_empty() {
            // A PRODUCTION consultation of the whole WHERE pattern's scope — once per
            // SELECT with expression-valued GROUP BY conditions.
            self.note_scope_consultation();
            let mut in_scope = VarScope::default();
            collect_vars(&where_pat, &mut in_scope, self.memory)?;
            for (variable, _) in &modifiers.group_extends {
                if !in_scope.note(variable, self.memory)? {
                    return Err(super::native_syntax(
                        &format_args!(
                            "GROUP BY target ?{} is already in scope in the WHERE clause or \
                             an earlier GROUP BY condition",
                            variable.as_str()
                        ),
                        self.span(),
                        self.memory,
                    ));
                }
            }
            in_scope.release(self.memory)?;
        }

        // §19.8: each SELECT `(expr AS ?v)` target must be fresh — not already in scope.
        // When the query aggregates (an explicit `GROUP BY` or any aggregate ⇒ implicit
        // single group), only the grouping keys and group-expression targets stay
        // visible to the projection; the raw WHERE pattern variables are projected away
        // by grouping, so re-binding one via `(expr AS ?v)` is legal (e.g. `SELECT (123
        // AS ?z) … GROUP BY ?s`).
        if !RDFLIB && !select_exprs.is_empty() {
            let aggregating = !modifiers.group_by.is_empty()
                || !modifiers.group_extends.is_empty()
                || !aggregates.is_empty();
            let mut in_scope = VarScope::default();
            if aggregating {
                for variable in &modifiers.group_by {
                    in_scope.note(variable, self.memory)?;
                }
                for (variable, _) in &modifiers.group_extends {
                    in_scope.note(variable, self.memory)?;
                }
            } else {
                self.note_scope_consultation();
                collect_vars(&where_pat, &mut in_scope, self.memory)?;
            }

            // SEP-0007 Part 3's projection-list position: resolve every `EXISTS`/`NOT
            // EXISTS` deferred while this projection list was being parsed — BEFORE the
            // loop below folds this SELECT's own targets into `in_scope`, so
            // `ExistsScopeBasis::Projection` entries resolve against the SAME root
            // `in_scope` currently holds (the grouped keys, or the full `WHERE` scope)
            // with no contamination from sibling targets
            // `PendingExistsScopeCheck::local_scope` did not already capture.
            // `ExistsScopeBasis::AggregateArgument` entries resolve against the raw
            // `WHERE`/grouping-extend scope instead, computed lazily (at most once, only
            // if this SELECT actually deferred an aggregate-argument `EXISTS`) since it
            // differs from `in_scope` only when the query aggregates.
            if !self.pending_exists_scope_checks.is_empty() {
                let mut pending_checks = std::mem::take(&mut self.pending_exists_scope_checks);
                let mut agg_arg_scope: Option<VarScope> = None;
                #[expect(
                    clippy::iter_with_drain,
                    reason = "draining moves each payload while retaining its original admitted Vec buffer until release_vec destroys that buffer before refund"
                )]
                for pending in pending_checks.drain(..) {
                    let root = match pending.basis {
                        ExistsScopeBasis::Projection => &in_scope,
                        ExistsScopeBasis::AggregateArgument => {
                            if aggregating {
                                if agg_arg_scope.is_none() {
                                    self.note_scope_consultation();
                                    let mut scope = VarScope::default();
                                    collect_vars(&where_pat, &mut scope, self.memory)?;
                                    for (variable, _) in &modifiers.group_extends {
                                        scope.note(variable, self.memory)?;
                                    }
                                    agg_arg_scope = Some(scope);
                                }
                                agg_arg_scope.as_ref().expect("just populated above")
                            } else {
                                &in_scope
                            }
                        }
                    };
                    let mut scope = VarScope::default();
                    for variable in pending.local_scope.iter().chain(root.as_slice()) {
                        scope.note(variable, self.memory)?;
                    }
                    let found = find_scope_conflict(scope.as_slice(), &pending.body, self.memory)?;
                    scope.release(self.memory)?;
                    self.memory.release_vec(pending.local_scope)?;
                    let failure = if let Some((var, intro)) = found {
                        Some(super::native_syntax(
                            &format_args!(
                                "{} ?{} inside {} is already in scope on {}",
                                intro.as_str(),
                                var.as_str(),
                                ScopeConstruct::Exists.keyword(),
                                ScopeConstruct::Exists.already_in_scope_clause(),
                            ),
                            pending.at,
                            self.memory,
                        ))
                    } else {
                        None
                    };
                    drop(pending.body);
                    self.memory.release_bytes(pending.body_bytes)?;
                    if let Some(failure) = failure {
                        return Err(failure);
                    }
                }
                if let Some(scope) = agg_arg_scope {
                    scope.release(self.memory)?;
                }
                self.memory.release_vec(pending_checks)?;
            }

            for (var, _) in &select_exprs {
                if !in_scope.note(var, self.memory)? {
                    return Err(super::native_syntax(
                        &format_args!(
                            "SELECT expression target ?{} is already in scope",
                            var.as_str()
                        ),
                        self.span(),
                        self.memory,
                    ));
                }
            }
            in_scope.release(self.memory)?;
        }

        // §11.1 grammar note: the `SELECT *` shorthand is illegal in an aggregate query —
        // an explicit `GROUP BY` (keys or expression conditions) or any aggregate makes
        // the projection ill-defined, so it is a hard syntax error (vendored W3C
        // `syntax-query` `syn-bad-01`: `SELECT * … GROUP BY`).
        if !RDFLIB
            && star
            && (!modifiers.group_by.is_empty()
                || !modifiers.group_extends.is_empty()
                || !aggregates.is_empty())
        {
            return Err(super::native_syntax(
                &"SELECT * is not allowed in an aggregate query (GROUP BY or aggregation)",
                self.span(),
                self.memory,
            ));
        }

        // §18.2.4.1 grouping constraint: when the query aggregates (an explicit `GROUP
        // BY`, or one or more aggregates in the SELECT clause ⇒ an implicit single
        // group), every BARE projected variable — one named directly as a `Var`, not the
        // fresh target of a `(expr AS ?v)` — must be one of the `GROUP BY` keys (explicit
        // or the synthetic var of an expression-valued GROUP BY condition). A bare
        // projected variable that is neither a group key nor confined to an aggregate is
        // a hard query error, not a silently wrong answer (vendored W3C
        // `grouping/group06`/`group07`). `SELECT *` is exempted here: its projection is
        // derived structurally from the (already-grouped) algebra node below, so it can
        // only ever expose grouped/aggregate variables.
        if !RDFLIB && !star {
            let is_aggregating = !modifiers.group_by.is_empty() || !aggregates.is_empty();
            if is_aggregating {
                let mut as_targets = VarScope::default();
                for (variable, _) in &select_exprs {
                    as_targets.note(variable, self.memory)?;
                }
                // A variable the caller binds before evaluation
                // (`SparqlParser::with_prebound_variables`) holds one value for the
                // whole evaluation, so every group reads the same value: it is as good
                // as a key. No other variable is exempt.
                let mut group_vars = VarScope::default();
                for variable in modifiers.group_by.iter().chain(&self.prebound) {
                    group_vars.note(variable, self.memory)?;
                }
                for var in &projected {
                    if !as_targets.contains(var) && !group_vars.contains(var) {
                        return Err(super::native_syntax(
                            &format_args!(
                                "SELECT projects ?{}, which is neither a GROUP BY key nor \
                                 confined to an aggregate",
                                var.as_str()
                            ),
                            self.span(),
                            self.memory,
                        ));
                    }
                }
                // The same constraint inside a `(expr AS ?v)` (SPARQL 1.1 §11.4): outside
                // an aggregate, an expression may read only group keys, aggregate
                // results and the targets of earlier SELECT expressions. Grouping BY an
                // expression does not make the variables in it keys, so `SELECT ((?a +
                // ?b) AS ?s) … GROUP BY (?a + ?b)` is refused (the vendored W3C
                // `aggregates/agg08` and `agg11` negative syntax tests), and so is a
                // variable the WHERE clause
                // never binds, or binds only inside `MINUS`. An `EXISTS` body is not
                // read: a variable that occurs only there is local to it.
                let mut readable = group_vars;
                for (variable, _) in &aggregates {
                    readable.note(variable, self.memory)?;
                }
                for (target, expr) in &select_exprs {
                    if let Some(var) =
                        first_projection_read(expr, |v| readable.contains(v), self.memory)?
                    {
                        return Err(super::native_syntax(
                            &format_args!(
                                "SELECT expression for ?{} reads ?{}, which is neither a \
                                 GROUP BY key nor confined to an aggregate",
                                target.as_str(),
                                var.as_str()
                            ),
                            self.span(),
                            self.memory,
                        ));
                    }
                    readable.note(target, self.memory)?;
                }
                as_targets.release(self.memory)?;
                readable.release(self.memory)?;
            }
        }

        // Trailing `ValuesClause` (§18.2.4.3): a `VALUES DataBlock` after the solution
        // modifiers — valid on both a top-level query and a `SubSelect`. It is joined
        // with the WHERE group graph pattern *before* grouping and projection, so the
        // inline data is visible to aggregation and `SELECT *`. Through the shared
        // `join()` helper (not a raw `GraphPattern::Join`), matching the
        // identity-absorbing construction every IN-BODY `VALUES` block already goes
        // through — an empty WHERE clause (`{}`) plus a trailing `VALUES` must reach the
        // SAME `Values { .. }` node an in-body `{ VALUES … }` does.
        let trailing_values = self.peek_kw("VALUES");
        let where_pat = if trailing_values {
            let values = self.parse_inline_data()?;
            self.group_join(where_pat, values)?
        } else {
            where_pat
        };

        let has_group = !modifiers.group_by.is_empty() || !aggregates.is_empty();
        let intro = where_intro || has_group || !select_exprs.is_empty() || trailing_values;
        let contextual_star = if RDFLIB && star {
            Some(super::contextual_visible_variables(
                &where_pat,
                self.memory,
            )?)
        } else {
            None
        };

        let mut sampled_projections = Vec::new();
        if RDFLIB && has_group {
            // Contextual grouping publishes aggregate mappings. A projected scalar
            // read becomes a SAMPLE input; the alias is assigned after grouping.
            let lifted = self
                .memory
                .collect(aggregates.iter().map(|(name, _)| name.clone()))?;
            let mut order_has_aggregates = false;
            for order in &modifiers.order_by {
                let (OrderExpression::Asc(expression) | OrderExpression::Desc(expression)) = order;
                crate::walk::walk_pre_post_with_memory(
                    crate::walk::NodeRef::Expr(expression),
                    |visit, node, _| {
                        if visit == crate::walk::Visit::Enter {
                            node.for_each_variable(|name| {
                                order_has_aggregates |= lifted.contains(name);
                            });
                        }
                        Ok::<_, ParseError>(crate::walk::Flow::Descend)
                    },
                    self.memory,
                )?;
            }
            for (target, expression) in &mut select_exprs {
                self.sample_contextual_reads(expression, Some(target), &lifted, &mut aggregates)?;
            }
            for expression in &mut modifiers.having {
                self.sample_contextual_reads(expression, None, &lifted, &mut aggregates)?;
            }
            if order_has_aggregates {
                for order in &mut modifiers.order_by {
                    let (OrderExpression::Asc(expression) | OrderExpression::Desc(expression)) =
                        order;
                    self.sample_contextual_reads(expression, None, &lifted, &mut aggregates)?;
                }
            }
            for name in &projected {
                if !select_exprs.iter().any(|(target, _)| target == name) {
                    let sampled = self.fresh_agg_var()?;
                    {
                        let native_value = (
                            sampled.clone(),
                            AggregateExpression::new(
                                AggregateFunction::Sample,
                                self.memory.collect([Expression::Variable(name.clone())])?,
                                Vec::new(),
                                Vec::new(),
                                false,
                            )
                            .expect("SAMPLE has one positional argument"),
                        );
                        self.memory.push(&mut aggregates, native_value)?;
                    };
                    {
                        let native_value = (name.clone(), Expression::Variable(sampled));
                        self.memory.push(&mut sampled_projections, native_value)?;
                    };
                }
            }
        }

        // Build the algebra (§18.2.4 ordering).
        let mut p = where_pat;
        // Expression-valued GROUP BY conditions bind their synthetic/explicit grouping
        // variable BELOW the Group, so `eval_group` sees a ready column.
        for (var, expr) in modifiers.group_extends {
            p = GraphPattern::Extend {
                inner: Child::try_new(p, self.memory)?,
                variable: var,
                expression: expr,
            };
        }
        if has_group {
            p = GraphPattern::Group {
                inner: Child::try_new(p, self.memory)?,
                variables: self.memory.collect(modifiers.group_by.iter().cloned())?,
                aggregates,
            };
        }
        for (variable, expression) in sampled_projections {
            p = GraphPattern::Extend {
                inner: Child::try_new(p, self.memory)?,
                variable,
                expression,
            };
        }
        for expr in modifiers.having {
            p = GraphPattern::Filter {
                expr,
                inner: Child::try_new(p, self.memory)?,
            };
        }
        for (var, expr) in select_exprs {
            p = GraphPattern::Extend {
                inner: Child::try_new(p, self.memory)?,
                variable: var,
                expression: expr,
            };
        }
        if !modifiers.order_by.is_empty() {
            p = GraphPattern::OrderBy {
                inner: Child::try_new(p, self.memory)?,
                expression: modifiers.order_by,
            };
        }
        let variables = if star {
            // A PRODUCTION consultation of the whole (modifier-wrapped) query pattern's
            // scope — once per `SELECT *`, never once per element of the WHERE pattern
            // it wraps.
            self.note_scope_consultation();
            if RDFLIB {
                contextual_star.expect("a contextual star captured its source projection")
            } else {
                visible_variables_with_memory(&p, self.memory)?
            }
        } else {
            projected
        };
        p = GraphPattern::Project {
            inner: Child::try_new(p, self.memory)?,
            variables,
        };
        match dedup {
            Dedup::Distinct => {
                p = GraphPattern::Distinct {
                    inner: Child::try_new(p, self.memory)?,
                };
            }
            Dedup::Reduced => {
                p = GraphPattern::Reduced {
                    inner: Child::try_new(p, self.memory)?,
                };
            }
            Dedup::None => {}
        }
        if modifiers.offset.is_some() || modifiers.limit.is_some() {
            p = GraphPattern::Slice {
                inner: Child::try_new(p, self.memory)?,
                start: modifiers.offset.unwrap_or(0),
                length: modifiers.limit,
            };
        }
        self.pop_exists_scope_boundary();
        // Every deferred check this SELECT recorded is resolved inside the
        // `!select_exprs.is_empty()` §19.8 block above — entries can exist ONLY if that
        // block ran (an `EXISTS` can be parsed here solely from within a `(expr AS ?v)`
        // target's own expression, which always pushes to `select_exprs`), so nothing
        // should ever reach this point still unresolved. A debug-only guard, not a
        // silent drop: were this invariant ever wrong, restoring the saved (unrelated,
        // outer) list below would discard the unresolved checks instead of erroring.
        debug_assert!(
            self.pending_exists_scope_checks.is_empty(),
            "a deferred EXISTS scope check was never resolved"
        );
        // Restore the enclosing parse's own deferred-EXISTS-scope window.
        self.projection_scope_pending = saved_pending;
        self.in_aggregate_argument = saved_in_aggregate;
        self.projection_seen_targets = saved_seen;
        self.pending_exists_scope_checks = saved_checks;
        let version = self
            .version
            .as_ref()
            .map(|version| crate::SparqlVersion::parse_with_memory(version.raw(), self.memory))
            .transpose()?;
        Ok(Step::Return(Val::Query(
            self.boxed(Query::Select {
                pattern: p,
                dataset,
                base_iri,
                version,
            })?,
            intro,
        )))
    }

    /// Replace scalar reads outside aggregates and EXISTS with SAMPLE outputs.
    fn sample_contextual_reads(
        &mut self,
        expression: &mut Expression,
        target: Option<&Variable>,
        lifted: &[Variable],
        aggregates: &mut Vec<Lifted>,
    ) -> Result<()> {
        let counter = &mut self.agg_counter;
        crate::walk::for_each_expression_mut_with_memory(
            [expression],
            self.memory,
            |expression, memory| {
                if let Expression::Variable(name) = expression
                    && target != Some(name)
                    && !lifted.contains(name)
                {
                    let sampled = Variable::from_admitted(
                        memory
                            .admission_mut()
                            .text(&format_args!("__purrdf_agg_{}", *counter))?,
                    );
                    *counter = counter
                        .checked_add(1)
                        .ok_or(purrdf_lex::allocation::StorageError::SizeOverflow)?;
                    let arguments = memory.collect([Expression::Variable(name.clone())])?;
                    let aggregate = AggregateExpression::new(
                        AggregateFunction::Sample,
                        arguments,
                        Vec::new(),
                        Vec::new(),
                        false,
                    )
                    .expect("SAMPLE has one positional argument");
                    memory.push(aggregates, (sampled.clone(), aggregate))?;
                    *name = sampled;
                }
                Ok::<_, ParseError>(())
            },
        )
        .map_err(|error| match error {
            crate::walk::MutationError::Visitor(error) => error,
            crate::walk::MutationError::Storage(error) => ParseError::Storage(error),
        })
    }

    // ── solution modifiers ───────────────────────────────────────────────────

    /// A solution-modifier list: its `GROUP BY`, then the rest.
    fn start_modifiers(&mut self, aggregates: Vec<Lifted>) -> Result<Step> {
        {
            let native_value = ModState {
                m: Modifiers::default(),
                aggregates,
            };
            self.memory
                .push(&mut self.machine.modifiers, native_value)?;
        };
        if self.eat_kw("GROUP") {
            self.expect_kw("BY")?;
            // `GroupClause ::= 'GROUP' 'BY' GroupCondition+`: at least one condition.
            if !(matches!(self.peek(), Some(Token::Variable(_)))
                || self.at(&Token::LParen)
                || self.at_bare_constraint())
            {
                return Err(empty_modifier_clause(
                    "GROUP BY",
                    "GroupCondition",
                    self.span(),
                    self.memory,
                ));
            }
            return self.group_by();
        }
        self.having_clause()
    }

    /// The rest of a `GROUP BY` condition list.
    fn group_by(&mut self) -> Result<Step> {
        loop {
            if let Some(Token::Variable(_)) = self.peek() {
                let v = self.expect_var()?;
                {
                    let native_value = v;
                    self.memory.push(
                        &mut self
                            .machine
                            .modifiers
                            .last_mut()
                            .expect("active modifiers")
                            .m
                            .group_by,
                        native_value,
                    )?;
                };
            } else if self.at(&Token::LParen) {
                // `( Expr [AS ?v] )` — SPARQL 1.1 §18.2.4 GroupCondition, lowered to an
                // Extend(?v := Expr) under the Group, then grouped by ?v. An aggregate in
                // a GROUP BY key is illegal and surfaces as `Unsupported`.
                self.expect(&Token::LParen)?;
                {
                    let native_value = Ctl::Modifiers(ModStage::GroupBracketed);
                    self.memory.push(&mut self.machine.ctl, native_value)?;
                };
                return self.activate(Reach::Full, Sink::Refuse(AGGREGATE_OUTSIDE));
            } else if self.at_bare_constraint() {
                // A bare `BuiltInCall` / `FunctionCall` GroupCondition, e.g. `GROUP BY
                // STR(?x)` — lowered to a synthetic-var Extend.
                {
                    let native_value = Ctl::Modifiers(ModStage::GroupBare);
                    self.memory.push(&mut self.machine.ctl, native_value)?;
                };
                return self.activate(Reach::Full, Sink::Refuse(AGGREGATE_OUTSIDE));
            } else {
                break;
            }
        }
        self.having_clause()
    }

    /// An optional `HAVING Constraint+` clause.
    fn having_clause(&mut self) -> Result<Step> {
        if self.eat_kw("HAVING") {
            return self.having_constraint();
        }
        self.order_clause()
    }

    /// One element of `HAVING`'s `Constraint+` list (§Constraint):
    /// `BrackettedExpression | BuiltInCall | FunctionCall`, with aggregates lifted into
    /// the query's aggregation — `HAVING (COUNT(?x) > 1)` needs that lift for the
    /// bracketed form, and a bare aggregate `BuiltInCall` (e.g. `HAVING COUNT(?x)`,
    /// unusual but grammar-legal) needs the same treatment.
    ///
    /// A bare variable or literal (`HAVING ?x`, `HAVING 1`, `HAVING true`) is not a
    /// `Constraint` and is refused rather than read as a primary expression.
    fn having_constraint(&mut self) -> Result<Step> {
        if self.at(&Token::LParen) {
            self.pos += 1;
            {
                let native_value = Ctl::Modifiers(ModStage::HavingBracketed);
                self.memory.push(&mut self.machine.ctl, native_value)?;
            };
            self.activate(Reach::Full, Sink::Lift)
        } else if self.at_bare_constraint() {
            {
                let native_value = Ctl::Modifiers(ModStage::HavingBare);
                self.memory.push(&mut self.machine.ctl, native_value)?;
            };
            self.activate(Reach::Primary, Sink::Lift)
        } else {
            Err(super::native_syntax(
                &format_args!(
                    "HAVING expects a Constraint (a bracketed expression, a built-in call \
                     or a function call), found {:?}",
                    self.tokens
                        .get(self.pos)
                        .and_then(Option::as_ref)
                        .map(|token| &token.token)
                ),
                self.span(),
                self.memory,
            ))
        }
    }

    /// After a `HAVING` constraint: another one, or the clause's end.
    fn having_next(&mut self) -> Result<Step> {
        // A long `HAVING (c1) (c2) … (cN)` condition list lowers to a chain of `Filter`
        // nodes. Each `cN` is itself a `Constraint` — bracketed (`Token::LParen`) or bare
        // (`Self::at_bare_constraint`).
        if self.at(&Token::LParen) || self.at_bare_constraint() {
            return self.having_constraint();
        }
        self.order_clause()
    }

    /// An optional `ORDER BY OrderCondition+` clause.
    fn order_clause(&mut self) -> Result<Step> {
        if self.eat_kw("ORDER") {
            self.expect_kw("BY")?;
            // `OrderClause ::= 'ORDER' 'BY' OrderCondition+`: at least one condition.
            if !(self.peek_kw("ASC") || self.peek_kw("DESC") || self.order_key_ahead()) {
                return Err(empty_modifier_clause(
                    "ORDER BY",
                    "OrderCondition",
                    self.span(),
                    self.memory,
                ));
            }
            return self.order_keys();
        }
        self.bound_clauses()
    }

    /// The next `ORDER BY` key, or the clause's end.
    fn order_keys(&mut self) -> Result<Step> {
        let (stage, reach) = if self.eat_kw("ASC") {
            self.expect(&Token::LParen)?;
            (ModStage::OrderAscending, Reach::Full)
        } else if self.eat_kw("DESC") {
            self.expect(&Token::LParen)?;
            (ModStage::OrderDescending, Reach::Full)
        } else if self.order_key_ahead() {
            (ModStage::OrderBare, Reach::Primary)
        } else {
            return self.bound_clauses();
        };
        {
            let native_value = Ctl::Modifiers(stage);
            self.memory.push(&mut self.machine.ctl, native_value)?;
        };
        self.activate(reach, Sink::Lift)
    }

    /// `LimitOffsetClauses ::= LimitClause OffsetClause? | OffsetClause LimitClause?` —
    /// at most ONE of each, in either order — and the list's end.
    ///
    /// A repeat is refused instead of overwriting the earlier clause: a caller who wrote
    /// `LIMIT 2` and got every row back because a later `LIMIT 13` overwrote the bound
    /// has had their own bound silently dropped, and the query would mean one thing here
    /// and another to a conforming processor. The offset is captured BEFORE the keyword
    /// is eaten so the diagnostic points at the repeated clause, not at its integer.
    fn bound_clauses(&mut self) -> Result<Step> {
        let mut state = self
            .machine
            .modifiers
            .pop()
            .expect("a solution-modifier list is being read");
        loop {
            let at = self.span();
            if self.eat_kw("LIMIT") {
                if state.m.limit.is_some() {
                    return Err(repeated_bound_clause("LIMIT", at, self.memory));
                }
                state.m.limit = Some(self.expect_integer()?);
            } else if self.eat_kw("OFFSET") {
                if state.m.offset.is_some() {
                    return Err(repeated_bound_clause("OFFSET", at, self.memory));
                }
                state.m.offset = Some(self.expect_integer()?);
            } else {
                break;
            }
        }
        Ok(Step::Return(Val::Modifiers(state.m, state.aggregates)))
    }

    /// Hand a solution-modifier list the expression its stage was reading.
    fn resume_modifiers(&mut self, stage: ModStage, val: Val) -> Result<Step> {
        let (expr, lifted) = val.expr();
        match stage {
            ModStage::GroupBracketed => {
                // `GROUP BY (?s)` (or `((?s))`): a condition that is only a variable
                // groups by that variable, so it is a key the SELECT clause may project
                // (§11.4), exactly as the unbracketed `GROUP BY ?s`.
                if let Expression::Variable(v) = &expr
                    && !self.peek_kw("AS")
                {
                    let v = v.clone();
                    self.expect(&Token::RParen)?;
                    {
                        let native_value = v;
                        self.memory.push(
                            &mut self
                                .machine
                                .modifiers
                                .last_mut()
                                .expect("active modifiers")
                                .m
                                .group_by,
                            native_value,
                        )?;
                    };
                    return self.group_by();
                }
                let var = if self.eat_kw("AS") {
                    self.expect_var()?
                } else {
                    self.fresh_group_var()?
                };
                self.expect(&Token::RParen)?;
                let m = &mut self
                    .machine
                    .modifiers
                    .last_mut()
                    .expect("active modifiers")
                    .m;
                {
                    let native_value = (var.clone(), expr);
                    self.memory.push(&mut m.group_extends, native_value)?;
                };
                {
                    let native_value = var;
                    self.memory.push(&mut m.group_by, native_value)?;
                };
                self.group_by()
            }
            ModStage::GroupBare => {
                let var = self.fresh_group_var()?;
                let m = &mut self
                    .machine
                    .modifiers
                    .last_mut()
                    .expect("active modifiers")
                    .m;
                {
                    let native_value = (var.clone(), expr);
                    self.memory.push(&mut m.group_extends, native_value)?;
                };
                {
                    let native_value = var;
                    self.memory.push(&mut m.group_by, native_value)?;
                };
                self.group_by()
            }
            ModStage::HavingBracketed => {
                self.memory.append(
                    &mut self
                        .machine
                        .modifiers
                        .last_mut()
                        .expect("active production")
                        .aggregates,
                    lifted,
                )?;
                self.expect(&Token::RParen)?;
                {
                    let native_value = expr;
                    self.memory.push(
                        &mut self
                            .machine
                            .modifiers
                            .last_mut()
                            .expect("active modifiers")
                            .m
                            .having,
                        native_value,
                    )?;
                };
                self.having_next()
            }
            ModStage::HavingBare => {
                let state = self.machine.modifiers.last_mut().expect("active modifiers");
                self.memory.append(&mut state.aggregates, lifted)?;
                {
                    let native_value = expr;
                    self.memory.push(&mut state.m.having, native_value)?;
                };
                self.having_next()
            }
            ModStage::OrderAscending | ModStage::OrderDescending => {
                self.memory.append(
                    &mut self
                        .machine
                        .modifiers
                        .last_mut()
                        .expect("active production")
                        .aggregates,
                    lifted,
                )?;
                self.expect(&Token::RParen)?;
                let key = if matches!(stage, ModStage::OrderAscending) {
                    OrderExpression::Asc(expr)
                } else {
                    OrderExpression::Desc(expr)
                };
                {
                    let native_value = key;
                    self.memory.push(
                        &mut self
                            .machine
                            .modifiers
                            .last_mut()
                            .expect("active modifiers")
                            .m
                            .order_by,
                        native_value,
                    )?;
                };
                self.order_keys()
            }
            ModStage::OrderBare => {
                let state = self.machine.modifiers.last_mut().expect("active modifiers");
                self.memory.append(&mut state.aggregates, lifted)?;
                {
                    let native_value = OrderExpression::Asc(expr);
                    self.memory.push(&mut state.m.order_by, native_value)?;
                };
                self.order_keys()
            }
        }
    }
}

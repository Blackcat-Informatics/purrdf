// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The SPARQL query algebra (W3C SPARQL 1.1 §18.2), purrdf-owned and RDF 1.2-native.
//!
//! This is the *algebra* form, not a raw syntax tree: solution modifiers
//! (`DISTINCT`, `ORDER BY`, `LIMIT`/`OFFSET`, `GROUP BY`) are encoded as
//! [`GraphPattern`] nodes wrapping the `WHERE` algebra, exactly as the standard
//! translation prescribes. That is why a [`Query::Select`] holds only its root
//! `pattern` and a consumer walks *into* the pattern to find `Project`/`Distinct`/
//! `Slice`/`OrderBy`/`Group`.
//!
//! ## S6 extension seam (#912)
//!
//! This algebra is intentionally a faithful, standard, *evaluable* IR — the form
//! the downstream evaluator S6 (`sparql-eval`) consumes. The greenfield lever for
//! exploiting the native OWL/EL-DL reasoner (e.g. routing `rdfs:subClassOf*` to
//! the DL subsumption closure rather than evaluating the path structurally, or
//! making the entailment regime a first-class concern) is an *evaluation*-time
//! decision and belongs in S6: it would annotate or wrap these nodes there. S5
//! keeps the door open by owning its own enums (free to grow variants/annotations
//! later) rather than cloning a fixed external type.

use crate::ast::{
    GroundTerm, Literal, NamedNode, NamedNodePattern, QuadPattern, TermPattern, TriplePattern,
    Variable,
};

/// A parsed SPARQL query. The four query forms differ only in their head; the
/// `WHERE` clause and all solution modifiers live inside `pattern` as algebra.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Query {
    /// `SELECT` query. `pattern` is the full modifier-wrapped algebra.
    Select {
        /// The root graph pattern (already wrapped by projection/modifiers).
        pattern: GraphPattern,
        /// The `FROM` / `FROM NAMED` dataset clause (empty = the store's default).
        dataset: QueryDataset,
        /// An explicit `BASE` IRI, if the prologue declared one.
        base_iri: Option<NamedNode>,
    },
    /// `CONSTRUCT` query. `template` is the output triple template.
    Construct {
        /// The `CONSTRUCT { ... }` triple template.
        template: Vec<TriplePattern>,
        /// The `WHERE` algebra.
        pattern: GraphPattern,
        /// The `FROM` / `FROM NAMED` dataset clause (empty = the store's default).
        dataset: QueryDataset,
        /// An explicit `BASE` IRI, if any.
        base_iri: Option<NamedNode>,
    },
    /// `DESCRIBE` query.
    Describe {
        /// The `WHERE` algebra (or the unit pattern for a bare `DESCRIBE <iri>`).
        pattern: GraphPattern,
        /// The resources to describe (IRIs and/or variables).
        targets: Vec<NamedNodePattern>,
        /// The `FROM` / `FROM NAMED` dataset clause (empty = the store's default).
        dataset: QueryDataset,
        /// An explicit `BASE` IRI, if any.
        base_iri: Option<NamedNode>,
    },
    /// `ASK` query.
    Ask {
        /// The `WHERE` algebra.
        pattern: GraphPattern,
        /// The `FROM` / `FROM NAMED` dataset clause (empty = the store's default).
        dataset: QueryDataset,
        /// An explicit `BASE` IRI, if any.
        base_iri: Option<NamedNode>,
    },
}

impl Query {
    /// The query's `FROM` / `FROM NAMED` dataset clause (empty = the store default).
    pub fn dataset(&self) -> &QueryDataset {
        match self {
            Self::Select { dataset, .. }
            | Self::Construct { dataset, .. }
            | Self::Describe { dataset, .. }
            | Self::Ask { dataset, .. } => dataset,
        }
    }
}

/// A SPARQL query **dataset clause** (`FROM` / `FROM NAMED`, §13.2). An empty
/// clause (both lists empty) means "use the store's default dataset" — the default
/// graph plus every named graph. A non-empty clause replaces it: the active default
/// graph becomes the RDF-merge of the `default` IRIs (the store default graph is then
/// excluded), and only the `named` IRIs are addressable via `GRAPH`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct QueryDataset {
    /// `FROM <iri>` graphs, merged to form the active default graph.
    pub default: Vec<NamedNode>,
    /// `FROM NAMED <iri>` graphs, the named graphs addressable by `GRAPH`.
    pub named: Vec<NamedNode>,
}

/// One `USING` / `USING NAMED` clause of a `DELETE`/`INSERT` operation (§3.1.3) — the
/// UPDATE counterpart of [`QueryDataset`], scoping the `WHERE` active dataset. The
/// `NAMED` modifier is preserved (unlike a bare [`GraphTarget`]), because `USING <g>`
/// (folds `g` into the active default graph) and `USING NAMED <g>` (makes `g`
/// addressable via `GRAPH`) have distinct semantics.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum UsingClause {
    /// `USING <iri>` — adds the graph to the active default graph (≡ `FROM`).
    Default(NamedNode),
    /// `USING NAMED <iri>` — makes the graph addressable via `GRAPH` (≡ `FROM NAMED`).
    Named(NamedNode),
}

/// A parsed SPARQL 1.1 Update request: a sequence of graph-update operations,
/// applied in order (later operations observe earlier ones' effects).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Update {
    /// The operations, in request order.
    pub operations: Vec<GraphUpdateOperation>,
    /// An explicit `BASE` IRI, if the prologue declared one.
    pub base_iri: Option<NamedNode>,
}

/// The target of a graph-management operation
/// (`CLEAR`/`DROP`/`ADD`/`MOVE`/`COPY`/`LOAD` destination). Models the SPARQL
/// `GraphRefAll` production's four forms.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GraphTarget {
    /// The `DEFAULT` keyword: the default (unnamed) graph.
    Default,
    /// `GRAPH <iri>` (or a bare `<iri>`): a single specific named graph.
    Named(NamedNode),
    /// The `NAMED` keyword: every named graph, but **not** the default graph.
    NamedGraphs,
    /// The `ALL` keyword: the default graph **and** every named graph.
    All,
}

/// One SPARQL 1.1 Update operation (§3.1–§3.2).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GraphUpdateOperation {
    /// `INSERT DATA { ... }` — add concrete quads. The data is variable-free (a
    /// hard parser invariant) but MAY contain blank nodes (standard SPARQL §3.1.1:
    /// blanks are minted fresh per request); hence [`QuadPattern`], not a ground
    /// quad type that cannot hold blanks.
    InsertData {
        /// The quads to add (variable-free; blank nodes allowed).
        data: Vec<QuadPattern>,
    },
    /// `DELETE DATA { ... }` — remove concrete quads. The data is variable-free AND
    /// blank-node-free (both hard parser invariants per §3.1.2), but is modeled as
    /// [`QuadPattern`] for a single uniform DATA representation.
    DeleteData {
        /// The quads to remove (variable-free and blank-node-free).
        data: Vec<QuadPattern>,
    },
    /// `DELETE { ... } INSERT { ... } WHERE { ... }` and its `DELETE WHERE` /
    /// insert-only / `WITH`/`USING` shorthands. Either template may be empty.
    DeleteInsert {
        /// The `DELETE` template (quad patterns to remove per solution). Empty for insert-only.
        delete: Vec<QuadPattern>,
        /// The `INSERT` template (quad patterns to add per solution). Empty for delete-only.
        insert: Vec<QuadPattern>,
        /// The `WITH <iri>` default graph for the operation, if any.
        with: Option<NamedNode>,
        /// The `USING` / `USING NAMED` dataset clauses, if any (the active dataset for WHERE).
        using: Vec<UsingClause>,
        /// The `WHERE` graph pattern (the unit pattern for a bare `DELETE WHERE { ... }`).
        pattern: Box<GraphPattern>,
    },
    /// `LOAD [SILENT] <iri> [INTO GRAPH <iri>]`. `destination` is a [`GraphTarget`]
    /// for uniformity with the other graph-management ops, but only its `Default`
    /// (no `INTO GRAPH` — load into the default graph) and `Named` (explicit
    /// `INTO GRAPH <iri>`) variants are valid here; `NamedGraphs`/`All` are not.
    Load {
        /// The `SILENT` flag.
        silent: bool,
        /// The `<iri>` to dereference and load.
        source: NamedNode,
        /// The destination graph (`Default` = no explicit `INTO GRAPH`).
        destination: GraphTarget,
    },
    /// `CLEAR [SILENT] <target>` — remove all quads in the target.
    Clear {
        /// The `SILENT` flag.
        silent: bool,
        /// The graph(s) to clear.
        target: GraphTarget,
    },
    /// `DROP [SILENT] <target>` — remove the graph(s).
    Drop {
        /// The `SILENT` flag.
        silent: bool,
        /// The graph(s) to drop.
        target: GraphTarget,
    },
    /// `CREATE [SILENT] GRAPH <iri>`.
    Create {
        /// The `SILENT` flag.
        silent: bool,
        /// The named graph to create.
        graph: NamedNode,
    },
    /// `ADD [SILENT] <source> TO <destination>` — copy all quads, leaving source intact.
    Add {
        /// The `SILENT` flag.
        silent: bool,
        /// The source graph.
        source: GraphTarget,
        /// The destination graph.
        destination: GraphTarget,
    },
    /// `MOVE [SILENT] <source> TO <destination>` — move all quads (dest cleared first).
    Move {
        /// The `SILENT` flag.
        silent: bool,
        /// The source graph.
        source: GraphTarget,
        /// The destination graph.
        destination: GraphTarget,
    },
    /// `COPY [SILENT] <source> TO <destination>` — copy all quads (dest cleared first).
    Copy {
        /// The `SILENT` flag.
        silent: bool,
        /// The source graph.
        source: GraphTarget,
        /// The destination graph.
        destination: GraphTarget,
    },
}

impl core::fmt::Display for GraphTarget {
    /// Serialize a graph target to its SPARQL `GraphRefAll` surface syntax.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Default => write!(f, "DEFAULT"),
            Self::Named(n) => write!(f, "GRAPH <{}>", n.as_str()),
            Self::NamedGraphs => write!(f, "NAMED"),
            Self::All => write!(f, "ALL"),
        }
    }
}

impl core::fmt::Display for UsingClause {
    /// Serialize a `USING` clause, preserving the `NAMED` modifier.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Default(n) => write!(f, "USING <{}>", n.as_str()),
            Self::Named(n) => write!(f, "USING NAMED <{}>", n.as_str()),
        }
    }
}

impl core::fmt::Display for QueryDataset {
    /// Serialize a query dataset clause: `FROM <iri>` and `FROM NAMED <iri>` per graph.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        for g in &self.default {
            write!(f, "FROM <{}> ", g.as_str())?;
        }
        for g in &self.named {
            write!(f, "FROM NAMED <{}> ", g.as_str())?;
        }
        Ok(())
    }
}

impl core::fmt::Display for Update {
    /// Serialize an Update request: its operations joined by `;`.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if let Some(base) = &self.base_iri {
            write!(f, "BASE <{}> ", base.as_str())?;
        }
        for (i, op) in self.operations.iter().enumerate() {
            if i > 0 {
                write!(f, " ; ")?;
            }
            write!(f, "{op}")?;
        }
        Ok(())
    }
}

/// Render an [`NamedNodePattern`] in SPARQL surface syntax (`<iri>` or `?var`).
fn fmt_named_node_pattern(n: &NamedNodePattern) -> String {
    match n {
        NamedNodePattern::NamedNode(node) => format!("<{}>", node.as_str()),
        NamedNodePattern::Variable(v) => format!("?{}", v.as_str()),
    }
}

/// Render a [`TermPattern`] in SPARQL surface syntax.
fn fmt_term_pattern(t: &TermPattern) -> String {
    match t {
        TermPattern::NamedNode(n) => format!("<{}>", n.as_str()),
        TermPattern::BlankNode(b) => format!("_:{}", b.as_str()),
        TermPattern::Literal(l) => fmt_literal(l),
        TermPattern::Variable(v) => format!("?{}", v.as_str()),
        TermPattern::Triple(t) => format!(
            "<<( {} {} {} )>>",
            fmt_term_pattern(&t.subject),
            fmt_named_node_pattern(&t.predicate),
            fmt_term_pattern(&t.object),
        ),
    }
}

/// Render a [`TriplePattern`] as `s p o`.
fn fmt_triple_pattern(t: &TriplePattern) -> String {
    format!(
        "{} {} {}",
        fmt_term_pattern(&t.subject),
        fmt_named_node_pattern(&t.predicate),
        fmt_term_pattern(&t.object),
    )
}

/// Render a [`Literal`] in SPARQL surface syntax.
fn fmt_literal(l: &Literal) -> String {
    match (l.language(), l.direction()) {
        (Some(lang), Some(dir)) => {
            let d = match dir {
                crate::ast::BaseDirection::Ltr => "ltr",
                crate::ast::BaseDirection::Rtl => "rtl",
            };
            format!("{:?}@{lang}--{d}", l.value())
        }
        (Some(lang), None) => format!("{:?}@{lang}", l.value()),
        (None, _) => format!("{:?}^^<{}>", l.value(), l.datatype().as_str()),
    }
}

/// Render a `DELETE`/`INSERT` template (a list of [`QuadPattern`]s) as the body of
/// a `{ ... }` block, grouping graph-scoped patterns into `GRAPH g { ... }`.
fn fmt_quad_pattern_body(quads: &[QuadPattern]) -> String {
    use core::fmt::Write as _;

    let mut out = String::new();
    for q in quads {
        // Writing to a `String` is infallible, so the `write!` results are ignored.
        match &q.graph {
            None => {
                let _ = write!(out, "{} . ", fmt_triple_pattern(&q.triple));
            }
            Some(g) => {
                let _ = write!(
                    out,
                    "GRAPH {} {{ {} . }} ",
                    fmt_named_node_pattern(g),
                    fmt_triple_pattern(&q.triple),
                );
            }
        }
    }
    out.trim_end().to_owned()
}

impl core::fmt::Display for GraphUpdateOperation {
    /// Serialize one update operation to SPARQL Update surface syntax.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InsertData { data } => {
                write!(f, "INSERT DATA {{ {} }}", fmt_quad_pattern_body(data))
            }
            Self::DeleteData { data } => {
                write!(f, "DELETE DATA {{ {} }}", fmt_quad_pattern_body(data))
            }
            Self::DeleteInsert {
                delete,
                insert,
                with,
                using,
                pattern,
            } => {
                if let Some(w) = with {
                    write!(f, "WITH <{}> ", w.as_str())?;
                }
                if !delete.is_empty() {
                    write!(f, "DELETE {{ {} }} ", fmt_quad_pattern_body(delete))?;
                }
                if !insert.is_empty() {
                    write!(f, "INSERT {{ {} }} ", fmt_quad_pattern_body(insert))?;
                }
                for u in using {
                    write!(f, "{u} ")?;
                }
                write!(f, "WHERE {{ {pattern:?} }}")
            }
            Self::Load {
                silent,
                source,
                destination,
            } => {
                write!(f, "LOAD ")?;
                if *silent {
                    write!(f, "SILENT ")?;
                }
                write!(f, "<{}>", source.as_str())?;
                match destination {
                    GraphTarget::Default => Ok(()),
                    GraphTarget::Named(n) => write!(f, " INTO GRAPH <{}>", n.as_str()),
                    other => write!(f, " INTO {other}"),
                }
            }
            Self::Clear { silent, target } => {
                write!(f, "CLEAR ")?;
                if *silent {
                    write!(f, "SILENT ")?;
                }
                write!(f, "{target}")
            }
            Self::Drop { silent, target } => {
                write!(f, "DROP ")?;
                if *silent {
                    write!(f, "SILENT ")?;
                }
                write!(f, "{target}")
            }
            Self::Create { silent, graph } => {
                write!(f, "CREATE ")?;
                if *silent {
                    write!(f, "SILENT ")?;
                }
                write!(f, "GRAPH <{}>", graph.as_str())
            }
            Self::Add {
                silent,
                source,
                destination,
            } => {
                write!(f, "ADD ")?;
                if *silent {
                    write!(f, "SILENT ")?;
                }
                write!(f, "{source} TO {destination}")
            }
            Self::Move {
                silent,
                source,
                destination,
            } => {
                write!(f, "MOVE ")?;
                if *silent {
                    write!(f, "SILENT ")?;
                }
                write!(f, "{source} TO {destination}")
            }
            Self::Copy {
                silent,
                source,
                destination,
            } => {
                write!(f, "COPY ")?;
                if *silent {
                    write!(f, "SILENT ")?;
                }
                write!(f, "{source} TO {destination}")
            }
        }
    }
}

/// A node of the SPARQL graph-pattern algebra (§18.2). The empty pattern (the
/// identity table `Z`) is represented as `Bgp { patterns: vec![] }`.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GraphPattern {
    /// A basic graph pattern: a conjunction of triple patterns.
    Bgp {
        /// The triple patterns (RDF 1.2 quoted triples admitted).
        patterns: Vec<TriplePattern>,
    },
    /// A property-path constraint `subject path object`.
    Path {
        /// The path's subject term.
        subject: TermPattern,
        /// The property path.
        path: PropertyPathExpression,
        /// The path's object term.
        object: TermPattern,
    },
    /// Conjunction (`Join`) of two patterns.
    Join {
        /// Left operand.
        left: Box<Self>,
        /// Right operand.
        right: Box<Self>,
    },
    /// `OPTIONAL` (left outer join), with an optional join condition (a `FILTER`
    /// lifted into the `OPTIONAL` per §18.2.2.3).
    LeftJoin {
        /// Left (required) operand.
        left: Box<Self>,
        /// Right (optional) operand.
        right: Box<Self>,
        /// The join-condition expression, if the `OPTIONAL` had a `FILTER`.
        expression: Option<Expression>,
    },
    /// A correlated/lateral join (`LATERAL`), kept for algebra completeness.
    Lateral {
        /// Left operand.
        left: Box<Self>,
        /// Right operand, evaluated per left solution.
        right: Box<Self>,
    },
    /// `FILTER expr` over an inner pattern.
    Filter {
        /// The filter expression.
        expr: Expression,
        /// The pattern being filtered.
        inner: Box<Self>,
    },
    /// `UNION` of two patterns.
    Union {
        /// Left operand.
        left: Box<Self>,
        /// Right operand.
        right: Box<Self>,
    },
    /// `GRAPH name { ... }`.
    Graph {
        /// The named-graph IRI or variable.
        name: NamedNodePattern,
        /// The inner pattern scoped to that graph.
        inner: Box<Self>,
    },
    /// `BIND(expression AS variable)` — `Extend` in algebra.
    Extend {
        /// The pattern being extended.
        inner: Box<Self>,
        /// The newly bound variable.
        variable: Variable,
        /// The expression whose value it binds.
        expression: Expression,
    },
    /// `MINUS` (set difference on compatible solutions).
    Minus {
        /// Left operand.
        left: Box<Self>,
        /// Right operand (solutions to subtract).
        right: Box<Self>,
    },
    /// `SERVICE` (federated query). In scope structurally; the evaluator may
    /// reject it. `silent` is the `SILENT` flag.
    Service {
        /// The service endpoint IRI or variable.
        name: NamedNodePattern,
        /// The pattern sent to the endpoint.
        inner: Box<Self>,
        /// Whether the `SILENT` keyword was present.
        silent: bool,
    },
    /// Inline `VALUES` data.
    Values {
        /// The column variables.
        variables: Vec<Variable>,
        /// The rows; `None` is `UNDEF`.
        bindings: Vec<Vec<Option<GroundTerm>>>,
    },
    /// `ORDER BY`.
    OrderBy {
        /// The pattern being ordered.
        inner: Box<Self>,
        /// The ordered list of sort keys.
        expression: Vec<OrderExpression>,
    },
    /// Projection (`SELECT` variable list, or `SELECT *`).
    Project {
        /// The pattern being projected.
        inner: Box<Self>,
        /// The projected variables.
        variables: Vec<Variable>,
    },
    /// `DISTINCT`.
    Distinct {
        /// The pattern whose solutions are de-duplicated.
        inner: Box<Self>,
    },
    /// `REDUCED`.
    Reduced {
        /// The pattern whose solutions may be de-duplicated.
        inner: Box<Self>,
    },
    /// `LIMIT`/`OFFSET`.
    Slice {
        /// The pattern being sliced.
        inner: Box<Self>,
        /// The `OFFSET` (0 if absent).
        start: usize,
        /// The `LIMIT`, if present.
        length: Option<usize>,
    },
    /// `GROUP BY` + aggregates.
    Group {
        /// The pattern being grouped.
        inner: Box<Self>,
        /// The grouping key variables.
        variables: Vec<Variable>,
        /// The `(output variable, aggregate)` pairs.
        aggregates: Vec<(Variable, AggregateExpression)>,
    },
}

/// A SPARQL property-path expression (§18.1.7 / §9).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PropertyPathExpression {
    /// A single predicate IRI.
    NamedNode(NamedNode),
    /// `^path` — inverse.
    Reverse(Box<Self>),
    /// `p1 / p2` — sequence.
    Sequence(Box<Self>, Box<Self>),
    /// `p1 | p2` — alternative.
    Alternative(Box<Self>, Box<Self>),
    /// `path*` — zero or more.
    ZeroOrMore(Box<Self>),
    /// `path+` — one or more.
    OneOrMore(Box<Self>),
    /// `path?` — zero or one.
    ZeroOrOne(Box<Self>),
    /// `!(p1|...|pn)` — negated property set.
    NegatedPropertySet(Vec<NamedNode>),
    /// `path{min,max}` — **bounded repetition** (a PurRDF extension *beyond* SPARQL
    /// 1.1 §9, which has only `*`/`+`/`?`).  `max == None` means unbounded (`{n,}`);
    /// `max == Some(min)` is exactly-`n` (`{n}`).  The invariant `min <= max` (when
    /// `max` is `Some`) is enforced at construction by the parser.
    Range {
        /// The repeated sub-path.
        inner: Box<Self>,
        /// Inclusive lower bound on repetitions.
        min: u32,
        /// Inclusive upper bound; `None` ⇒ unbounded.
        max: Option<u32>,
    },
    /// A **predicate wildcard** matching ANY predicate (a PurRDF extension beyond
    /// SPARQL 1.1 §9, which can only name predicates).  Optionally scoped to a
    /// predicate namespace IRI prefix (`namespace`), bounding the otherwise
    /// unbounded fan-out.
    Wildcard {
        /// A predicate-namespace IRI prefix the wildcard is restricted to, or
        /// `None` for any namespace.
        namespace: Option<NamedNode>,
    },
}

impl core::fmt::Display for PropertyPathExpression {
    /// Serialize a property path to its SPARQL surface syntax.  The standard
    /// operators round-trip with the parser; the two PurRDF extensions render as
    /// `path{min,max}` (bounded repetition — round-trips) and `<any>` / `<any:ns>`
    /// (predicate wildcard — **emit-only**, no parse, per `LOGIC-PATHS.md`).
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NamedNode(n) => write!(f, "<{}>", n.as_str()),
            Self::Reverse(a) => write!(f, "^{}", PathElt(a)),
            Self::Sequence(a, b) => write!(f, "{a}/{b}"),
            Self::Alternative(a, b) => write!(f, "{a}|{b}"),
            Self::ZeroOrMore(a) => write!(f, "{}*", PathElt(a)),
            Self::OneOrMore(a) => write!(f, "{}+", PathElt(a)),
            Self::ZeroOrOne(a) => write!(f, "{}?", PathElt(a)),
            Self::Range { inner, min, max } => match max {
                Some(m) if *m == *min => write!(f, "{}{{{min}}}", PathElt(inner)),
                Some(m) => write!(f, "{}{{{min},{m}}}", PathElt(inner)),
                None => write!(f, "{}{{{min},}}", PathElt(inner)),
            },
            Self::NegatedPropertySet(nodes) => {
                let inner = nodes
                    .iter()
                    .map(|n| format!("<{}>", n.as_str()))
                    .collect::<Vec<_>>()
                    .join("|");
                write!(f, "!({inner})")
            }
            Self::Wildcard { namespace } => match namespace {
                Some(ns) => write!(f, "<any:{}>", ns.as_str()),
                None => write!(f, "<any>"),
            },
        }
    }
}

/// Wraps a property path in parentheses when it must be grouped to sit under a
/// postfix operator (`*`/`+`/`?`/`{n,m}`) — i.e. when it is a sequence,
/// alternative, or **inverse** (`^`) path.
///
/// The postfix quantifiers bind tighter than `^` in the SPARQL grammar:
/// `parse_path_elt_or_inverse` applies `^` and then delegates to
/// `parse_path_elt` for the quantified primary.  So `^<p>*` reparses as
/// `Reverse(ZeroOrMore(<p>))`, not `ZeroOrMore(Reverse(<p>))`.  Wrapping a
/// `Reverse` inner in parentheses — `(^<p>)*` — forces the parser to treat
/// the whole inverse path as the primary before the quantifier is applied,
/// preserving the original AST on round-trip.
struct PathElt<'a>(&'a PropertyPathExpression);

impl core::fmt::Display for PathElt<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.0 {
            PropertyPathExpression::Sequence(..)
            | PropertyPathExpression::Alternative(..)
            | PropertyPathExpression::Reverse(..) => {
                write!(f, "({})", self.0)
            }
            other => write!(f, "{other}"),
        }
    }
}

/// A SPARQL expression (filter/bind/having/order/select-expression position).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Expression {
    /// An IRI constant.
    NamedNode(NamedNode),
    /// A literal constant.
    Literal(Literal),
    /// A variable reference.
    Variable(Variable),
    /// `BOUND(?v)`.
    Bound(Variable),
    /// Logical `||`.
    Or(Box<Self>, Box<Self>),
    /// Logical `&&`.
    And(Box<Self>, Box<Self>),
    /// `=`.
    Equal(Box<Self>, Box<Self>),
    /// `sameTerm(a, b)`.
    SameTerm(Box<Self>, Box<Self>),
    /// `>`.
    Greater(Box<Self>, Box<Self>),
    /// `>=`.
    GreaterOrEqual(Box<Self>, Box<Self>),
    /// `<`.
    Less(Box<Self>, Box<Self>),
    /// `<=`.
    LessOrEqual(Box<Self>, Box<Self>),
    /// `+`.
    Add(Box<Self>, Box<Self>),
    /// `-` (binary).
    Subtract(Box<Self>, Box<Self>),
    /// `*`.
    Multiply(Box<Self>, Box<Self>),
    /// `/`.
    Divide(Box<Self>, Box<Self>),
    /// Unary `+`.
    UnaryPlus(Box<Self>),
    /// Unary `-`.
    UnaryMinus(Box<Self>),
    /// `!`.
    Not(Box<Self>),
    /// `expr IN (list)`.
    In(Box<Self>, Vec<Self>),
    /// `IF(cond, then, else)`.
    If(Box<Self>, Box<Self>, Box<Self>),
    /// `COALESCE(list)`.
    Coalesce(Vec<Self>),
    /// A built-in or custom function call.
    FunctionCall(Function, Vec<Self>),
    /// `EXISTS { pattern }` (`NOT EXISTS` is `Not(Exists(...))`).
    Exists(Box<GraphPattern>),
}

/// A SPARQL function: a built-in (`BuiltInCall`) or a custom IRI-named function.
///
/// Only [`Function::Custom`] carries an IRI; the built-ins are keyword-named and
/// reference no term. The set is complete (the full SPARQL 1.1 `BuiltInCall`
/// surface) so the algebra can subsume any in-corpus call without a fallback.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[allow(missing_docs)] // self-describing 1:1 mappings of SPARQL built-in names
pub enum Function {
    Str,
    Lang,
    LangMatches,
    Datatype,
    Iri,
    Uri,
    BNode,
    Rand,
    Abs,
    Ceil,
    Floor,
    Round,
    Concat,
    SubStr,
    StrLen,
    Replace,
    UCase,
    LCase,
    EncodeForUri,
    Contains,
    StrStarts,
    StrEnds,
    StrBefore,
    StrAfter,
    Year,
    Month,
    Day,
    Hours,
    Minutes,
    Seconds,
    Timezone,
    Tz,
    Now,
    Uuid,
    StrUuid,
    Md5,
    Sha1,
    Sha256,
    Sha384,
    Sha512,
    StrLang,
    StrDt,
    IsIri,
    IsUri,
    IsBlank,
    IsLiteral,
    IsNumeric,
    Regex,
    /// `TRIPLE(s, p, o)` — RDF 1.2 triple-term constructor.
    Triple,
    /// `SUBJECT(t)` — RDF 1.2 triple-term accessor.
    Subject,
    /// `PREDICATE(t)` — RDF 1.2 triple-term accessor.
    Predicate,
    /// `OBJECT(t)` — RDF 1.2 triple-term accessor.
    Object,
    /// `isTRIPLE(t)` — RDF 1.2 triple-term test.
    IsTriple,
    /// A purrdf extension function (a CLOSED, exhaustive seam, dispatched at parse
    /// time from an IRI under a configured extension-function namespace — default
    /// the canonical purrdf namespace). See [`PurrdfFn`].
    Purrdf(PurrdfFn),
    /// A custom function identified by an arbitrary IRI outside every configured
    /// extension-function namespace.
    Custom(NamedNode),
}

/// The CLOSED set of purrdf SPARQL extension functions.
///
/// Recognized at PARSE time from an IRI under any *configured* extension-function
/// namespace (`{ns}{local-name}`; see
/// [`crate::parser::ParserOptions::extension_fn_namespaces`], whose default is the
/// single published carrier-vocabulary namespace [`crate::PURRDF_NS`]). The set is
/// exhaustive: an IRI under a configured namespace whose local-name is not one of
/// these in call position is a hard parse error, never a [`Function::Custom`].
/// This keeps the purrdf function surface a small, fully-enumerated contract
/// rather than an open custom-IRI escape hatch.
///
/// The namespace is an alias, not part of the identity: `gmeow:heldIn(...)`
/// (parsed with the gmeow namespace configured) and `purrdf:heldIn(...)` dispatch
/// to the same [`PurrdfFn::HeldIn`], and serialization normalizes both back to the
/// default [`crate::PURRDF_NS`] spelling.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PurrdfFn {
    /// `purrdf:heldIn(reifier, standpoint) -> xsd:boolean` — direct (already-reasoned)
    /// standpoint-membership: true iff the reified statement `reifier` is held in
    /// `standpoint` (its vantage standpoint equals, or sharpens, the queried one).
    HeldIn,
    /// `purrdf:listLength(list) -> xsd:integer` — the number of members of an
    /// `rdf:List` (`rdf:nil` is length 0).
    ListLength,
    /// `purrdf:listGet(list, index) -> term` — the member at the zero-based `index`,
    /// or a SPARQL error when the index is out of range.
    ListGet,
    /// `purrdf:listIndexOf(list, value) -> xsd:integer` — the zero-based index of the
    /// first occurrence of `value`, or a SPARQL error when it is absent.
    ListIndexOf,
    /// `purrdf:listContains(list, value) -> xsd:boolean` — whether `value` is a member.
    ListContains,
    /// `purrdf:listSlice(list, start, end) -> rdf:List` — a fresh list of the members
    /// in the half-open index range `[start, end)` (clamped; inverted/out-of-range
    /// yields `rdf:nil`).
    ListSlice,
    /// `purrdf:listConcat(listA, listB) -> rdf:List` — a fresh list of `listA`'s
    /// members followed by `listB`'s.
    ListConcat,
}

impl PurrdfFn {
    /// The purrdf vocabulary local-name (the suffix after [`crate::PURRDF_NS`]) for this
    /// function — used by both the parser (to recognize) and the serializer (to emit).
    #[must_use]
    pub const fn local_name(self) -> &'static str {
        match self {
            Self::HeldIn => "heldIn",
            Self::ListLength => "listLength",
            Self::ListGet => "listGet",
            Self::ListIndexOf => "listIndexOf",
            Self::ListContains => "listContains",
            Self::ListSlice => "listSlice",
            Self::ListConcat => "listConcat",
        }
    }

    /// Map a purrdf vocabulary local-name to its [`PurrdfFn`], or `None` if it is not a
    /// recognized purrdf extension function. The inverse of [`PurrdfFn::local_name`].
    #[must_use]
    pub fn from_local_name(name: &str) -> Option<Self> {
        match name {
            "heldIn" => Some(Self::HeldIn),
            "listLength" => Some(Self::ListLength),
            "listGet" => Some(Self::ListGet),
            "listIndexOf" => Some(Self::ListIndexOf),
            "listContains" => Some(Self::ListContains),
            "listSlice" => Some(Self::ListSlice),
            "listConcat" => Some(Self::ListConcat),
            _ => None,
        }
    }
}

/// A single `ORDER BY` sort key.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum OrderExpression {
    /// Ascending (`ASC(expr)` or a bare expression).
    Asc(Expression),
    /// Descending (`DESC(expr)`).
    Desc(Expression),
}

/// A `GROUP BY` aggregate.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AggregateExpression {
    /// `COUNT(*)`.
    CountStar {
        /// Whether `DISTINCT` was present.
        distinct: bool,
    },
    /// An aggregate over an expression, e.g. `SUM(?x)` or `COUNT(DISTINCT ?x)`.
    FunctionCall {
        /// Which aggregate function.
        function: AggregateFunction,
        /// The aggregated expression.
        expression: Box<Expression>,
        /// Whether `DISTINCT` was present.
        distinct: bool,
    },
}

/// The named SPARQL aggregate functions.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AggregateFunction {
    /// `COUNT`.
    Count,
    /// `SUM`.
    Sum,
    /// `AVG`.
    Avg,
    /// `MIN`.
    Min,
    /// `MAX`.
    Max,
    /// `SAMPLE`.
    Sample,
    /// `GROUP_CONCAT`, with an optional `SEPARATOR`.
    GroupConcat {
        /// The `SEPARATOR` string, if given.
        separator: Option<String>,
    },
    /// A custom aggregate identified by IRI.
    Custom(NamedNode),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nn(iri: &str) -> NamedNode {
        NamedNode::new_unchecked(iri)
    }

    /// A concrete (variable-free) data quad, for the DATA Display tests.
    fn data_quad(graph: Option<NamedNodePattern>) -> QuadPattern {
        QuadPattern {
            triple: TriplePattern {
                subject: TermPattern::NamedNode(nn("http://ex/s")),
                predicate: NamedNodePattern::NamedNode(nn("http://ex/p")),
                object: TermPattern::NamedNode(nn("http://ex/o")),
            },
            graph,
        }
    }

    fn quad_pattern(graph: Option<NamedNodePattern>) -> QuadPattern {
        QuadPattern {
            triple: TriplePattern {
                subject: TermPattern::Variable(Variable::new("s")),
                predicate: NamedNodePattern::NamedNode(nn("http://ex/p")),
                object: TermPattern::Variable(Variable::new("o")),
            },
            graph,
        }
    }

    #[test]
    fn graph_target_display() {
        assert_eq!(GraphTarget::Default.to_string(), "DEFAULT");
        assert_eq!(GraphTarget::NamedGraphs.to_string(), "NAMED");
        assert_eq!(GraphTarget::All.to_string(), "ALL");
        assert_eq!(
            GraphTarget::Named(nn("http://ex/g")).to_string(),
            "GRAPH <http://ex/g>"
        );
    }

    #[test]
    fn insert_data_display() {
        let op = GraphUpdateOperation::InsertData {
            data: vec![data_quad(None)],
        };
        assert_eq!(
            op.to_string(),
            "INSERT DATA { <http://ex/s> <http://ex/p> <http://ex/o> . }"
        );
    }

    #[test]
    fn delete_data_display_with_graph() {
        let op = GraphUpdateOperation::DeleteData {
            data: vec![data_quad(Some(NamedNodePattern::NamedNode(nn(
                "http://ex/g",
            ))))],
        };
        assert_eq!(
            op.to_string(),
            "DELETE DATA { GRAPH <http://ex/g> { <http://ex/s> <http://ex/p> <http://ex/o> . } }"
        );
    }

    #[test]
    fn delete_insert_display() {
        let op = GraphUpdateOperation::DeleteInsert {
            delete: vec![quad_pattern(None)],
            insert: vec![quad_pattern(Some(NamedNodePattern::NamedNode(nn(
                "http://ex/g",
            ))))],
            with: Some(nn("http://ex/w")),
            using: vec![
                UsingClause::Default(nn("http://ex/u")),
                UsingClause::Named(nn("http://ex/n")),
            ],
            pattern: Box::new(GraphPattern::Bgp { patterns: vec![] }),
        };
        let s = op.to_string();
        assert!(s.starts_with("WITH <http://ex/w> "), "{s}");
        assert!(s.contains("DELETE { ?s <http://ex/p> ?o . }"), "{s}");
        assert!(
            s.contains("INSERT { GRAPH <http://ex/g> { ?s <http://ex/p> ?o . } }"),
            "{s}"
        );
        assert!(s.contains("USING <http://ex/u> "), "{s}");
        // The NAMED modifier must survive (previously collapsed to a bare USING).
        assert!(s.contains("USING NAMED <http://ex/n> "), "{s}");
        assert!(s.contains("WHERE {"), "{s}");
    }

    #[test]
    fn load_display() {
        let bare = GraphUpdateOperation::Load {
            silent: false,
            source: nn("http://ex/doc"),
            destination: GraphTarget::Default,
        };
        assert_eq!(bare.to_string(), "LOAD <http://ex/doc>");

        let into = GraphUpdateOperation::Load {
            silent: true,
            source: nn("http://ex/doc"),
            destination: GraphTarget::Named(nn("http://ex/g")),
        };
        assert_eq!(
            into.to_string(),
            "LOAD SILENT <http://ex/doc> INTO GRAPH <http://ex/g>"
        );
    }

    #[test]
    fn clear_drop_display() {
        let clear = GraphUpdateOperation::Clear {
            silent: false,
            target: GraphTarget::All,
        };
        assert_eq!(clear.to_string(), "CLEAR ALL");

        let drop = GraphUpdateOperation::Drop {
            silent: true,
            target: GraphTarget::Named(nn("http://ex/g")),
        };
        assert_eq!(drop.to_string(), "DROP SILENT GRAPH <http://ex/g>");
    }

    #[test]
    fn create_display() {
        let op = GraphUpdateOperation::Create {
            silent: false,
            graph: nn("http://ex/g"),
        };
        assert_eq!(op.to_string(), "CREATE GRAPH <http://ex/g>");
    }

    #[test]
    fn add_move_copy_display() {
        let add = GraphUpdateOperation::Add {
            silent: false,
            source: GraphTarget::Default,
            destination: GraphTarget::Named(nn("http://ex/g")),
        };
        assert_eq!(add.to_string(), "ADD DEFAULT TO GRAPH <http://ex/g>");

        let mv = GraphUpdateOperation::Move {
            silent: true,
            source: GraphTarget::Named(nn("http://ex/a")),
            destination: GraphTarget::Named(nn("http://ex/b")),
        };
        assert_eq!(
            mv.to_string(),
            "MOVE SILENT GRAPH <http://ex/a> TO GRAPH <http://ex/b>"
        );

        let cp = GraphUpdateOperation::Copy {
            silent: false,
            source: GraphTarget::Named(nn("http://ex/a")),
            destination: GraphTarget::Default,
        };
        assert_eq!(cp.to_string(), "COPY GRAPH <http://ex/a> TO DEFAULT");
    }

    #[test]
    fn update_joins_operations_with_semicolon() {
        let upd = Update {
            operations: vec![
                GraphUpdateOperation::Create {
                    silent: false,
                    graph: nn("http://ex/g"),
                },
                GraphUpdateOperation::Clear {
                    silent: false,
                    target: GraphTarget::Default,
                },
            ],
            base_iri: None,
        };
        assert_eq!(
            upd.to_string(),
            "CREATE GRAPH <http://ex/g> ; CLEAR DEFAULT"
        );
    }

    #[test]
    fn update_renders_base_iri() {
        let upd = Update {
            operations: vec![GraphUpdateOperation::Clear {
                silent: false,
                target: GraphTarget::All,
            }],
            base_iri: Some(nn("http://ex/base")),
        };
        assert_eq!(upd.to_string(), "BASE <http://ex/base> CLEAR ALL");
    }
}

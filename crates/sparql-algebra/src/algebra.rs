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
//! ## S6 extension seam
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

/// The SPARQL version an in-prologue `VERSION "<string>"` declaration named
/// (SPARQL 1.2 Query specification §4.4 / grammar production `Version`).
///
/// Two spellings are recognized and get their own variant; anything else is
/// retained verbatim in [`Self::Other`] rather than rejected — the `VERSION`
/// clause is **syntax-only** (parsing never rejects an unrecognized string; see
/// vendored W3C `w3c-sparql12` `version-04.rq`, which declares `"1.1"` and is a
/// `PositiveSyntaxTest`). [`Self::raw`] returns the declared string byte-exactly
/// for every variant, including the two recognized ones, whose raw spelling is
/// always identical to their canonical one (recognition is an exact match).
///
/// A prologue may repeat the declaration (the grammar's own `Version*`); the
/// parser records the LAST one when several appear (an owned reading — the
/// SPARQL 1.2 Query specification does not itself state a tie-breaking rule for
/// a request that legally re-declares the clause).
///
/// # What evaluation does with each
///
/// Recognition is enforced at evaluation ADMISSION, not at parse time — by
/// `purrdf-sparql-eval`'s `admit_version`, the ONE function both the query-evaluation
/// entry point and the update-evaluation entry point call, so a query and an UPDATE
/// declaring the same unrecognized version are refused identically (an UPDATE's
/// refusal, in particular, applies no mutation):
///
/// - [`Self::V12`] evaluates normally on the full engine.
/// - [`Self::V12Basic`] is admitted, then walked: the SPARQL 1.2 Query
///   specification's §4.3.1 "Version Labels" table defines `1.2-basic` as
///   `1.2` syntax "without triple terms and without triple patterns that have
///   a triple pattern in their subject or object position" — the RDF 1.2
///   triple-term/reification feature area (`<<( s p o )>>`, `<< s p o >>`,
///   `{| ... |}`, and the `TRIPLE`/`isTRIPLE`/`SUBJECT`/`PREDICATE`/`OBJECT`
///   functions on triple terms, §17.4.6). `purrdf-sparql-eval`'s
///   `basic_profile` module enforces exactly that restriction (see its docs
///   for the full spec citation and the gated construct set); a `1.2-basic`
///   request that uses none of those constructs evaluates exactly as a `1.2`
///   one would, and one that does use one of them is refused at admission,
///   naming the offending construct.
/// - [`Self::Other`] is refused at admission with a typed error naming the
///   declared version — an unrecognized `VERSION` names a spec this evaluator
///   does not know how to honor, so admitting it would silently evaluate (or,
///   for an UPDATE, silently mutate) under the wrong (or an unknown) semantics.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum SparqlVersion {
    /// `VERSION "1.2"` — the SPARQL 1.2 Query specification, full profile.
    V12,
    /// `VERSION "1.2-basic"` — the SPARQL 1.2 Query specification's Basic profile.
    V12Basic,
    /// Any other declared string, retained verbatim. Refused at evaluation
    /// admission (see the type docs); never a parse error.
    Other(String),
}

impl SparqlVersion {
    /// Classify a declared `VERSION` string, recognizing `"1.2"` and
    /// `"1.2-basic"`; anything else becomes [`Self::Other`] verbatim.
    #[must_use]
    pub fn parse(raw: &str) -> Self {
        match raw {
            "1.2" => Self::V12,
            "1.2-basic" => Self::V12Basic,
            other => Self::Other(other.to_owned()),
        }
    }

    /// The declared version string, byte-exactly as it appeared in the
    /// `VERSION "<string>"` declaration — including for the two recognized
    /// variants, whose raw spelling is always their canonical one.
    #[must_use]
    pub fn raw(&self) -> &str {
        match self {
            Self::V12 => "1.2",
            Self::V12Basic => "1.2-basic",
            Self::Other(s) => s,
        }
    }

    /// Is this a version this evaluator recognizes and evaluates normally?
    #[must_use]
    pub fn is_recognized(&self) -> bool {
        !matches!(self, Self::Other(_))
    }
}

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
        /// The prologue's `VERSION` declaration, if any (last-wins; see [`SparqlVersion`]).
        version: Option<SparqlVersion>,
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
        /// The prologue's `VERSION` declaration, if any (last-wins; see [`SparqlVersion`]).
        version: Option<SparqlVersion>,
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
        /// The prologue's `VERSION` declaration, if any (last-wins; see [`SparqlVersion`]).
        version: Option<SparqlVersion>,
    },
    /// `ASK` query.
    Ask {
        /// The `WHERE` algebra.
        pattern: GraphPattern,
        /// The `FROM` / `FROM NAMED` dataset clause (empty = the store's default).
        dataset: QueryDataset,
        /// An explicit `BASE` IRI, if any.
        base_iri: Option<NamedNode>,
        /// The prologue's `VERSION` declaration, if any (last-wins; see [`SparqlVersion`]).
        version: Option<SparqlVersion>,
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

    /// The query's effective base IRI (an explicit `BASE` decl, or the
    /// caller-supplied document base the parser was constructed with) — the base
    /// against which a runtime `IRI()`/`URI()` call resolves its string argument
    /// (SPARQL 1.1 §17.4.2.6). `None` when neither was ever supplied.
    pub fn base_iri(&self) -> Option<&NamedNode> {
        match self {
            Self::Select { base_iri, .. }
            | Self::Construct { base_iri, .. }
            | Self::Describe { base_iri, .. }
            | Self::Ask { base_iri, .. } => base_iri.as_ref(),
        }
    }

    /// The query's `VERSION` declaration, if the prologue declared one (last-wins
    /// across repeated declarations; see [`SparqlVersion`]).
    pub fn version(&self) -> Option<&SparqlVersion> {
        match self {
            Self::Select { version, .. }
            | Self::Construct { version, .. }
            | Self::Describe { version, .. }
            | Self::Ask { version, .. } => version.as_ref(),
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
    /// The prologue's `VERSION` declaration, if any (last-wins across repeated
    /// declarations, including one that follows a `;` operation separator; see
    /// [`SparqlVersion`]).
    pub version: Option<SparqlVersion>,
}

impl Update {
    /// The request's `VERSION` declaration, if the prologue declared one
    /// (last-wins across repeated declarations; see [`SparqlVersion`]).
    #[must_use]
    pub fn version(&self) -> Option<&SparqlVersion> {
        self.version.as_ref()
    }
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
    ///
    /// The `WHERE` clause of a [`Self::DeleteInsert`] is rendered through
    /// `crate::serialize::fmt_group_body` — the SAME group-graph-pattern
    /// renderer [`crate::pattern_to_select_query`] uses for a query's own WHERE
    /// body — rather than a second, duplicate pattern-to-text implementation.
    /// The two call sites want the identical shape: [`crate::parser::SparqlParser`]
    /// parses an UPDATE's `WHERE { … }` with the very same
    /// `parse_group_graph_pattern` a `SELECT`'s `WHERE { … }` goes through, so
    /// anything the query-side renderer can reproduce (including a `LATERAL`
    /// join, now legal in an UPDATE WHERE clause) the update-side renderer must
    /// reproduce too. The output round-trips through
    /// [`crate::parser::SparqlParser::parse_update`]: `parse_update(op.to_string())`
    /// reproduces `op` for every variant of this enum.
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
                if delete.is_empty() && insert.is_empty() {
                    // The `Modify` grammar (§3, `[43]`) requires at least one of
                    // `DeleteClause`/`InsertClause` — an empty template on BOTH
                    // sides (a legal, if useless, `DELETE WHERE { }` / `INSERT { }
                    // WHERE { }` / bare `WITH … WHERE { }`) still needs ONE emitted
                    // or the text is not valid Update syntax at all. Which keyword
                    // is chosen is immaterial: re-parsing either yields the same
                    // `delete: vec![], insert: vec![]` this operation already
                    // carries, so `INSERT { }` is picked unconditionally here.
                    write!(f, "INSERT {{ }} ")?;
                } else {
                    if !delete.is_empty() {
                        write!(f, "DELETE {{ {} }} ", fmt_quad_pattern_body(delete))?;
                    }
                    if !insert.is_empty() {
                        write!(f, "INSERT {{ {} }} ", fmt_quad_pattern_body(insert))?;
                    }
                }
                for u in using {
                    write!(f, "{u} ")?;
                }
                let mut body = String::new();
                crate::serialize::fmt_group_body(&mut body, pattern);
                write!(f, "WHERE {{ {body} }}")
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
    /// A **property-function** call — a predicate IRI under a *caller-configured*
    /// property-function namespace that invokes a registered RELATION instead of
    /// matching data in the graph (`subjectArgs <iri> objectArgs`).
    ///
    /// # Scope
    ///
    /// Every variable in either argument vector is VISIBLE in the enclosing group
    /// graph pattern: the arguments are simultaneously the call's inputs and its
    /// bindings (which side is input and which is output is decided per relation
    /// at evaluation time, not here), so `SELECT *` projects them and a `FILTER`
    /// in the same group sees them.
    ///
    /// # Argument grammar
    ///
    /// Each side of the predicate is an argument VECTOR:
    ///
    /// * a plain term — IRI, literal, blank node, variable, or an RDF 1.2 quoted
    ///   triple — is a ONE-element vector;
    /// * a collection `( … )` is the vector of its elements, taken STRUCTURALLY:
    ///   it is NOT desugared into `rdf:first`/`rdf:rest` cons cells the way a
    ///   collection in ordinary triple position is;
    /// * the empty collection `()` is a ZERO-length vector, whereas a bare
    ///   `rdf:nil` spelled as an IRI is a one-element vector holding that IRI —
    ///   the two spellings are deliberately distinct;
    /// * a nested collection inside an argument list is a hard parse error;
    /// * a blank node is admitted as a non-distinguished variable (the evaluator
    ///   gives it its synthetic-slot treatment); the parser passes it through.
    ///
    /// A repeated variable — within one side or across both — is passed through
    /// as written; the equality semantics it implies belong to the evaluator.
    ///
    /// # Only ever produced from configuration
    ///
    /// The parser mints this node ONLY when the predicate IRI matches an entry of
    /// [`crate::parser::ParserOptions::property_fn_namespaces`] (prefix match) OR
    /// [`crate::parser::ParserOptions::property_fn_iris`] (exact match), both of
    /// which default to EMPTY. With neither configured the seam is off and such a
    /// triple is an ordinary BGP triple pattern — PurRDF mints no vocabulary IRIs
    /// of its own, so nothing is ever recognized by default.
    PropertyFunction(PropertyFunctionCall),
}

/// A property-function call resolved at parse time: the predicate IRI plus the
/// subject-side and object-side argument vectors.
///
/// See [`GraphPattern::PropertyFunction`] for the scope rule and the argument
/// grammar. The IRI is retained BYTE-EXACT as the query author spelled it (after
/// prefix/base resolution) so serialization re-emits exactly that IRI and never
/// fabricates a namespace — the same contract as [`PurrdfCall::iri`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PropertyFunctionCall {
    /// The predicate IRI the call was parsed from, byte-exact.
    pub iri: String,
    /// The subject-side arguments, in written order (possibly empty).
    pub subject_args: Vec<TermPattern>,
    /// The object-side arguments, in written order (possibly empty).
    pub object_args: Vec<TermPattern>,
}

/// One element of a negated property set list (`!(p1|^p2|...)`, SPARQL 1.1
/// §18.2 grammar production `PathOneInPropertySet`): a predicate IRI plus
/// whether it was written with a leading `^`.
///
/// A plain element (`inverse: false`) excludes that predicate from the
/// **forward** hop; a `^`-prefixed element (`inverse: true`) excludes it from
/// the **reverse** hop. Per §18.3's evaluation semantics, a negated set with
/// both kinds of element decomposes into the union (`Alternative`) of a
/// forward-only negated step over the plain elements and a reverse-only
/// negated step (`Reverse(NegatedPropertySet(inverse elements))`) over the
/// `^`-elements — see `sparql-eval`'s `path` module for the evaluator.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct NegatedPathElement {
    /// The excluded predicate IRI.
    pub predicate: NamedNode,
    /// `true` for a `^iri` element (excludes a reverse hop); `false` for a
    /// plain `iri` element (excludes a forward hop).
    pub inverse: bool,
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
    /// `!(p1|...|pn)` — negated property set, each element optionally inverted
    /// (`^pi`, SPARQL 1.1 §18.2/§18.3). See [`NegatedPathElement`].
    NegatedPropertySet(Vec<NegatedPathElement>),
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
    /// (predicate wildcard — **emit-only**: constructed via the algebra API and
    /// evaluated directly by the engine (`sparql-eval::path`), but the parser has
    /// no grammar production for it, so this text does not round-trip back
    /// through [`crate::parser::SparqlParser`]).
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NamedNode(n) => write!(f, "<{}>", n.as_str()),
            Self::Reverse(a) => write!(f, "^{}", PathElt(a)),
            // `/` (this arm) binds TIGHTER than `|`, and both are
            // left-associative: an `Alternative` operand on EITHER side needs
            // parens (it would otherwise mis-group into the surrounding `|`
            // on re-parse — `crates/sparql-algebra/tests/serializer_roundtrip_sweep.rs`'s
            // `property-path/path-p2.rq`/`path-p4.rq` findings, e.g.
            // `(p1|p2)/(p3|p4)` rendered bare as `p1|p2/p3|p4` reparses as
            // `(p1|(p2/p3))|p4`); a `Sequence` operand needs parens ONLY on
            // the RIGHT (the left one reproduces via `/`'s own
            // left-associativity — `a/b/c` IS `Sequence(Sequence(a,b),c)`
            // already — but a RIGHT-nested `Sequence(a, Sequence(b,c))`
            // rendered bare as `a/b/c` would reparse LEFT-nested instead).
            Self::Sequence(a, b) => write!(f, "{}/{}", SeqLeft(a), SeqRight(b)),
            // `|`'s own left operand never needs parens (lowest precedence,
            // left-associative — any shape reproduces bare); the right
            // operand needs them only for a nested `Alternative` (the same
            // right-nesting-vs-left-associativity mismatch `Sequence` has).
            Self::Alternative(a, b) => write!(f, "{a}|{}", AltRight(b)),
            Self::ZeroOrMore(a) => write!(f, "{}*", QuantifierOperand(a)),
            Self::OneOrMore(a) => write!(f, "{}+", QuantifierOperand(a)),
            Self::ZeroOrOne(a) => write!(f, "{}?", QuantifierOperand(a)),
            Self::Range { inner, min, max } => match max {
                Some(m) if *m == *min => write!(f, "{}{{{min}}}", QuantifierOperand(inner)),
                Some(m) => write!(f, "{}{{{min},{m}}}", QuantifierOperand(inner)),
                None => write!(f, "{}{{{min},}}", QuantifierOperand(inner)),
            },
            Self::NegatedPropertySet(elems) => {
                let inner = elems
                    .iter()
                    .map(|e| {
                        if e.inverse {
                            format!("^<{}>", e.predicate.as_str())
                        } else {
                            format!("<{}>", e.predicate.as_str())
                        }
                    })
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

/// Wraps a property path in parentheses when it must be grouped to sit as
/// `^`'s (`Reverse`'s) operand — i.e. when it is a sequence or alternative
/// path (lower precedence than `^`, needs disambiguating) or ITSELF another
/// inverse (`^^p` needs `^(^p)` to stay two `Reverse`s rather than folding
/// however a double-`^` might otherwise lex/parse).
///
/// A QUANTIFIED path (`ZeroOrMore`/`OneOrMore`/`ZeroOrOne`/`Range`) is
/// DELIBERATELY NOT in this list: the postfix quantifiers bind tighter than
/// `^` in the SPARQL grammar — `parse_path_elt_or_inverse` applies `^` and
/// then delegates to `parse_path_elt` for the quantified primary, so
/// `^<p>*` reparses as `Reverse(ZeroOrMore(<p>))` (not
/// `ZeroOrMore(Reverse(<p>))`) WITHOUT any parens needed — the grammar rule
/// `^` sits in already expects a possibly-quantified primary right after it.
/// [`QuantifierOperand`] is the analogous wrapper for the OTHER context
/// (a quantifier's own operand), which has different rules — see its doc.
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

/// Wraps a property path in parentheses when it must be grouped to sit as a
/// postfix quantifier's (`*`/`+`/`?`/`{n,m}`) OWN operand — a DIFFERENT
/// context from [`PathElt`] (`^`'s operand), with a wider parenthesize set:
/// a `Sequence`/`Alternative`/`Reverse` operand needs the same
/// disambiguation `PathElt` gives `^`, but ALSO a NESTED quantified path
/// (`ZeroOrMore`/`OneOrMore`/`ZeroOrOne`/`Range`) does here, unlike under
/// `^` — `PathMod` attaches to exactly one `PathPrimary`, and an
/// already-quantified path is not one, so chaining two quantifiers directly
/// (`p**`, the un-parenthesized spelling `(p*)*` would otherwise collapse
/// to) is not even valid surface syntax
/// (`crates/sparql-algebra/tests/serializer_roundtrip_sweep.rs`'s
/// `property-path/pp37.rq` finding: `((:P)*)*` used to render as the
/// un-reparseable `<P>**`). Parenthesizing the inner quantified path makes it
/// a primary again, exactly like `PathElt` does for `Sequence`/`Alternative`.
struct QuantifierOperand<'a>(&'a PropertyPathExpression);

impl core::fmt::Display for QuantifierOperand<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.0 {
            PropertyPathExpression::Sequence(..)
            | PropertyPathExpression::Alternative(..)
            | PropertyPathExpression::Reverse(..)
            | PropertyPathExpression::ZeroOrMore(..)
            | PropertyPathExpression::OneOrMore(..)
            | PropertyPathExpression::ZeroOrOne(..)
            | PropertyPathExpression::Range { .. } => {
                write!(f, "({})", self.0)
            }
            other => write!(f, "{other}"),
        }
    }
}

/// The LEFT operand of `/` (`Sequence`): parens only for `Alternative` (lower
/// precedence — see [`PropertyPathExpression`]'s `Display`'s `Sequence` arm
/// for the full precedence argument). A nested `Sequence` reproduces bare via
/// `/`'s own left-associativity.
struct SeqLeft<'a>(&'a PropertyPathExpression);

impl core::fmt::Display for SeqLeft<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.0 {
            PropertyPathExpression::Alternative(..) => write!(f, "({})", self.0),
            other => write!(f, "{other}"),
        }
    }
}

/// The RIGHT operand of `/` (`Sequence`): parens for `Alternative` (lower
/// precedence) AND for a nested `Sequence` (right-nesting does not survive
/// `/`'s left-associative re-parse — see the `Sequence` `Display` arm).
struct SeqRight<'a>(&'a PropertyPathExpression);

impl core::fmt::Display for SeqRight<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.0 {
            PropertyPathExpression::Alternative(..) | PropertyPathExpression::Sequence(..) => {
                write!(f, "({})", self.0)
            }
            other => write!(f, "{other}"),
        }
    }
}

/// The RIGHT operand of `|` (`Alternative`): parens only for a nested
/// `Alternative` (right-nesting does not survive `|`'s left-associative
/// re-parse). `Sequence` binds tighter and never needs parens on either side
/// of `|`.
struct AltRight<'a>(&'a PropertyPathExpression);

impl core::fmt::Display for AltRight<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.0 {
            PropertyPathExpression::Alternative(..) => write!(f, "({})", self.0),
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
    /// `ADJUST(value, timezone)` — SPARQL 1.2 timezone adjustment for
    /// `xsd:dateTime`/`xsd:date`/`xsd:time`, mapping to XPath and XQuery
    /// Functions and Operators §9.6 `fn:adjust-*-to-timezone` (SEP-0002's
    /// "Add Support Durations, Dates, and Times" addition to the SPARQL 1.2
    /// Query specification's Functions on Dates and Times table). `timezone`
    /// is an `xsd:dayTimeDuration` in `[-PT14H, PT14H]`, or the empty simple
    /// literal `""` — SPARQL's stand-in for XPath's empty-sequence "remove
    /// the timezone" case (SPARQL itself has no empty sequence). See the
    /// eval arm in `purrdf-sparql-eval` for the full domain-error contract.
    Adjust,
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
    /// `LANGDIR(literal)` — RDF 1.2 base-direction accessor (`"ltr"`/`"rtl"`,
    /// or the empty string when the literal carries no base direction).
    LangDir,
    /// `STRLANGDIR(lex, lang, dir)` — RDF 1.2 directional-language-string
    /// constructor (an `rdf:dirLangString`).
    StrLangDir,
    /// `hasLANG(literal)` — RDF 1.2 test: does the literal carry a language tag?
    HasLang,
    /// `hasLANGDIR(literal)` — RDF 1.2 test: does the literal carry a base
    /// direction?
    HasLangDir,
    /// An extension function call (a CLOSED, exhaustive local-name seam, dispatched
    /// at parse time from an IRI under a *caller-configured* extension-function
    /// namespace — there is no default namespace). Carries the original call IRI
    /// so serialization round-trips exactly. See [`PurrdfCall`] and [`PurrdfFn`].
    Purrdf(PurrdfCall),
    /// A custom function identified by an arbitrary IRI outside every configured
    /// extension-function namespace.
    Custom(NamedNode),
}

/// An extension function call resolved at parse time: the closed [`PurrdfFn`]
/// kind plus the ORIGINAL IRI the call was parsed from.
///
/// The original IRI is retained so serialization re-emits exactly the IRI the
/// query author wrote — PurRDF is a library, not an ontology, and never mints
/// or fabricates vocabulary IRIs of its own on output.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PurrdfCall {
    /// The closed extension-function kind this call dispatches to.
    pub fn_kind: PurrdfFn,
    /// The full original IRI the call was parsed from
    /// (`{configured-namespace}{local-name}`). Serialization emits exactly this
    /// IRI — no namespace is ever fabricated on output.
    pub iri: String,
}

impl PurrdfCall {
    /// The extension-function local-name (the suffix after the configured
    /// namespace) — the same as [`PurrdfFn::local_name`] on [`Self::fn_kind`].
    #[must_use]
    pub const fn local_name(&self) -> &'static str {
        self.fn_kind.local_name()
    }
}

/// The CLOSED set of SPARQL extension functions (the type name `PurrdfFn` is an
/// internal identifier; the *namespace* the functions are spelled under is
/// caller configuration, never a purrdf-owned vocabulary).
///
/// Recognized at PARSE time from an IRI under any *configured* extension-function
/// namespace (`{ns}{local-name}`; see
/// [`crate::parser::ParserOptions::extension_fn_namespaces`], whose default is
/// EMPTY — with no configured namespace the seam is off and every call-position
/// IRI is an ordinary [`Function::Custom`]). The local-name set is exhaustive:
/// an IRI under a configured namespace whose local-name is not one of these in
/// call position is a hard parse error, never a [`Function::Custom`]. This keeps
/// the extension-function surface a small, fully-enumerated contract rather than
/// an open custom-IRI escape hatch.
///
/// The namespace is caller configuration, not part of the identity:
/// `gmeow:heldIn(...)` (parsed with the gmeow namespace configured) and
/// `ext:heldIn(...)` (parsed with an `ext:` namespace configured) dispatch to
/// the same [`PurrdfFn::HeldIn`]; serialization re-emits the original IRI
/// recorded in [`PurrdfCall::iri`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PurrdfFn {
    /// `heldIn(reifier, standpoint) -> xsd:boolean` — direct (already-reasoned)
    /// standpoint-membership: true iff the reified statement `reifier` is held in
    /// `standpoint` (its vantage standpoint equals, or sharpens, the queried one).
    HeldIn,
    /// `listLength(list) -> xsd:integer` — the number of members of an
    /// `rdf:List` (`rdf:nil` is length 0).
    ListLength,
    /// `listGet(list, index) -> term` — the member at the zero-based `index`,
    /// or a SPARQL error when the index is out of range.
    ListGet,
    /// `listIndexOf(list, value) -> xsd:integer` — the zero-based index of the
    /// first occurrence of `value`, or a SPARQL error when it is absent.
    ListIndexOf,
    /// `listContains(list, value) -> xsd:boolean` — whether `value` is a member.
    ListContains,
    /// `listSlice(list, start, end) -> rdf:List` — a fresh list of the members
    /// in the half-open index range `[start, end)` (clamped; inverted/out-of-range
    /// yields `rdf:nil`).
    ListSlice,
    /// `listConcat(listA, listB) -> rdf:List` — a fresh list of `listA`'s
    /// members followed by `listB`'s.
    ListConcat,
}

impl PurrdfFn {
    /// The extension-function local-name (the suffix after whichever configured
    /// namespace the call was spelled under) — used by both the parser (to
    /// recognize) and consumers that need the bare name.
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

    /// Map an extension-function local-name to its [`PurrdfFn`], or `None` if it is
    /// not a recognized extension function. The inverse of [`PurrdfFn::local_name`].
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

/// A `GROUP BY` aggregate — SPARQL 1.1 §18.5.1's algebra node
/// `Aggregation(exprlist, func, scalarvals, G)`, carried 1:1 rather than as the
/// ad hoc `CountStar`/`FunctionCall` split this replaces.
///
/// * [`Self::function`] — the aggregate operator: one of the SPARQL 1.1
///   built-ins, or [`AggregateFunction::Custom`] for an extension aggregate
///   parsed from the `AGG(<iri>, …)` surface.
/// * [`Self::args`] — the spec's `exprlist`. Every built-in aggregate except
///   `COUNT` is fixed-arity ONE (`SUM(?x)`, `GROUP_CONCAT(?x)`, `AVG(?x)`, …).
///   `COUNT` is the one variable-shape built-in: `COUNT(?x)` has
///   `args == [?x]`; `COUNT(*)` has the spec's EMPTY exprlist —
///   `args == []` — with NO separate "count-star" variant, exactly mirroring
///   §18.5.1's algebra (a bare `function: Count` with an empty `args` IS
///   `COUNT(*)`; a non-empty `args` is `COUNT(expr)`). A
///   [`AggregateFunction::Custom`] aggregate's arity is whatever the query
///   supplied via the positional `AGG(<iri>, arg, arg, …)` surface — one or
///   more expressions; there is no fixed arity to check structurally, only the
///   parser's "at least one" rule.
/// * [`Self::scalarvals`] — the spec's scalar-values map. An ORDERED
///   `Vec<(key, value)>` — deliberately never a hash map, so serialization and
///   any diagnostic built from it stay byte-deterministic.
///   * A built-in's `scalarvals` uses only the keys the SPARQL grammar itself
///     defines: today, `"separator"` for `GROUP_CONCAT`'s optional
///     `SEPARATOR="…"` (absent — `scalarvals` empty — when no `SEPARATOR` was
///     written). Every other built-in's `scalarvals` is empty.
///   * A [`AggregateFunction::Custom`] aggregate's `scalarvals` holds every
///     `NAME=value` clause the `AGG(<iri>, …; NAME=value; …)` surface's
///     trailing scalarval clauses supplied (see that variant's docs) — empty
///     when the call wrote none. The KEY is the parser's upper-cased spelling
///     of `NAME`; it is NOT validated against any registry here — the parser
///     accepts any name structurally, and whether a given custom aggregate
///     accepts a given name (and whether its value's type is right) is a
///     prepare-time host concern (see `purrdf_sparql_eval::agg_fn::CustomAggregate::scalarvals`),
///     not a parser-level one, exactly as an unregistered `AGG(<iri>, …)` IRI
///     itself is a prepare-time refusal rather than a parse error.
/// * [`Self::distinct`] — whether `DISTINCT` preceded the arguments
///   (`COUNT(DISTINCT *)` / `COUNT(DISTINCT ?x)` / `AGG(<iri>, DISTINCT ?a,
///   ?b)` / …); recorded verbatim regardless of whether the function gives it
///   meaning.
///
/// # Constructing one
///
/// [`Self::new`] is the ONLY way to build a value from scratch: it rejects an
/// empty `args` for any `function` other than [`AggregateFunction::Count`],
/// so `SUM(*)`/`AVG(*)`/`MIN(*)`/`MAX(*)`/`SAMPLE(*)`/`GROUP_CONCAT(*)` — and a
/// zero-arity [`AggregateFunction::Custom`] — cannot be built at all, from
/// inside this crate or out. It also rejects a `scalarvals` key `function`
/// does not admit: only [`AggregateFunction::GroupConcat`] accepts one (the
/// key `"separator"`), every other BUILT-IN accepts none at all, and
/// [`AggregateFunction::Custom`] accepts any key structurally (the closed
/// check against a specific registered aggregate's own declaration is a
/// prepare-time concern in `sparql-eval`, not this crate's). `args`,
/// `function`, AND `scalarvals` are therefore all private outside this
/// crate: [`Self::args`]/[`Self::function`]/[`Self::scalarvals`] (the
/// accessor methods) give read access without opening a second, unchecked
/// write path. A `pub function` field would have let a caller build a valid
/// value through [`Self::new`] and then assign a DIFFERENT `function` into
/// it — e.g. build `COUNT(*)` (empty `args`, legal) and reassign `function =
/// Sum`, producing an in-memory `SUM` with zero args that `new` would have
/// refused outright. A `pub scalarvals` field is the SAME hole one level
/// over: build a legal `SUM(?v)` through [`Self::new`] and then push a
/// `"separator"` entry onto it directly, and [`crate::serialize`]'s
/// `SUM`-branch renderer — which writes every `scalarvals` entry it is
/// handed, by key, for every function alike — emits `SUM(?v; SEPARATOR="…")`,
/// which is not SPARQL grammar for anything: it is unparseable text a
/// checked constructor exists specifically to make unreachable. `distinct`
/// stays `pub`: no `bool` value it could hold makes the serializer emit
/// anything ungrammatical (`SUM(DISTINCT ?v)` parses fine even though
/// `DISTINCT` gives `SUM` no extra meaning), so hiding it would only add
/// call-site friction with no matching safety gain. The three private fields
/// are `pub(crate)` rather than fully private: `serialize.rs`'s formatter
/// and `parser.rs`'s own tests read (and, in tests, pattern-match) them
/// directly elsewhere in this crate, and this crate is small and
/// disciplined enough that a `pub(crate)` seam is an honest tradeoff — the
/// invariant only needs to hold at the crate's public boundary, which `new`
/// alone already guarantees for every external caller (embedders
/// included), since none of the three has a public setter.
///
/// An embedder outside this crate cannot reach past [`Self::new`] to mutate a
/// checked-valid value into one [`crate::serialize`] cannot render — this fails
/// to compile, the same way it would for `args`/`function`:
///
/// ```compile_fail,E0616
/// use purrdf_sparql_algebra::{AggregateExpression, AggregateFunction, Expression, Literal, Variable};
///
/// let mut agg = AggregateExpression::new(
///     AggregateFunction::Sum,
///     vec![Expression::Variable(Variable::new("v"))],
///     Vec::new(),
///     false,
/// )
/// .expect("SUM(?v) is a valid one-argument call");
/// // `scalarvals` is private outside this crate — this line does not compile.
/// agg.scalarvals.push(("separator".to_owned(), Literal::new_simple("|")));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AggregateExpression {
    /// Which aggregate function. Private outside this crate — go through
    /// [`Self::new`] to build one, [`Self::function`] to read it back; see
    /// the struct docs for why a public setter would reopen the arity hole
    /// [`Self::new`] closes.
    pub(crate) function: AggregateFunction,
    /// The aggregate's expression list (the spec's `exprlist`). Empty only for
    /// `COUNT(*)`; see the struct docs. Private outside this crate — go
    /// through [`Self::new`] to build one, [`Self::args`] to read it back.
    pub(crate) args: Vec<Expression>,
    /// The spec's scalar-values map — an ordered, deterministic
    /// `(key, value)` list; see the struct docs. Private outside this crate —
    /// go through [`Self::new`] to build one, [`Self::scalarvals`] to read it
    /// back; see the struct docs for why a public setter would let a
    /// checked-valid value be mutated into one the serializer cannot render.
    pub(crate) scalarvals: Vec<(String, Literal)>,
    /// Whether `DISTINCT` was present.
    pub distinct: bool,
}

impl AggregateExpression {
    /// The checked constructor: the ONLY way to build an [`AggregateExpression`]
    /// from its parts.
    ///
    /// # Errors
    ///
    /// [`AggregateExpressionError::Arity`] if `args` is empty and `function` is
    /// anything other than [`AggregateFunction::Count`] — SPARQL's `'*'`
    /// exprlist shorthand names only `COUNT(*)`/`COUNT(DISTINCT *)`; every
    /// other built-in is fixed-arity one, and every
    /// [`AggregateFunction::Custom`] call is positional-only, one-or-more
    /// (see that variant's docs). This is what makes a re-founded "empty
    /// exprlist" bug like the one this constructor replaces unrepresentable:
    /// the shape that used to be read off `args.is_empty()` — with the
    /// reader trusting an un-enforced comment — is now enforced once, here,
    /// at the only place a value comes into existence.
    ///
    /// [`AggregateExpressionError::Scalarval`] if `scalarvals` carries a key
    /// `function` does not admit — every built-in but
    /// [`AggregateFunction::GroupConcat`] (whose one admitted key is
    /// `"separator"`) admits none at all, so a `scalarvals` entry there is
    /// always refused. [`AggregateFunction::Custom`] admits any key
    /// structurally (the closed check against a specific registered
    /// aggregate's own declaration happens at prepare time, in
    /// `sparql-eval`, against data this crate does not have). This is what
    /// makes handing [`crate::serialize`]'s renderer a value it cannot
    /// render back out — e.g. a `SUM` carrying a `"separator"` entry, which
    /// would emit `SUM(?v; SEPARATOR="…")`, not SPARQL grammar for anything
    /// — unrepresentable, the same way the arity check makes `SUM(*)`
    /// unrepresentable.
    pub fn new(
        function: AggregateFunction,
        args: Vec<Expression>,
        scalarvals: Vec<(String, Literal)>,
        distinct: bool,
    ) -> Result<Self, AggregateExpressionError> {
        if args.is_empty() && !matches!(function, AggregateFunction::Count) {
            return Err(AggregateExpressionError::Arity(AggregateArityError {
                function,
            }));
        }
        if let Some((key, _)) = scalarvals
            .iter()
            .find(|(key, _)| !scalarval_key_is_admitted(&function, key))
        {
            return Err(AggregateExpressionError::Scalarval(
                AggregateScalarvalError {
                    function,
                    key: key.clone(),
                },
            ));
        }
        Ok(Self {
            function,
            args,
            scalarvals,
            distinct,
        })
    }

    /// The aggregate's expression list (the spec's `exprlist`); see the
    /// struct docs. Empty iff [`Self::function`] is
    /// [`AggregateFunction::Count`] (`COUNT(*)`/`COUNT(DISTINCT *)`) —
    /// [`Self::new`] enforces that for every value that exists.
    #[must_use]
    pub fn args(&self) -> &[Expression] {
        &self.args
    }

    /// Which aggregate function this is; see the struct docs. There is no
    /// public setter — mutating `function` after construction without
    /// re-checking `args`'/`scalarvals`' validity is exactly the hole
    /// [`Self::new`] closes, so changing which function a value names means
    /// building a new one through [`Self::new`] (or [`Self::into_parts`]
    /// plus [`Self::new`]).
    #[must_use]
    pub fn function(&self) -> &AggregateFunction {
        &self.function
    }

    /// The spec's scalar-values map; see the struct docs. Every key is one
    /// [`Self::function`] admits — [`Self::new`] enforces that for every
    /// value that exists. There is no public setter — mutating `scalarvals`
    /// after construction without re-checking each key against `function` is
    /// exactly the hole [`Self::new`] closes.
    #[must_use]
    pub fn scalarvals(&self) -> &[(String, Literal)] {
        &self.scalarvals
    }

    /// Decompose into `(function, args, scalarvals, distinct)`, consuming
    /// `self`. The inverse of [`Self::new`] minus its checks — for a caller
    /// (an expression-substitution or query-planning rewrite) that only ever
    /// replaces `args` with a same-length transform of itself and leaves
    /// `function`/`scalarvals` untouched, so neither invariant this type
    /// protects can be disturbed by the round trip: feed the tuple back
    /// through [`Self::new`] and the call cannot fail.
    #[must_use]
    pub fn into_parts(
        self,
    ) -> (
        AggregateFunction,
        Vec<Expression>,
        Vec<(String, Literal)>,
        bool,
    ) {
        (self.function, self.args, self.scalarvals, self.distinct)
    }

    /// `GROUP_CONCAT`'s `SEPARATOR` scalarval, if present (looked up by key in
    /// [`Self::scalarvals`]). `None` for a bare `GROUP_CONCAT(?x)` with no
    /// `; SEPARATOR="…"`, or for any other aggregate.
    #[must_use]
    pub fn separator(&self) -> Option<&str> {
        self.scalarvals
            .iter()
            .find(|(k, _)| k == "separator")
            .map(|(_, v)| v.value())
    }
}

/// Whether `key` is a `scalarvals` entry [`AggregateExpression::new`] admits for
/// `function` — the single source of truth the constructor's validation and this
/// module's docs both describe. [`AggregateFunction::Custom`] admits any key
/// structurally; every other (built-in) function admits none, except
/// [`AggregateFunction::GroupConcat`], which admits exactly `"separator"`.
fn scalarval_key_is_admitted(function: &AggregateFunction, key: &str) -> bool {
    match function {
        AggregateFunction::Custom(_) => true,
        AggregateFunction::GroupConcat => key == "separator",
        AggregateFunction::Count
        | AggregateFunction::Sum
        | AggregateFunction::Avg
        | AggregateFunction::Min
        | AggregateFunction::Max
        | AggregateFunction::Sample => false,
    }
}

/// Why [`AggregateExpression::new`] refused to build a value: either an
/// [`AggregateArityError`] (an empty `args` for a `function` that requires at
/// least one argument) or an [`AggregateScalarvalError`] (a `scalarvals` key
/// `function` does not admit). See [`AggregateExpression::new`]'s `# Errors`
/// section for exactly when each arm fires.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AggregateExpressionError {
    /// See [`AggregateArityError`].
    Arity(AggregateArityError),
    /// See [`AggregateScalarvalError`].
    Scalarval(AggregateScalarvalError),
}

impl AggregateExpressionError {
    /// The function that was refused, regardless of which arm this is —
    /// [`AggregateArityError::function`]/[`AggregateScalarvalError::function`]
    /// under the hood.
    #[must_use]
    pub fn function(&self) -> &AggregateFunction {
        match self {
            Self::Arity(error) => error.function(),
            Self::Scalarval(error) => error.function(),
        }
    }
}

impl core::fmt::Display for AggregateExpressionError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Arity(error) => core::fmt::Display::fmt(error, f),
            Self::Scalarval(error) => core::fmt::Display::fmt(error, f),
        }
    }
}

impl std::error::Error for AggregateExpressionError {}

impl From<AggregateArityError> for AggregateExpressionError {
    fn from(error: AggregateArityError) -> Self {
        Self::Arity(error)
    }
}

impl From<AggregateScalarvalError> for AggregateExpressionError {
    fn from(error: AggregateScalarvalError) -> Self {
        Self::Scalarval(error)
    }
}

/// Why [`AggregateExpression::new`] refused to build a value: `args` was
/// empty for a `function` other than [`AggregateFunction::Count`]. SPARQL's
/// `'*'` exprlist shorthand is defined only in the `Count` production
/// (SPARQL 1.1/1.2 §18.5.1/§19.8); every other aggregate — built-in or
/// [`AggregateFunction::Custom`] — requires at least one expression argument.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AggregateArityError {
    function: AggregateFunction,
}

impl AggregateArityError {
    /// The function that was refused an empty `args`.
    #[must_use]
    pub fn function(&self) -> &AggregateFunction {
        &self.function
    }
}

impl core::fmt::Display for AggregateArityError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "only COUNT accepts an empty exprlist ('*'); {:?} requires at least one argument",
            self.function
        )
    }
}

impl std::error::Error for AggregateArityError {}

/// Why [`AggregateExpression::new`] refused to build a value: `scalarvals`
/// carried a key `function` does not admit. Every built-in but
/// [`AggregateFunction::GroupConcat`] admits no `scalarvals` key at all;
/// `GroupConcat` admits exactly `"separator"`; [`AggregateFunction::Custom`]
/// admits any key (see [`AggregateExpression::new`]'s `# Errors` section).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct AggregateScalarvalError {
    function: AggregateFunction,
    key: String,
}

impl AggregateScalarvalError {
    /// The function that refused `key`.
    #[must_use]
    pub fn function(&self) -> &AggregateFunction {
        &self.function
    }

    /// The `scalarvals` key that was refused.
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }
}

impl core::fmt::Display for AggregateScalarvalError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{:?} does not admit a scalarval named {:?}",
            self.function, self.key
        )
    }
}

impl std::error::Error for AggregateScalarvalError {}

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
    /// `GROUP_CONCAT`. The optional `SEPARATOR` lives on the owning
    /// [`AggregateExpression::scalarvals`] (key `"separator"`), not here — the
    /// spec's scalar-values map is a property of the aggregation node, not of
    /// the function name.
    GroupConcat,
    /// A custom aggregate identified by an arbitrary IRI, parsed from
    /// `AGG(<iri>, [DISTINCT] arg, arg, … [; NAME=value]*)`: positional
    /// expression arguments (`args`, evaluated PER ROW like any built-in
    /// aggregate's arguments), followed by zero or more trailing NAMED
    /// scalar-value clauses (landing in the owning
    /// [`AggregateExpression::scalarvals`]) — ONE value for the whole
    /// aggregation, never re-evaluated per row. `<iri>` may be any IRI,
    /// including a prefixed name resolved against the query's prologue; it is
    /// retained byte-exact so serialization re-emits exactly what the query
    /// author wrote (PurRDF fabricates no vocabulary IRI of its own on
    /// output).
    ///
    /// This is a deliberate divergence from Jena's ARQ, which spells a custom
    /// aggregate as `AGG <iri>(args)` (the IRI directly prefixing the call,
    /// not as the first positional argument); PurRDF places the IRI as the
    /// first positional argument so the call form needs no grammar beyond an
    /// ordinary argument list.
    ///
    /// # The `; NAME=value` scalarval clause
    ///
    /// Generalizes SPARQL's own precedent for a named scalar aggregate
    /// parameter — `GROUP_CONCAT`'s `; SEPARATOR="…"` — to an arbitrary custom
    /// aggregate's own named parameters: `AGG(<{NS}PERCENTILE>, ?v; P=0.95)`,
    /// `AGG(<{NS}TOPK>, ?v; K=3)`. `NAME` is matched case-insensitively by the
    /// parser and stored upper-cased in [`AggregateExpression::scalarvals`], so
    /// `; p=0.95` and `; P=0.95` normalize to the same key; `value` is any
    /// SPARQL literal, so a numeric scalarval parses to its natural numeric
    /// datatype rather than being forced through a string the way
    /// `GROUP_CONCAT`'s own `SEPARATOR` is. See
    /// [`crate::parser::SparqlParser`]'s module docs for the grammar and
    /// `purrdf_sparql_eval::agg_fn::CustomAggregate::scalarvals` for how a
    /// registered aggregate declares which names it accepts.
    ///
    /// Evaluation resolves the IRI against a caller-supplied aggregate
    /// registry in `sparql-eval`; an unregistered IRI, an unrecognized
    /// scalarval name, a duplicate scalarval name, a missing required
    /// scalarval, or a wrong-typed scalarval value is refused with a typed
    /// error when the query is prepared, before any evaluation work is spent.
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
            version: None,
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
            version: None,
        };
        assert_eq!(upd.to_string(), "BASE <http://ex/base> CLEAR ALL");
    }

    // ── `Display for GraphUpdateOperation` round-trips through the real parser ──
    //
    // These prove `Display` emits genuine, re-parseable SPARQL Update surface
    // syntax (not the `{pattern:?}` Debug dump it used to) — one test per enum
    // arm at minimum, `parse_update(op.to_string()) == op` for a fresh op built
    // by the real parser (never hand-built, so the comparison exercises the
    // exact structure `parse_update_operation` produces).

    /// Parse `text` as a single-operation Update and return that one operation.
    fn parse_single_op(text: &str) -> GraphUpdateOperation {
        let update = crate::parser::SparqlParser::new()
            .parse_update(text)
            .unwrap_or_else(|e| panic!("failed to parse {text:?}: {e}"));
        assert_eq!(update.operations.len(), 1, "expected exactly one operation");
        update
            .operations
            .into_iter()
            .next()
            .expect("checked len == 1")
    }

    /// Parse `text`, `Display` the resulting operation, re-parse that text, and
    /// assert the two operations are equal. Returns the rendered text so callers
    /// can inspect it further (e.g. assert it is free of Debug punctuation).
    fn assert_op_round_trips(text: &str) -> String {
        let op = parse_single_op(text);
        let rendered = op.to_string();
        let reparsed = parse_single_op(&rendered);
        assert_eq!(
            op, reparsed,
            "round trip mismatch\n  original text: {text}\n  rendered:      {rendered}"
        );
        rendered
    }

    #[test]
    fn insert_data_round_trips() {
        assert_op_round_trips(
            "INSERT DATA { <http://ex/s> <http://ex/p> <http://ex/o> . \
             GRAPH <http://ex/g> { <http://ex/s2> <http://ex/p2> <http://ex/o2> . } }",
        );
    }

    #[test]
    fn delete_data_round_trips() {
        assert_op_round_trips("DELETE DATA { <http://ex/s> <http://ex/p> <http://ex/o> . }");
    }

    #[test]
    fn delete_where_round_trips() {
        assert_op_round_trips("DELETE WHERE { ?s <http://ex/p> ?o . ?o <http://ex/q> ?x . }");
    }

    #[test]
    fn insert_where_with_a_lateral_round_trips() {
        let rendered = assert_op_round_trips(
            "INSERT { ?s <http://ex/q> ?x } WHERE { \
             ?s <http://ex/p> ?o . LATERAL { ?o <http://ex/r> ?x } }",
        );
        // The rendered WHERE clause must be real SPARQL, not a Debug dump: no
        // `Bgp {`/`TriplePattern {`/`Lateral {` Rust-struct punctuation, and the
        // `LATERAL` keyword itself must survive into the surface text.
        assert!(!rendered.contains("Bgp {"), "{rendered}");
        assert!(!rendered.contains("TriplePattern"), "{rendered}");
        assert!(rendered.contains("LATERAL"), "{rendered}");
    }

    #[test]
    fn with_delete_insert_where_round_trips() {
        assert_op_round_trips(
            "WITH <http://ex/g> DELETE { ?s <http://ex/p> ?o } INSERT { ?s <http://ex/q> ?o } \
             USING <http://ex/u> USING NAMED <http://ex/n> WHERE { ?s <http://ex/p> ?o }",
        );
    }

    #[test]
    fn insert_only_where_with_empty_template_round_trips() {
        // `delete` and `insert` are BOTH empty here — the one shape `Display`
        // must still emit a syntactically valid `Modify` (at least one of
        // `DeleteClause`/`InsertClause`), never a bare `WHERE { … }`.
        assert_op_round_trips("INSERT { } WHERE { ?s <http://ex/p> ?o }");
    }

    #[test]
    fn load_round_trips() {
        assert_op_round_trips("LOAD SILENT <http://ex/doc> INTO GRAPH <http://ex/g>");
        assert_op_round_trips("LOAD <http://ex/doc>");
    }

    #[test]
    fn clear_round_trips() {
        assert_op_round_trips("CLEAR SILENT ALL");
        assert_op_round_trips("CLEAR GRAPH <http://ex/g>");
    }

    #[test]
    fn drop_round_trips() {
        assert_op_round_trips("DROP SILENT NAMED");
        assert_op_round_trips("DROP DEFAULT");
    }

    #[test]
    fn create_round_trips() {
        assert_op_round_trips("CREATE SILENT GRAPH <http://ex/g>");
    }

    #[test]
    fn add_round_trips() {
        assert_op_round_trips("ADD SILENT DEFAULT TO GRAPH <http://ex/g>");
    }

    #[test]
    fn move_round_trips() {
        assert_op_round_trips("MOVE GRAPH <http://ex/a> TO GRAPH <http://ex/b>");
    }

    #[test]
    fn copy_round_trips() {
        assert_op_round_trips("COPY SILENT GRAPH <http://ex/a> TO DEFAULT");
    }

    #[test]
    fn update_display_round_trips_a_multi_operation_request() {
        let text = "BASE <http://ex/base> \
                     INSERT DATA { <http://ex/s> <http://ex/p> <http://ex/o> } ; \
                     DELETE WHERE { ?s <http://ex/p> ?o } ; \
                     CLEAR ALL";
        let update = crate::parser::SparqlParser::new()
            .parse_update(text)
            .unwrap_or_else(|e| panic!("failed to parse {text:?}: {e}"));
        let rendered = update.to_string();
        let reparsed = crate::parser::SparqlParser::new()
            .parse_update(&rendered)
            .unwrap_or_else(|e| panic!("failed to re-parse {rendered:?}: {e}"));
        assert_eq!(update, reparsed);
    }
}

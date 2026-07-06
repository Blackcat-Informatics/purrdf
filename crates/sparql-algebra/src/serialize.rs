// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Algebra → SPARQL **surface-text** serialization.
//!
//! The inverse of [`crate::parser`]: it renders a [`GraphPattern`] back to a
//! complete `SELECT * WHERE { ... }` query string. The driving use case is
//! SPARQL `SERVICE` federation: the evaluator forwards a federated sub-pattern
//! to a remote endpoint as a complete query, and that requires re-materializing
//! the algebra as text.
//!
//! # Design
//!
//! * Pure `core::fmt::Write` into a `String` — **wasm-clean**, no std-only deps,
//!   reusing the existing [`crate::algebra::PropertyPathExpression`] `Display`
//!   for paths.
//! * **Round-trips** with the parser: `parse(serialize(p))` reproduces `p` for
//!   every [`GraphPattern`]/[`Expression`] variant the parser emits. Expressions
//!   are conservatively fully parenthesized — over-parenthesization is a no-op on
//!   re-parse, so correctness never depends on reproducing the exact precedence.
//! * Solution-modifier nodes (`Project`/`Distinct`/`Reduced`/`Slice`/`OrderBy`/
//!   `Group`) re-materialize as a braced **sub-`SELECT`** `{ SELECT ... }`, the
//!   shape the parser produces for an inline subquery.
//!
//! The PurRDF predicate-wildcard path extension (`<any>`) is emit-only (no parse),
//! exactly as documented on [`crate::algebra::PropertyPathExpression`]'s
//! `Display`; a path carrying it does not round-trip, which is the established
//! contract.

use core::fmt::Write as _;

use crate::algebra::{AggregateExpression, Expression, Function, GraphPattern, OrderExpression};
use crate::ast::{
    BaseDirection, GroundTerm, GroundTriple, Literal, NamedNodePattern, RDF_LANG_STRING,
    TermPattern, TriplePattern, Variable, XSD_STRING,
};

/// A `GROUP BY` key + its `(output var, aggregate)` pairs, borrowed from a
/// [`GraphPattern::Group`] node during sub-`SELECT` reconstruction.
type GroupSpec<'a> = (&'a [Variable], &'a [(Variable, AggregateExpression)]);

// `AggregateFunction` is referenced only by the test-only aggregate renderer.
#[cfg(test)]
use crate::algebra::AggregateFunction;

/// Render `inner` as a complete `SELECT * WHERE { … }` query string.
///
/// This is the entry point SERVICE federation uses to forward a sub-pattern to a
/// remote endpoint. The result is a syntactically complete, re-parseable query.
///
/// # Examples
///
/// ```
/// use purrdf_sparql_algebra::{GraphPattern, Query, SparqlParser};
/// use purrdf_sparql_algebra::pattern_to_select_query;
///
/// let parser = SparqlParser::new();
/// let Query::Select { pattern, .. } = parser
///     .parse_query("SELECT * WHERE { ?s <http://example.org/p> ?o }")
///     .expect("a well-formed query parses")
/// else {
///     panic!("a SELECT query parses to `Query::Select`");
/// };
/// let GraphPattern::Project { inner, .. } = pattern else {
///     panic!("the projection wraps the root pattern");
/// };
///
/// let rendered = pattern_to_select_query(&inner);
/// assert!(rendered.starts_with("SELECT * WHERE {"));
/// // The rendering is complete and re-parseable.
/// assert!(parser.parse_query(&rendered).is_ok());
/// ```
#[must_use]
pub fn pattern_to_select_query(inner: &GraphPattern) -> String {
    let mut s = String::new();
    s.push_str("SELECT * WHERE { ");
    fmt_group_body(&mut s, inner);
    s.push_str(" }");
    s
}

/// `true` for the solution-modifier nodes that re-materialize as a sub-`SELECT`
/// rather than as a bare group-graph-pattern element.
fn is_subselect_node(p: &GraphPattern) -> bool {
    matches!(
        p,
        GraphPattern::Project { .. }
            | GraphPattern::Distinct { .. }
            | GraphPattern::Reduced { .. }
            | GraphPattern::Slice { .. }
            | GraphPattern::OrderBy { .. }
            | GraphPattern::Group { .. }
    )
}

/// Emit a graph pattern as the body of a `{ … }` group. Modifier-wrapped patterns
/// (subqueries) are emitted as a braced `{ SELECT … }` block.
fn fmt_group_body(s: &mut String, p: &GraphPattern) {
    if is_subselect_node(p) {
        s.push_str("{ ");
        fmt_subselect(s, p);
        s.push_str(" }");
        return;
    }
    match p {
        GraphPattern::Bgp { patterns } => fmt_bgp(s, patterns),
        GraphPattern::Path {
            subject,
            path,
            object,
        } => {
            fmt_term(s, subject);
            let _ = write!(s, " {path} ");
            fmt_term(s, object);
            s.push_str(" .");
        }
        GraphPattern::Join { left, right } => {
            fmt_group_body(s, left);
            s.push(' ');
            fmt_group_body(s, right);
        }
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => {
            fmt_group_body(s, left);
            s.push_str(" OPTIONAL { ");
            fmt_group_body(s, right);
            if let Some(expr) = expression {
                s.push_str(" FILTER(");
                fmt_expr(s, expr);
                s.push(')');
            }
            s.push_str(" }");
        }
        GraphPattern::Lateral { left, right } => {
            fmt_group_body(s, left);
            s.push_str(" LATERAL { ");
            fmt_group_body(s, right);
            s.push_str(" }");
        }
        GraphPattern::Filter { expr, inner } => {
            fmt_group_body(s, inner);
            s.push_str(" FILTER(");
            fmt_expr(s, expr);
            s.push(')');
        }
        GraphPattern::Union { left, right } => {
            s.push_str("{ ");
            fmt_group_body(s, left);
            s.push_str(" } UNION { ");
            fmt_group_body(s, right);
            s.push_str(" }");
        }
        GraphPattern::Graph { name, inner } => {
            s.push_str("GRAPH ");
            fmt_named_node_pattern(s, name);
            s.push_str(" { ");
            fmt_group_body(s, inner);
            s.push_str(" }");
        }
        GraphPattern::Extend {
            inner,
            variable,
            expression,
        } => {
            fmt_group_body(s, inner);
            s.push_str(" BIND(");
            fmt_expr(s, expression);
            let _ = write!(s, " AS {})", VarRef(variable));
        }
        GraphPattern::Minus { left, right } => {
            fmt_group_body(s, left);
            s.push_str(" MINUS { ");
            fmt_group_body(s, right);
            s.push_str(" }");
        }
        GraphPattern::Service {
            name,
            inner,
            silent,
        } => {
            s.push_str("SERVICE ");
            if *silent {
                s.push_str("SILENT ");
            }
            fmt_named_node_pattern(s, name);
            s.push_str(" { ");
            fmt_group_body(s, inner);
            s.push_str(" }");
        }
        GraphPattern::Values {
            variables,
            bindings,
        } => fmt_values(s, variables, bindings),
        // Subselect nodes are handled by the `is_subselect_node` branch above.
        GraphPattern::Project { .. }
        | GraphPattern::Distinct { .. }
        | GraphPattern::Reduced { .. }
        | GraphPattern::Slice { .. }
        | GraphPattern::OrderBy { .. }
        | GraphPattern::Group { .. } => unreachable!("handled by is_subselect_node"),
    }
}

/// Emit a basic graph pattern (a conjunction of triple patterns).
fn fmt_bgp(s: &mut String, patterns: &[TriplePattern]) {
    for (i, tp) in patterns.iter().enumerate() {
        if i > 0 {
            s.push(' ');
        }
        fmt_term(s, &tp.subject);
        s.push(' ');
        fmt_named_node_pattern(s, &tp.predicate);
        s.push(' ');
        fmt_term(s, &tp.object);
        s.push_str(" .");
    }
}

/// Peel the solution-modifier chain (outermost → innermost) and emit a
/// `SELECT [DISTINCT|REDUCED] <vars|*> WHERE { <body> } [GROUP BY] [HAVING]
/// [ORDER BY] [LIMIT] [OFFSET]`.
fn fmt_subselect(s: &mut String, p: &GraphPattern) {
    // Peel outer modifiers, recording each, until we reach the WHERE body.
    let mut cur = p;
    let mut distinct = false;
    let mut reduced = false;
    let mut slice: Option<(usize, Option<usize>)> = None;
    let mut project: Option<&[Variable]> = None;
    let mut order: Option<&[OrderExpression]> = None;
    // SELECT-expression binds (Extend nodes sitting above the Group/body).
    let mut select_exprs: Vec<(&Variable, &Expression)> = Vec::new();
    let mut group: Option<GroupSpec<'_>> = None;
    let mut having: Vec<&Expression> = Vec::new();

    loop {
        match cur {
            GraphPattern::Slice {
                inner,
                start,
                length,
            } => {
                slice = Some((*start, *length));
                cur = inner;
            }
            GraphPattern::Distinct { inner } => {
                distinct = true;
                cur = inner;
            }
            GraphPattern::Reduced { inner } => {
                reduced = true;
                cur = inner;
            }
            GraphPattern::Project { inner, variables } => {
                project = Some(variables);
                cur = inner;
            }
            GraphPattern::OrderBy { inner, expression } => {
                order = Some(expression);
                cur = inner;
            }
            GraphPattern::Extend {
                inner,
                variable,
                expression,
            } => {
                // A SELECT-expression bind only when it sits in the modifier
                // chain above the WHERE body (i.e. above a Group, or directly
                // above the body with no remaining group/where structure). We
                // greedily treat Extends encountered during the peel as SELECT
                // expressions; a BIND inside the WHERE body is reached only after
                // we stop peeling (it stays part of the body).
                select_exprs.push((variable, expression));
                cur = inner;
            }
            GraphPattern::Filter { expr, inner }
                if matches!(**inner, GraphPattern::Group { .. }) =>
            {
                // HAVING: a Filter directly wrapping the Group.
                having.push(expr);
                cur = inner;
            }
            GraphPattern::Group {
                inner,
                variables,
                aggregates,
            } => {
                group = Some((variables, aggregates));
                cur = inner;
                break;
            }
            _ => break,
        }
    }
    // `select_exprs` was collected outermost-first; restore source order.
    select_exprs.reverse();
    having.reverse();

    s.push_str("SELECT ");
    if distinct {
        s.push_str("DISTINCT ");
    } else if reduced {
        s.push_str("REDUCED ");
    }
    match project {
        None => {
            if select_exprs.is_empty() {
                s.push('*');
            }
        }
        Some(vars) if vars.is_empty() && select_exprs.is_empty() => s.push('*'),
        Some(vars) => {
            // Skip any var whose binding will be emitted via `(expr AS ?v)`;
            // emitting it here too would produce an invalid duplicate projection.
            let as_targets: std::collections::HashSet<&Variable> =
                select_exprs.iter().map(|(v, _)| *v).collect();
            let mut plain_emitted = false;
            for v in vars {
                if as_targets.contains(v) {
                    continue;
                }
                if plain_emitted {
                    s.push(' ');
                }
                let _ = write!(s, "{}", VarRef(v));
                plain_emitted = true;
            }
        }
    }
    // Determine whether any plain var was emitted (for spacing before AS-exprs).
    let plain_emitted = match project {
        None => false,
        Some(vars) if vars.is_empty() && select_exprs.is_empty() => false,
        Some(vars) => {
            let as_targets: std::collections::HashSet<&Variable> =
                select_exprs.iter().map(|(v, _)| *v).collect();
            vars.iter().any(|v| !as_targets.contains(v))
        }
    };
    for (i, (var, expr)) in select_exprs.iter().enumerate() {
        if plain_emitted || i > 0 {
            s.push(' ');
        }
        s.push('(');
        fmt_expr(s, expr);
        let _ = write!(s, " AS {})", VarRef(var));
    }

    s.push_str(" WHERE { ");
    fmt_group_body(s, cur);
    s.push_str(" }");

    if let Some((vars, aggs)) = group {
        if !vars.is_empty() {
            s.push_str(" GROUP BY");
            for v in vars {
                let _ = write!(s, " {}", VarRef(v));
            }
        } else if !aggs.is_empty() {
            // Implicit single group (aggregates with no GROUP BY): no clause.
        }
    }
    if !having.is_empty() {
        s.push_str(" HAVING");
        for expr in &having {
            s.push('(');
            fmt_expr(s, expr);
            s.push(')');
        }
    }
    if let Some(exprs) = order {
        s.push_str(" ORDER BY");
        for oe in exprs {
            match oe {
                OrderExpression::Asc(e) => {
                    s.push_str(" ASC(");
                    fmt_expr(s, e);
                    s.push(')');
                }
                OrderExpression::Desc(e) => {
                    s.push_str(" DESC(");
                    fmt_expr(s, e);
                    s.push(')');
                }
            }
        }
    }
    if let Some((start, length)) = slice {
        if let Some(len) = length {
            let _ = write!(s, " LIMIT {len}");
        }
        if start > 0 {
            let _ = write!(s, " OFFSET {start}");
        }
    }
}

/// Emit a `VALUES (?v …) { (term …) … }` block (always the parenthesized form).
fn fmt_values(s: &mut String, variables: &[Variable], bindings: &[Vec<Option<GroundTerm>>]) {
    s.push_str("VALUES (");
    for (i, v) in variables.iter().enumerate() {
        if i > 0 {
            s.push(' ');
        }
        let _ = write!(s, "{}", VarRef(v));
    }
    s.push_str(") {");
    for row in bindings {
        s.push_str(" (");
        for (i, cell) in row.iter().enumerate() {
            if i > 0 {
                s.push(' ');
            }
            match cell {
                None => s.push_str("UNDEF"),
                Some(gt) => fmt_ground_term(s, gt),
            }
        }
        s.push(')');
    }
    s.push_str(" }");
}

/// Emit a query-pattern term.
fn fmt_term(s: &mut String, t: &TermPattern) {
    match t {
        TermPattern::NamedNode(n) => {
            let _ = write!(s, "<{}>", n.as_str());
        }
        TermPattern::BlankNode(b) => {
            let _ = write!(s, "_:{}", b.as_str());
        }
        TermPattern::Literal(l) => fmt_literal(s, l),
        TermPattern::Variable(v) => {
            let _ = write!(s, "{}", VarRef(v));
        }
        TermPattern::Triple(t) => fmt_triple_pattern(s, t),
    }
}

/// Emit an RDF 1.2 quoted triple term `<<( s p o )>>`.
fn fmt_triple_pattern(s: &mut String, t: &TriplePattern) {
    s.push_str("<<( ");
    fmt_term(s, &t.subject);
    s.push(' ');
    fmt_named_node_pattern(s, &t.predicate);
    s.push(' ');
    fmt_term(s, &t.object);
    s.push_str(" )>>");
}

/// Emit an IRI-or-variable (predicate / `GRAPH`/`SERVICE` name position).
fn fmt_named_node_pattern(s: &mut String, n: &NamedNodePattern) {
    match n {
        NamedNodePattern::NamedNode(node) => {
            let _ = write!(s, "<{}>", node.as_str());
        }
        NamedNodePattern::Variable(v) => {
            let _ = write!(s, "{}", VarRef(v));
        }
    }
}

/// Emit a ground term (VALUES cell).
fn fmt_ground_term(s: &mut String, gt: &GroundTerm) {
    match gt {
        GroundTerm::NamedNode(n) => {
            let _ = write!(s, "<{}>", n.as_str());
        }
        GroundTerm::Literal(l) => fmt_literal(s, l),
        GroundTerm::Triple(t) => fmt_ground_triple(s, t),
        // Injection-only (GAP-A): emitted as a blank-node label. This appears
        // only if a substituted query is re-serialized (e.g. forwarded to SERVICE);
        // the parser never produces it, so a round-trip of an un-substituted query is
        // unaffected.
        GroundTerm::BlankNode(b) => {
            let _ = write!(s, "_:{}", b.as_str());
        }
    }
}

/// Emit a ground RDF 1.2 quoted triple term.
fn fmt_ground_triple(s: &mut String, t: &GroundTriple) {
    s.push_str("<<( ");
    fmt_ground_term(s, &t.subject);
    let _ = write!(s, " <{}> ", t.predicate.as_str());
    fmt_ground_term(s, &t.object);
    s.push_str(" )>>");
}

/// Emit a literal, escaping the lexical form to mirror the lexer's string rules.
fn fmt_literal(s: &mut String, l: &Literal) {
    s.push('"');
    push_escaped(s, l.value());
    s.push('"');
    match (l.language(), l.direction()) {
        (Some(lang), Some(dir)) => {
            let d = match dir {
                BaseDirection::Ltr => "ltr",
                BaseDirection::Rtl => "rtl",
            };
            let _ = write!(s, "@{lang}--{d}");
        }
        (Some(lang), None) => {
            let _ = write!(s, "@{lang}");
        }
        (None, _) => {
            let dt = l.datatype().as_str();
            // `xsd:string` and `rdf:langString` are implied; everything else is
            // explicit `^^<datatype>`.
            if dt != XSD_STRING && dt != RDF_LANG_STRING {
                let _ = write!(s, "^^<{dt}>");
            }
        }
    }
}

/// Escape a string literal's lexical content for a short `"…"` form, mirroring
/// the lexer's `lex_string` escape table (`\`, `"`, `\n`, `\r`, `\t`).
fn push_escaped(s: &mut String, value: &str) {
    for c in value.chars() {
        match c {
            '\\' => s.push_str("\\\\"),
            '"' => s.push_str("\\\""),
            '\n' => s.push_str("\\n"),
            '\r' => s.push_str("\\r"),
            '\t' => s.push_str("\\t"),
            '\u{0008}' => s.push_str("\\b"),
            '\u{000C}' => s.push_str("\\f"),
            other => s.push(other),
        }
    }
}

/// A `Display` shim that renders a [`Variable`] with its `?` sigil.
struct VarRef<'a>(&'a Variable);

impl core::fmt::Display for VarRef<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "?{}", self.0.as_str())
    }
}

/// Emit an expression. Binary and unary operators are conservatively
/// parenthesized so re-parse never depends on reproducing exact precedence.
fn fmt_expr(s: &mut String, e: &Expression) {
    match e {
        Expression::NamedNode(n) => {
            let _ = write!(s, "<{}>", n.as_str());
        }
        Expression::Literal(l) => fmt_literal(s, l),
        Expression::Variable(v) => {
            let _ = write!(s, "{}", VarRef(v));
        }
        Expression::Bound(v) => {
            let _ = write!(s, "BOUND({})", VarRef(v));
        }
        Expression::Or(a, b) => fmt_binop(s, a, "||", b),
        Expression::And(a, b) => fmt_binop(s, a, "&&", b),
        Expression::Equal(a, b) => fmt_binop(s, a, "=", b),
        Expression::SameTerm(a, b) => {
            s.push_str("sameTerm(");
            fmt_expr(s, a);
            s.push_str(", ");
            fmt_expr(s, b);
            s.push(')');
        }
        Expression::Greater(a, b) => fmt_binop(s, a, ">", b),
        Expression::GreaterOrEqual(a, b) => fmt_binop(s, a, ">=", b),
        Expression::Less(a, b) => fmt_binop(s, a, "<", b),
        Expression::LessOrEqual(a, b) => fmt_binop(s, a, "<=", b),
        Expression::Add(a, b) => fmt_binop(s, a, "+", b),
        Expression::Subtract(a, b) => fmt_binop(s, a, "-", b),
        Expression::Multiply(a, b) => fmt_binop(s, a, "*", b),
        Expression::Divide(a, b) => fmt_binop(s, a, "/", b),
        Expression::UnaryPlus(a) => {
            s.push_str("(+");
            fmt_expr(s, a);
            s.push(')');
        }
        Expression::UnaryMinus(a) => {
            s.push_str("(-");
            fmt_expr(s, a);
            s.push(')');
        }
        Expression::Not(a) => {
            s.push_str("(!");
            fmt_expr(s, a);
            s.push(')');
        }
        Expression::In(a, list) => {
            s.push('(');
            fmt_expr(s, a);
            s.push_str(" IN (");
            fmt_expr_list(s, list);
            s.push_str("))");
        }
        Expression::If(c, t, e2) => {
            s.push_str("IF(");
            fmt_expr(s, c);
            s.push_str(", ");
            fmt_expr(s, t);
            s.push_str(", ");
            fmt_expr(s, e2);
            s.push(')');
        }
        Expression::Coalesce(list) => {
            s.push_str("COALESCE(");
            fmt_expr_list(s, list);
            s.push(')');
        }
        Expression::FunctionCall(func, args) => {
            fmt_function_name(s, func);
            s.push('(');
            fmt_expr_list(s, args);
            s.push(')');
        }
        Expression::Exists(p) => {
            s.push_str("EXISTS { ");
            fmt_group_body(s, p);
            s.push_str(" }");
        }
    }
}

/// Emit `(a OP b)` with conservative parentheses.
fn fmt_binop(s: &mut String, a: &Expression, op: &str, b: &Expression) {
    s.push('(');
    fmt_expr(s, a);
    let _ = write!(s, " {op} ");
    fmt_expr(s, b);
    s.push(')');
}

/// Emit a comma-separated expression list.
fn fmt_expr_list(s: &mut String, list: &[Expression]) {
    for (i, e) in list.iter().enumerate() {
        if i > 0 {
            s.push_str(", ");
        }
        fmt_expr(s, e);
    }
}

/// Emit a SPARQL built-in or custom function name.
fn fmt_function_name(s: &mut String, f: &Function) {
    let name = match f {
        Function::Str => "STR",
        Function::Lang => "LANG",
        Function::LangMatches => "LANGMATCHES",
        Function::Datatype => "DATATYPE",
        Function::Iri => "IRI",
        Function::Uri => "URI",
        Function::BNode => "BNODE",
        Function::Rand => "RAND",
        Function::Abs => "ABS",
        Function::Ceil => "CEIL",
        Function::Floor => "FLOOR",
        Function::Round => "ROUND",
        Function::Concat => "CONCAT",
        Function::SubStr => "SUBSTR",
        Function::StrLen => "STRLEN",
        Function::Replace => "REPLACE",
        Function::UCase => "UCASE",
        Function::LCase => "LCASE",
        Function::EncodeForUri => "ENCODE_FOR_URI",
        Function::Contains => "CONTAINS",
        Function::StrStarts => "STRSTARTS",
        Function::StrEnds => "STRENDS",
        Function::StrBefore => "STRBEFORE",
        Function::StrAfter => "STRAFTER",
        Function::Year => "YEAR",
        Function::Month => "MONTH",
        Function::Day => "DAY",
        Function::Hours => "HOURS",
        Function::Minutes => "MINUTES",
        Function::Seconds => "SECONDS",
        Function::Timezone => "TIMEZONE",
        Function::Tz => "TZ",
        Function::Now => "NOW",
        Function::Uuid => "UUID",
        Function::StrUuid => "STRUUID",
        Function::Md5 => "MD5",
        Function::Sha1 => "SHA1",
        Function::Sha256 => "SHA256",
        Function::Sha384 => "SHA384",
        Function::Sha512 => "SHA512",
        Function::StrLang => "STRLANG",
        Function::StrDt => "STRDT",
        Function::IsIri => "isIRI",
        Function::IsUri => "isURI",
        Function::IsBlank => "isBLANK",
        Function::IsLiteral => "isLITERAL",
        Function::IsNumeric => "isNUMERIC",
        Function::Regex => "REGEX",
        Function::Triple => "TRIPLE",
        Function::Subject => "SUBJECT",
        Function::Predicate => "PREDICATE",
        Function::Object => "OBJECT",
        Function::IsTriple => "isTRIPLE",
        Function::LangDir => "LANGDIR",
        Function::StrLangDir => "STRLANGDIR",
        Function::HasLang => "hasLANG",
        Function::HasLangDir => "hasLANGDIR",
        Function::Purrdf(call) => {
            // Emit the ORIGINAL IRI the call was parsed from (recorded in the AST
            // node). PurRDF mints no vocabulary of its own, so no namespace is ever
            // fabricated on output; re-parsing with the same ParserOptions
            // re-dispatches to the same PurrdfFn.
            let _ = write!(s, "<{}>", call.iri);
            return;
        }
        Function::Custom(n) => {
            let _ = write!(s, "<{}>", n.as_str());
            return;
        }
    };
    s.push_str(name);
}

/// Emit a SPARQL aggregate expression `FUNC([DISTINCT] expr [; SEPARATOR="…"])`.
///
/// Used by [`fmt_subselect`] only when an aggregate appears explicitly; the
/// hoisted synthetic-variable form is handled structurally. Exposed for tests.
#[cfg(test)]
fn fmt_aggregate(s: &mut String, agg: &AggregateExpression) {
    match agg {
        AggregateExpression::CountStar { distinct } => {
            s.push_str("COUNT(");
            if *distinct {
                s.push_str("DISTINCT ");
            }
            s.push_str("*)");
        }
        AggregateExpression::FunctionCall {
            function,
            expression,
            distinct,
        } => {
            let name = match function {
                AggregateFunction::Count => "COUNT",
                AggregateFunction::Sum => "SUM",
                AggregateFunction::Avg => "AVG",
                AggregateFunction::Min => "MIN",
                AggregateFunction::Max => "MAX",
                AggregateFunction::Sample => "SAMPLE",
                AggregateFunction::GroupConcat { .. } => "GROUP_CONCAT",
                AggregateFunction::Custom(n) => {
                    let _ = write!(s, "<{}>(", n.as_str());
                    if *distinct {
                        s.push_str("DISTINCT ");
                    }
                    fmt_expr(s, expression);
                    s.push(')');
                    return;
                }
            };
            s.push_str(name);
            s.push('(');
            if *distinct {
                s.push_str("DISTINCT ");
            }
            fmt_expr(s, expression);
            if let AggregateFunction::GroupConcat {
                separator: Some(sep),
            } = function
            {
                s.push_str("; SEPARATOR=\"");
                push_escaped(s, sep);
                s.push('"');
            }
            s.push(')');
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Query;
    use crate::parser::SparqlParser;

    /// Parse a full query and return its root pattern.
    fn pattern_of(query: &str) -> GraphPattern {
        let gm = "PREFIX purrdf: <https://example.org/ext/>\n".to_owned();
        let gm = gm.as_str();
        match SparqlParser::new()
            .parse_query(&format!("{gm}{query}"))
            .unwrap_or_else(|e| panic!("parse `{query}`: {e:?}"))
        {
            Query::Select { pattern, .. } => pattern,
            other => panic!("expected SELECT, got {other:?}"),
        }
    }

    /// Strip exactly one outer `Project` (the `SELECT …` scaffold) to recover the
    /// WHERE body — the shape a SERVICE node forwards and
    /// `pattern_to_select_query` consumes. The parser expands `SELECT *` to an
    /// explicit variable list, so the strip is unconditional.
    fn where_body(p: &GraphPattern) -> GraphPattern {
        match p {
            GraphPattern::Project { inner, .. } => (**inner).clone(),
            other => other.clone(),
        }
    }

    /// Assert that serializing the WHERE body then re-parsing reproduces the same
    /// algebra (round-trip stability) — exactly the SERVICE forward path.
    fn assert_roundtrip(query: &str) {
        let body = where_body(&pattern_of(query));
        let text = pattern_to_select_query(&body);
        let reparsed = match SparqlParser::new()
            .parse_query(&text)
            .unwrap_or_else(|e| panic!("re-parse `{text}`: {e:?}"))
        {
            Query::Select { pattern, .. } => pattern,
            other => panic!("expected SELECT, got {other:?}"),
        };
        let reparsed_body = where_body(&reparsed);
        assert_eq!(
            reparsed_body, body,
            "round-trip mismatch for `{query}`\n serialized: {text}"
        );
    }

    #[test]
    fn roundtrip_bgp() {
        assert_roundtrip("SELECT * WHERE { ?s <http://ex/p> ?o }");
    }

    #[test]
    fn roundtrip_multi_triple_bgp() {
        assert_roundtrip("SELECT * WHERE { ?s <http://ex/p> ?o . ?o <http://ex/q> ?z }");
    }

    #[test]
    fn roundtrip_optional() {
        assert_roundtrip("SELECT * WHERE { ?s <http://ex/p> ?o OPTIONAL { ?o <http://ex/q> ?z } }");
    }

    #[test]
    fn roundtrip_union() {
        assert_roundtrip(
            "SELECT * WHERE { { ?s <http://ex/p> ?o } UNION { ?s <http://ex/q> ?o } }",
        );
    }

    #[test]
    fn roundtrip_filter_and_bind() {
        assert_roundtrip(
            "SELECT * WHERE { ?s <http://ex/age> ?a FILTER(?a > 18) BIND((?a + 1) AS ?b) }",
        );
    }

    #[test]
    fn roundtrip_minus() {
        assert_roundtrip("SELECT * WHERE { ?s <http://ex/p> ?o MINUS { ?s <http://ex/q> ?o } }");
    }

    #[test]
    fn roundtrip_graph() {
        assert_roundtrip("SELECT * WHERE { GRAPH ?g { ?s <http://ex/p> ?o } }");
    }

    #[test]
    fn roundtrip_path() {
        assert_roundtrip("SELECT * WHERE { ?s <http://ex/p>+ ?o }");
    }

    #[test]
    fn roundtrip_values() {
        assert_roundtrip(
            "SELECT * WHERE { ?s <http://ex/p> ?o VALUES (?o) { (<http://ex/a>) (UNDEF) } }",
        );
    }

    #[test]
    fn roundtrip_typed_literal() {
        assert_roundtrip(
            "SELECT * WHERE { ?s <http://ex/p> \"42\"^^<http://www.w3.org/2001/XMLSchema#integer> }",
        );
    }

    #[test]
    fn roundtrip_lang_literal() {
        assert_roundtrip("SELECT * WHERE { ?s <http://ex/p> \"hi\"@en }");
    }

    #[test]
    fn roundtrip_quoted_triple() {
        assert_roundtrip(
            "SELECT ?r WHERE { ?r <http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies> <<( ?s ?p ?o )>> }",
        );
    }

    #[test]
    fn roundtrip_exists() {
        assert_roundtrip(
            "SELECT * WHERE { ?s <http://ex/p> ?o FILTER EXISTS { ?o <http://ex/q> ?z } }",
        );
    }

    #[test]
    fn roundtrip_nested_service() {
        assert_roundtrip("SELECT * WHERE { SERVICE <http://ep/sparql> { ?s <http://ex/p> ?o } }");
    }

    #[test]
    fn roundtrip_service_silent() {
        assert_roundtrip(
            "SELECT * WHERE { SERVICE SILENT <http://ep/sparql> { ?s <http://ex/p> ?o } }",
        );
    }

    #[test]
    fn roundtrip_subselect_distinct_limit() {
        assert_roundtrip(
            "SELECT * WHERE { { SELECT DISTINCT ?s WHERE { ?s <http://ex/p> ?o } ORDER BY ?s LIMIT 5 OFFSET 2 } }",
        );
    }

    #[test]
    fn produces_complete_select() {
        let p = pattern_of("SELECT * WHERE { ?s <http://ex/p> ?o }");
        let text = pattern_to_select_query(&p);
        assert!(text.starts_with("SELECT * WHERE {"), "got: {text}");
        assert!(text.contains("<http://ex/p>"), "got: {text}");
    }

    /// A subselect that mixes a plain projected variable and a SELECT expression
    /// (`(expr AS ?v)`) must not duplicate the AS-target var in the projection
    /// list. Before the fix, parsing `SELECT ?s (?o + 1 AS ?x) WHERE { … }`
    /// pushed `?x` into both `projected` and `select_exprs`, so the serializer
    /// emitted `SELECT ?s ?x (?o + 1 AS ?x)` — invalid SPARQL 1.1 (double projection).
    #[test]
    fn subselect_select_expr_no_duplicate_projection() {
        // Build a subselect that has a plain var (?s) and an AS-expression (?x).
        // The subselect is embedded so `fmt_subselect` is exercised.
        let query = "SELECT * WHERE { { SELECT ?s (?o + 1 AS ?x) WHERE { ?s <http://ex/p> ?o } } }";
        let body = where_body(&pattern_of(query));
        let text = pattern_to_select_query(&body);

        // The AS-target ?x must appear exactly once, only inside `(… AS ?x)`.
        let count_bare_x = text.split_whitespace().filter(|tok| *tok == "?x").count();
        assert_eq!(
            count_bare_x, 0,
            "?x must not appear as a bare projected var; got: {text}"
        );
        assert!(
            text.contains("AS ?x)"),
            "?x must still appear in AS-expression form; got: {text}"
        );
        // The plain projected var ?s must still appear.
        assert!(
            text.split_whitespace().any(|t| t == "?s"),
            "?s must appear as a plain projected var; got: {text}"
        );
        // Round-trip: the serialized text must parse without error.
        SparqlParser::new()
            .parse_query(&text)
            .unwrap_or_else(|e| panic!("re-parse of `{text}` failed: {e:?}"));
    }

    #[test]
    fn aggregate_renders_group_concat_separator() {
        let agg = AggregateExpression::FunctionCall {
            function: AggregateFunction::GroupConcat {
                separator: Some("|".to_owned()),
            },
            expression: Box::new(Expression::Variable(Variable::new("x"))),
            distinct: false,
        };
        let mut s = String::new();
        fmt_aggregate(&mut s, &agg);
        assert_eq!(s, "GROUP_CONCAT(?x; SEPARATOR=\"|\")");
    }
}

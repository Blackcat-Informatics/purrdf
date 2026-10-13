// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The whole-serializer round-trip guard `serialize.rs`'s own module doc
//! claims but no test before this one measured across a real corpus:
//! `crates/sparql-algebra/src/serialize.rs:17-19` says
//! `parse(pattern_to_select_query(p))` reproduces `p` "for every
//! `GraphPattern`/`Expression` variant the parser emits" — a claim a handful
//! of hand-authored `roundtrip_*` unit tests in `serialize.rs` itself can
//! gesture at but never PROVE — and the discovery of the
//! `fmt_join_right_operand` re-association class and the variable-endpoint
//! `SERVICE` double-wrap showed it FALSE. This sweep is the proof, run over
//! every real SPARQL query text this repository ships or vendors.
//!
//! # Corpus
//!
//! * The vendored W3C SPARQL 1.1 and 1.2 test suites
//!   (`crates/sparql-conformance/suite/w3c-sparql11`,
//!   `.../w3c-sparql12`) — every `.rq` file.
//! * The first-party `purrdf-extend` suite
//!   (`crates/sparql-conformance/suite/purrdf-extend`) — every `.rq` file.
//! * The user-guide's own worked examples — every fenced ` ```sparql ` block
//!   under `docs/**/*.md` (the same "documentation is a shipped surface"
//!   principle `shipped_sparql_examples.rs` established, narrowed here to the
//!   ONE extraction pass — whole fenced blocks — that yields genuinely
//!   complete, standalone queries worth round-tripping; that other test
//!   already proves every embedded fragment PARSES, which this sweep does
//!   not re-litigate). A fenced block may be a QUERY or an UPDATE request —
//!   the guide teaches both — so each one is routed by what it actually
//!   parses as (see [`collect_doc_examples`]'s call site): query text takes
//!   the `.rq` lane below, update text takes the `.ru` lane's
//!   `parse_update`/[`roundtrip_update`]/[`RU_XFAIL`] treatment. Routing it
//!   rather than parsing every block as a query is deliberate: an `INSERT`
//!   example handed only to `parse_query` fails to parse and would be tallied
//!   into [`MAX_UNPARSEABLE_RQ`] — a ceiling that counts genuinely
//!   NEGATIVE-SYNTAX fixtures — mislabelling a perfectly valid shipped
//!   example as a syntax error and permanently blinding the ceiling by one.
//!
//! **`.ru` (UPDATE) files are swept too.** `Display for GraphUpdateOperation`/
//! `Display for Update` (`crates/sparql-algebra/src/algebra.rs`) IS an
//! UPDATE-request serializer — it renders a parsed [`Update`] back to SPARQL
//! Update surface syntax, reusing [`crate::serialize::fmt_group_body`] (the
//! same WHERE-body renderer [`pattern_to_select_query`] uses) for each
//! operation's `WHERE` clause. Every `.ru` file under the same corpus roots is
//! parsed with [`SparqlParser::parse_update`], `Display`ed, re-parsed with
//! [`SparqlParser::parse_update`], and compared for equality — the same
//! shape as the `.rq` sweep below, with its own unparseable/`XFAIL` counters
//! ([`RU_XFAIL`]) kept separate from the query-side ones so the two corpora's
//! accounting never gets confused.
//!
//! # Method
//!
//! The sweep serializes a query's WHERE body as a complete SELECT carrier.
//! An outer source projection is removed once; a body that itself begins with
//! Project keeps that projection in the carrier. Both parsed bodies are compared
//! as complete typed algebra, including constants, expression roles and outputs.
//!
//! Join spines are left-associated before comparison. Non-distinguished match
//! identities receive legal carrier names through a bijective alpha map derived
//! from corresponding typed variable positions. Ordinary names remain exact, and
//! aliases that collide, merge identities or split one identity are refused.
//! The carrier's visible projection must match the source scope in exact order.
//! Neither corpus ceilings nor xfail accounting permit another disagreement.
//!
//! # Corpus items that do not even parse
//!
//! A `.rq`/doc-example text that fails to PARSE at all (a W3C
//! `NegativeSyntaxTest`/`NegativeUpdateSyntaxTest` fixture, or a construct
//! this parser does not accept) is outside this sweep's contract — there is
//! no pattern to hand the serializer. Parser grammar coverage is
//! `sparql_conformance`'s job (its own pass/xfail/unmodeled ledger against
//! the official manifest test types); this sweep's `XFAIL` ledger is
//! reserved for the DIFFERENT, narrower claim named below. Such items are
//! counted (`unparseable`, printed) but never asserted on.
//!
//! # The `XFAIL` ledger
//!
//! An item that PARSES but whose round-trip disagrees is either (a) a real
//! serializer defect — fixed in `serialize.rs` in the same change that added
//! this sweep, never ledgered — or (b) a genuine "emit-only" gap: an algebra
//! shape the parser can produce but the surface grammar has no construct to
//! re-emit it through (the PurRDF predicate-wildcard path extension
//! (`<any>`), documented emit-only on
//! [`purrdf_sparql_algebra::PropertyPathExpression`]'s `Display`, is the
//! one shape in this crate with that property). `XFAIL` entries name the
//! corpus-relative path and the construct; `assert_eq!(XFAIL.len(), K)`
//! keeps the ledger's size a visible diff, and an entry that round-trips
//! cleanly (the gap got fixed and nobody removed the ledger row) fails the
//! sweep rather than sitting stale.

#[path = "support/patterns.rs"]
mod patterns;

use patterns::where_body;
use purrdf_sparql_algebra::walk::{Flow, NodeRef, Visit, walk_pre_post};
use purrdf_sparql_algebra::{
    Child, Expression, NamedNodePattern, OrderExpression, TermPattern, TriplePattern, Variable,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use purrdf_sparql_algebra::{
    GraphPattern, GraphUpdateOperation, Query, SparqlParser, Update, pattern_to_select_query,
};
use purrdf_testkit::paths::workspace_root;

/// Every file with extension `ext` under `dir`, recursively. Shared by the
/// `.rq` (query) and `.ru` (update) collection passes below — every other
/// extension is ignored. Unsorted: sorting a growing `out` at every
/// recursion level is a redundant `O(depth)` re-sort of the same prefix, so
/// the one sort that actually matters is the caller's, once, after the
/// whole tree (or set of trees) has been walked — see [`collect_rq`]/
/// [`collect_ru`].
fn collect_by_ext(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_by_ext(&path, ext, out);
        } else if path.extension().is_some_and(|x| x == ext) {
            out.push(path);
        }
    }
}

/// Every `.rq` file under `dir`, recursively, appended to `out` and left in
/// a stable (sorted) order. Callable more than once against the same `out`
/// (each corpus root in the sweep below does) — the sort still runs exactly
/// once per call, over the whole accumulated vector, which is enough to
/// leave it fully sorted.
fn collect_rq(dir: &Path, out: &mut Vec<PathBuf>) {
    collect_by_ext(dir, "rq", out);
    out.sort();
}

/// Every `.ru` (UPDATE request) file under `dir`, recursively, appended to
/// `out` and left in a stable (sorted) order. See [`collect_rq`]'s doc for
/// the repeated-call/sort-once discipline this shares.
fn collect_ru(dir: &Path, out: &mut Vec<PathBuf>) {
    collect_by_ext(dir, "ru", out);
    out.sort();
}

/// Every fenced ` ```sparql ` block under `dir`'s Markdown files — the book's
/// own worked, standalone examples (unlike `shipped_sparql_examples.rs`'s
/// other extraction passes, which exist to catch a fragment BROKEN mid-prose
/// and so deliberately look at partial/schematic text too, a fenced whole
/// block is written to be a complete, real query, which is what a
/// round-trip sweep needs).
fn collect_doc_examples(dir: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if !dir.is_dir() {
        return out;
    }
    let mut md_files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for entry in rd.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
                md_files.push(path);
            }
        }
    }
    md_files.sort();
    for file in md_files {
        let text = std::fs::read_to_string(&file).unwrap_or_else(|e| panic!("read {file:?}: {e}"));
        let mut in_fence = false;
        let mut body = String::new();
        let mut block_no = 0usize;
        for line in text.lines() {
            let trimmed = line.trim_start();
            if in_fence {
                if trimmed.starts_with("```") {
                    in_fence = false;
                    block_no += 1;
                    let label = format!(
                        "{}#sparql-block-{block_no}",
                        file.to_string_lossy().replace('\\', "/")
                    );
                    out.push((label, std::mem::take(&mut body)));
                } else {
                    body.push_str(line);
                    body.push('\n');
                }
                continue;
            }
            if trimmed.starts_with("```sparql") {
                in_fence = true;
                body.clear();
            }
        }
    }
    out
}

/// Left-linearize Join spines and apply the checked hidden-only alpha map.
/// Every other
/// [`GraphPattern`] variant is reconstructed with its children normalized
/// the same way and every non-pattern field carried through unchanged; the
/// match is exhaustive (no wildcard arm), so a future algebra variant is a
/// compile error here until this function is taught its shape, not a
/// silently-unnormalized blind spot.
fn normalize_join_assoc(p: &GraphPattern, aliases: &BTreeMap<Variable, Variable>) -> GraphPattern {
    match p {
        GraphPattern::Join { .. } => {
            let mut leaves = Vec::new();
            flatten_join(p, &mut leaves);
            let mut normalized = leaves
                .into_iter()
                .map(|node| normalize_join_assoc(node, aliases));
            let first = normalized
                .next()
                .expect("a Join node flattens to at least two leaves");
            normalized.fold(first, |acc, next| GraphPattern::Join {
                left: Child::new(acc),
                right: Child::new(next),
            })
        }
        GraphPattern::Bgp { patterns } => GraphPattern::Bgp {
            patterns: patterns
                .iter()
                .map(|triple| alpha_triple(triple, aliases))
                .collect(),
        },
        GraphPattern::Path {
            subject,
            path,
            object,
        } => GraphPattern::Path {
            subject: alpha_term(subject, aliases),
            path: path.clone(),
            object: alpha_term(object, aliases),
        },
        GraphPattern::PropertyFunction(call) => {
            let mut call = call.clone();
            for term in call.subject_args.iter_mut().chain(&mut call.object_args) {
                *term = alpha_term(term, aliases);
            }
            GraphPattern::PropertyFunction(call)
        }
        GraphPattern::LeftJoin {
            left,
            right,
            expression,
        } => GraphPattern::LeftJoin {
            left: Child::new(normalize_join_assoc(left, aliases)),
            right: Child::new(normalize_join_assoc(right, aliases)),
            expression: expression
                .as_ref()
                .map(|expr| alpha_expression(expr, aliases)),
        },
        GraphPattern::Lateral { left, right } => GraphPattern::Lateral {
            left: Child::new(normalize_join_assoc(left, aliases)),
            right: Child::new(normalize_join_assoc(right, aliases)),
        },
        GraphPattern::Apply {
            left,
            right,
            policy,
        } => {
            let rename_pairs = |pairs: &[(Variable, Variable)]| {
                pairs
                    .iter()
                    .map(|(input, driver)| {
                        (
                            alpha_variable(input, aliases),
                            alpha_variable(driver, aliases),
                        )
                    })
                    .collect()
            };
            let mut policy = policy.clone();
            policy.group_domain = policy.group_domain.as_ref().map(|domain| {
                domain
                    .iter()
                    .map(|variable| alpha_variable(variable, aliases))
                    .collect()
            });
            policy.inputs = rename_pairs(&policy.inputs);
            if let Some(optional) = &mut policy.optional {
                optional.retry_inputs = rename_pairs(&optional.retry_inputs);
                optional.forget_marker = alpha_variable(&optional.forget_marker, aliases);
            }
            GraphPattern::Apply {
                left: Child::new(normalize_join_assoc(left, aliases)),
                right: Child::new(normalize_join_assoc(right, aliases)),
                policy,
            }
        }
        GraphPattern::Filter { expr, inner } => GraphPattern::Filter {
            expr: alpha_expression(expr, aliases),
            inner: Child::new(normalize_join_assoc(inner, aliases)),
        },
        GraphPattern::Union { arms } => GraphPattern::Union {
            arms: arms.clone().map(|arm| normalize_join_assoc(&arm, aliases)),
        },
        GraphPattern::Graph { name, inner } => GraphPattern::Graph {
            name: alpha_named(name, aliases),
            inner: Child::new(normalize_join_assoc(inner, aliases)),
        },
        GraphPattern::Extend {
            inner,
            variable,
            expression,
        } => GraphPattern::Extend {
            inner: Child::new(normalize_join_assoc(inner, aliases)),
            variable: alpha_variable(variable, aliases),
            expression: alpha_expression(expression, aliases),
        },
        GraphPattern::Unfold {
            inner,
            expression,
            element,
            companion,
        } => GraphPattern::Unfold {
            inner: Child::new(normalize_join_assoc(inner, aliases)),
            expression: alpha_expression(expression, aliases),
            element: alpha_variable(element, aliases),
            companion: companion.as_ref().map(|v| alpha_variable(v, aliases)),
        },
        GraphPattern::Minus { left, right } => GraphPattern::Minus {
            left: Child::new(normalize_join_assoc(left, aliases)),
            right: Child::new(normalize_join_assoc(right, aliases)),
        },
        GraphPattern::Service {
            name,
            inner,
            silent,
        } => GraphPattern::Service {
            name: alpha_named(name, aliases),
            inner: Child::new(normalize_join_assoc(inner, aliases)),
            silent: *silent,
        },
        GraphPattern::Values {
            variables,
            bindings,
        } => GraphPattern::Values {
            variables: variables
                .iter()
                .map(|v| alpha_variable(v, aliases))
                .collect(),
            bindings: bindings.clone(),
        },
        GraphPattern::OrderBy { inner, expression } => GraphPattern::OrderBy {
            inner: Child::new(normalize_join_assoc(inner, aliases)),
            expression: expression
                .iter()
                .map(|order| alpha_order(order, aliases))
                .collect(),
        },
        GraphPattern::Project { inner, variables } => GraphPattern::Project {
            inner: Child::new(normalize_join_assoc(inner, aliases)),
            variables: variables
                .iter()
                .map(|v| alpha_variable(v, aliases))
                .collect(),
        },
        GraphPattern::Distinct { inner } => GraphPattern::Distinct {
            inner: Child::new(normalize_join_assoc(inner, aliases)),
        },
        GraphPattern::Reduced { inner } => GraphPattern::Reduced {
            inner: Child::new(normalize_join_assoc(inner, aliases)),
        },
        GraphPattern::Slice {
            inner,
            start,
            length,
        } => GraphPattern::Slice {
            inner: Child::new(normalize_join_assoc(inner, aliases)),
            start: *start,
            length: *length,
        },
        GraphPattern::Group {
            inner,
            variables,
            aggregates,
        } => GraphPattern::Group {
            inner: Child::new(normalize_join_assoc(inner, aliases)),
            variables: variables
                .iter()
                .map(|v| alpha_variable(v, aliases))
                .collect(),
            aggregates: aggregates
                .iter()
                .map(|(v, aggregate)| {
                    (
                        alpha_variable(v, aliases),
                        purrdf_sparql_algebra::AggregateExpression::new(
                            aggregate.function().clone(),
                            aggregate
                                .args()
                                .iter()
                                .map(|expr| alpha_expression(expr, aliases))
                                .collect(),
                            aggregate.scalarvals().to_vec(),
                            aggregate
                                .order_by()
                                .iter()
                                .map(|order| alpha_order(order, aliases))
                                .collect(),
                            aggregate.distinct,
                        )
                        .expect("renaming match identities retains aggregate structure"),
                    )
                })
                .collect(),
        },
    }
}

fn alpha_variable(variable: &Variable, aliases: &BTreeMap<Variable, Variable>) -> Variable {
    aliases.get(variable).unwrap_or(variable).clone()
}

fn alpha_named(
    name: &NamedNodePattern,
    aliases: &BTreeMap<Variable, Variable>,
) -> NamedNodePattern {
    match name {
        NamedNodePattern::Variable(v) => NamedNodePattern::Variable(alpha_variable(v, aliases)),
        NamedNodePattern::NamedNode(_) => name.clone(),
    }
}

fn alpha_triple(triple: &TriplePattern, aliases: &BTreeMap<Variable, Variable>) -> TriplePattern {
    TriplePattern {
        subject: alpha_term(&triple.subject, aliases),
        predicate: alpha_named(&triple.predicate, aliases),
        object: alpha_term(&triple.object, aliases),
    }
}

fn alpha_term(term: &TermPattern, aliases: &BTreeMap<Variable, Variable>) -> TermPattern {
    match term {
        TermPattern::Variable(v) => TermPattern::Variable(alpha_variable(v, aliases)),
        TermPattern::Triple(triple) => {
            TermPattern::Triple(Child::new(alpha_triple(triple, aliases)))
        }
        TermPattern::NamedNode(_) | TermPattern::BlankNode(_) | TermPattern::Literal(_) => {
            term.clone()
        }
    }
}

fn alpha_order(order: &OrderExpression, aliases: &BTreeMap<Variable, Variable>) -> OrderExpression {
    match order {
        OrderExpression::Asc(expr) => OrderExpression::Asc(alpha_expression(expr, aliases)),
        OrderExpression::Desc(expr) => OrderExpression::Desc(alpha_expression(expr, aliases)),
    }
}

/// A typed oracle, not a Debug-text replacement: every constant, output role and
/// expression is retained, while EXISTS match witnesses receive the same mapping.
fn alpha_expression(value: &Expression, aliases: &BTreeMap<Variable, Variable>) -> Expression {
    let child = |expr: &Expression| Child::new(alpha_expression(expr, aliases));
    match value {
        Expression::NamedNode(_)
        | Expression::Literal(_)
        | Expression::Variable(_)
        | Expression::Bound(_) => value.clone(),
        Expression::Exists(inner) => {
            Expression::Exists(Child::new(normalize_join_assoc(inner, aliases)))
        }
        Expression::Or(operands) => Expression::Or(
            operands
                .clone()
                .map(|expr| alpha_expression(&expr, aliases)),
        ),
        Expression::And(operands) => Expression::And(
            operands
                .clone()
                .map(|expr| alpha_expression(&expr, aliases)),
        ),
        Expression::Equal(a, b) => Expression::Equal(child(a), child(b)),
        Expression::SameTerm(a, b) => Expression::SameTerm(child(a), child(b)),
        Expression::Greater(a, b) => Expression::Greater(child(a), child(b)),
        Expression::GreaterOrEqual(a, b) => Expression::GreaterOrEqual(child(a), child(b)),
        Expression::Less(a, b) => Expression::Less(child(a), child(b)),
        Expression::LessOrEqual(a, b) => Expression::LessOrEqual(child(a), child(b)),
        Expression::Arithmetic(first, steps) => Expression::Arithmetic(
            child(first),
            steps
                .clone()
                .map(|(op, expr)| (op, alpha_expression(&expr, aliases))),
        ),
        Expression::UnaryPlus(expr) => Expression::UnaryPlus(child(expr)),
        Expression::UnaryMinus(expr) => Expression::UnaryMinus(child(expr)),
        Expression::Not(expr) => Expression::Not(child(expr)),
        Expression::In(expr, args) => Expression::In(
            child(expr),
            args.iter()
                .map(|expr| alpha_expression(expr, aliases))
                .collect(),
        ),
        Expression::If(a, b, c) => Expression::If(child(a), child(b), child(c)),
        Expression::Coalesce(args) => Expression::Coalesce(
            args.iter()
                .map(|expr| alpha_expression(expr, aliases))
                .collect(),
        ),
        Expression::FunctionCall(function, args) => Expression::FunctionCall(
            function.clone(),
            args.iter()
                .map(|expr| alpha_expression(expr, aliases))
                .collect(),
        ),
    }
}

fn variable_occurrences(pattern: &GraphPattern) -> Vec<Variable> {
    let mut variables = Vec::new();
    walk_pre_post(NodeRef::Pattern(pattern), |visit, node| {
        if visit == Visit::Enter {
            node.for_each_variable(|variable| variables.push(variable.clone()));
        }
        Flow::Descend
    });
    variables
}

/// Derive aliases from corresponding typed variable positions, requiring a
/// bijection and exact ordinary names. No renderer naming algorithm is repeated.
fn hidden_aliases(
    original: &GraphPattern,
    reparsed: &GraphPattern,
    reserved: &BTreeSet<Variable>,
) -> Result<BTreeMap<Variable, Variable>, String> {
    let source = variable_occurrences(original);
    let target = variable_occurrences(reparsed);
    if source.len() != target.len() {
        return Err("the carrier changed the number of variable occurrences".to_owned());
    }
    let ordinary: BTreeSet<_> = source
        .iter()
        .filter(|v| !v.is_hidden())
        .chain(reserved)
        .collect();
    let mut aliases = BTreeMap::new();
    let mut reverse = BTreeMap::new();
    for (from, to) in source.iter().zip(target) {
        if !from.is_hidden() {
            if *from != to {
                return Err("the carrier renamed an ordinary variable".to_owned());
            }
            continue;
        }
        if to.is_hidden() || ordinary.contains(&to) {
            return Err("a hidden carrier alias collides with a source variable".to_owned());
        }
        if aliases
            .insert(from.clone(), to.clone())
            .is_some_and(|old| old != to)
            || reverse
                .insert(to, from.clone())
                .is_some_and(|old| old != *from)
        {
            return Err("hidden carrier aliases are not bijective".to_owned());
        }
    }
    Ok(aliases)
}

fn flatten_join<'a>(p: &'a GraphPattern, out: &mut Vec<&'a GraphPattern>) {
    match p {
        GraphPattern::Join { left, right } => {
            flatten_join(left, out);
            flatten_join(right, out);
        }
        other => out.push(other),
    }
}

/// Strip one source projection, serialize its body, and compare the complete
/// re-parsed body modulo Join association and bijective hidden-only names.
/// Preserve a body-level projection and check the carrier's visible schema.
/// `Ok(())` on success;
/// `Err` describes the mismatch. Panics only on an internal contract
/// violation (`pattern_to_select_query`'s own re-parse yielding something
/// other than `Query::Select`, which its own doc guarantees never happens).
fn roundtrip(original: &Query) -> Result<(), String> {
    let body = where_body(original.pattern());
    let text = pattern_to_select_query(&body);
    let reparsed = SparqlParser::new()
        .parse_query(&text)
        .map_err(|e| format!("re-parse of the serialized text failed: {e}\n  text: {text}"))?;
    let Query::Select {
        pattern: reparsed_pattern,
        ..
    } = &reparsed
    else {
        panic!("pattern_to_select_query's own contract is a re-parseable SELECT; got {reparsed:?}");
    };
    let reparsed_body = if matches!(body, GraphPattern::Project { .. }) {
        reparsed_pattern.clone()
    } else {
        where_body(reparsed_pattern)
    };
    let original_norm = normalize_join_assoc(&body, &BTreeMap::new());
    let reparsed_norm = normalize_join_assoc(&reparsed_body, &BTreeMap::new());
    let aliases = hidden_aliases(&original_norm, &reparsed_norm, &BTreeSet::new())?;
    if !aliases.is_empty() && !matches!(body, GraphPattern::Project { .. }) {
        let GraphPattern::Project { variables, .. } = reparsed_pattern else {
            return Err("the hidden carrier lacks its visible projection".to_owned());
        };
        if *variables != purrdf_sparql_algebra::parser::visible_variables(&body) {
            return Err("the carrier changed the visible schema or its order".to_owned());
        }
    }
    let original_norm = normalize_join_assoc(&body, &aliases);
    if original_norm != reparsed_norm {
        return Err(format!(
            "round-trip mismatch (modulo Join re-association)\n  text: {text}\n  \
             original:  {original_norm:?}\n  reparsed:  {reparsed_norm:?}"
        ));
    }
    Ok(())
}

/// Normalize the one [`GraphUpdateOperation`] shape that embeds a
/// [`GraphPattern`] (`DeleteInsert`'s `WHERE` clause) by
/// [`normalize_join_assoc`] — the same, and only, modulo the `.rq` side of
/// this sweep allows. Every other arm carries no `GraphPattern` and is
/// returned unchanged. The match is exhaustive (no wildcard arm), matching
/// [`normalize_join_assoc`]'s own discipline: a future enum variant is a
/// compile error here until this function is taught its shape.
fn normalize_update_op(
    op: &GraphUpdateOperation,
    aliases: &BTreeMap<Variable, Variable>,
) -> GraphUpdateOperation {
    match op {
        GraphUpdateOperation::InsertData { data } => {
            GraphUpdateOperation::InsertData { data: data.clone() }
        }
        GraphUpdateOperation::DeleteData { data } => {
            GraphUpdateOperation::DeleteData { data: data.clone() }
        }
        GraphUpdateOperation::DeleteInsert {
            delete,
            insert,
            with,
            using,
            pattern,
        } => GraphUpdateOperation::DeleteInsert {
            delete: delete.clone(),
            insert: insert.clone(),
            with: with.clone(),
            using: using.clone(),
            pattern: Box::new(normalize_join_assoc(pattern, aliases)),
        },
        GraphUpdateOperation::Load {
            silent,
            source,
            destination,
        } => GraphUpdateOperation::Load {
            silent: *silent,
            source: source.clone(),
            destination: destination.clone(),
        },
        GraphUpdateOperation::Clear { silent, target } => GraphUpdateOperation::Clear {
            silent: *silent,
            target: target.clone(),
        },
        GraphUpdateOperation::Drop { silent, target } => GraphUpdateOperation::Drop {
            silent: *silent,
            target: target.clone(),
        },
        GraphUpdateOperation::Create { silent, graph } => GraphUpdateOperation::Create {
            silent: *silent,
            graph: graph.clone(),
        },
        GraphUpdateOperation::Add {
            silent,
            source,
            destination,
        } => GraphUpdateOperation::Add {
            silent: *silent,
            source: source.clone(),
            destination: destination.clone(),
        },
        GraphUpdateOperation::Move {
            silent,
            source,
            destination,
        } => GraphUpdateOperation::Move {
            silent: *silent,
            source: source.clone(),
            destination: destination.clone(),
        },
        GraphUpdateOperation::Copy {
            silent,
            source,
            destination,
        } => GraphUpdateOperation::Copy {
            silent: *silent,
            source: source.clone(),
            destination: destination.clone(),
        },
    }
}

/// Normalize every operation of `u` by [`normalize_update_op`].
fn normalize_update(u: &Update, aliases: &[BTreeMap<Variable, Variable>]) -> Update {
    Update {
        operations: u
            .operations
            .iter()
            .zip(aliases)
            .map(|(operation, names)| normalize_update_op(operation, names))
            .collect(),
        base_iri: u.base_iri.clone(),
        version: u.version.clone(),
    }
}

/// Reserve every template variable, including a predicate or quoted slot that is
/// absent from WHERE. The public shared variable walk keeps this oracle typed.
fn template_variables(operation: &GraphUpdateOperation) -> BTreeSet<Variable> {
    let (first, second): (&[_], &[_]) = match operation {
        GraphUpdateOperation::InsertData { data } | GraphUpdateOperation::DeleteData { data } => {
            (data, &[])
        }
        GraphUpdateOperation::DeleteInsert { delete, insert, .. } => (delete, insert),
        GraphUpdateOperation::Load { .. }
        | GraphUpdateOperation::Clear { .. }
        | GraphUpdateOperation::Drop { .. }
        | GraphUpdateOperation::Create { .. }
        | GraphUpdateOperation::Add { .. }
        | GraphUpdateOperation::Move { .. }
        | GraphUpdateOperation::Copy { .. } => (&[], &[]),
    };
    let mut variables = BTreeSet::new();
    for quad in first.iter().chain(second) {
        walk_pre_post(NodeRef::Triple(&quad.triple), |visit, node| {
            if visit == Visit::Enter {
                node.for_each_variable(|variable| {
                    variables.insert(variable.clone());
                });
            }
            Flow::Descend
        });
        if let Some(NamedNodePattern::Variable(variable)) = &quad.graph {
            variables.insert(variable.clone());
        }
    }
    variables
}

fn update_aliases(
    original: &Update,
    reparsed: &Update,
) -> Result<Vec<BTreeMap<Variable, Variable>>, String> {
    if original.operations.len() != reparsed.operations.len() {
        return Err("the carrier changed the number of update operations".to_owned());
    }
    let mut aliases = Vec::new();
    for (source, target) in original.operations.iter().zip(&reparsed.operations) {
        aliases.push(match (source, target) {
            (
                GraphUpdateOperation::DeleteInsert { pattern: a, .. },
                GraphUpdateOperation::DeleteInsert { pattern: b, .. },
            ) => hidden_aliases(
                &normalize_join_assoc(a, &BTreeMap::new()),
                &normalize_join_assoc(b, &BTreeMap::new()),
                &template_variables(source),
            )?,
            _ => BTreeMap::new(),
        });
    }
    Ok(aliases)
}

/// Parse `text` as an Update, `Display` it, re-parse, and compare modulo
/// [`normalize_update`]/[`normalize_join_assoc`]. `Ok(())` on success; `Err`
/// describes the mismatch. The `.ru`-side counterpart of [`roundtrip`].
fn roundtrip_update(original: &Update) -> Result<(), String> {
    let text = original.to_string();
    let reparsed = SparqlParser::new()
        .parse_update(&text)
        .map_err(|e| format!("re-parse of the serialized update failed: {e}\n  text: {text}"))?;
    let aliases = update_aliases(original, &reparsed)?;
    let original_norm = normalize_update(original, &aliases);
    let reparsed_norm =
        normalize_update(&reparsed, &vec![BTreeMap::new(); reparsed.operations.len()]);
    if original_norm != reparsed_norm {
        return Err(format!(
            "round-trip mismatch (modulo Join re-association)\n  text: {text}\n  \
             original:  {original_norm:?}\n  reparsed:  {reparsed_norm:?}"
        ));
    }
    Ok(())
}

/// (corpus-relative path or `file#sparql-block-N` label, emit-only construct
/// name). See this file's module doc's "The `XFAIL` ledger" section: an
/// entry here is a construct the PARSER can produce but the surface grammar
/// has no construct to re-emit — never a route around a real serializer bug.
const XFAIL: [(&str, &str); 0] = [];

/// The `.ru` (UPDATE) side's own `XFAIL` ledger, kept separate from the `.rq`
/// side's [`XFAIL`] so the two corpora's accounting never gets confused (see
/// this file's module doc). Empty: nothing found in the vendored corpora that
/// parses but fails to round-trip through `Display for GraphUpdateOperation`.
const RU_XFAIL: [(&str, &str); 0] = [];

/// Measured at authoring time: the count of `.rq`/doc-example corpus items
/// that are genuinely negative-syntax fixtures (W3C `NegativeSyntaxTest`
/// manifests, mainly) rather than something this parser merely does not yet
/// accept. A rise past this ceiling means one of two things: either the
/// vendored corpora grew new negative-syntax `.rq` fixtures (raise this
/// constant deliberately, remeasuring by hand) or a parser regression is
/// rejecting queries it used to accept (fix the regression — do NOT raise
/// the ceiling to paper over it). Printed alongside the raw count either
/// way; see the module doc's "Corpus items that do not even parse" section.
///
/// Remeasured to 121 for two deliberately-illegal fixtures in the extension
/// suite's own `mf:NegativeSyntaxTest` entries: a `BIND` and a `VALUES`
/// clause, each rebinding a variable inside `FILTER EXISTS { }` that is
/// already in scope on the row being filtered — a hard syntax error under
/// the existence-scope rule, not a parser gap.
///
/// Remeasured again to 122 for one more `mf:NegativeSyntaxTest` fixture
/// (`exists-scope-projection-violation.rq`): the SAME existence-scope rule,
/// now enforced at a `SELECT`-list `(expr AS ?v)` projection target too
/// (previously unenforced — the projection list parses before `WHERE`, so
/// the collision check had nothing to compare against until the deferred
/// post-`WHERE` resolution `Parser::pending_exists_scope_checks` adds).
///
/// Remeasured to 127 for the five negative syntax tests the verbatim W3C
/// `aggregates` group carries (`agg08`, `agg09`, `agg10`, `agg11`, `agg12`):
/// each projects, outside an aggregate, a variable that is no group key, which
/// the grouping constraint (SPARQL 1.1 §11.4) refuses.
const MAX_UNPARSEABLE_RQ: usize = 127;

/// The `.ru`-side counterpart of [`MAX_UNPARSEABLE_RQ`]: the measured count
/// of genuinely negative-syntax `.ru` fixtures (W3C `NegativeUpdateSyntaxTest`
/// manifests). Same discipline applies — growth means either new vendored
/// negative-syntax `.ru` fixtures (raise deliberately) or a parser
/// regression (fix it, don't raise the ceiling).
const MAX_UNPARSEABLE_RU: usize = 22;

#[test]
fn corpus_round_trips_through_the_serializer() {
    let root = workspace_root();

    let mut rq_files = Vec::new();
    let mut ru_files = Vec::new();
    for suite in [
        "crates/sparql-conformance/suite/w3c-sparql11",
        "crates/sparql-conformance/suite/w3c-sparql12",
        "crates/sparql-conformance/suite/purrdf-extend",
    ] {
        let dir = root.join(suite);
        assert!(
            dir.is_dir(),
            "expected corpus root {suite} to exist under the workspace"
        );
        collect_rq(&dir, &mut rq_files);
        collect_ru(&dir, &mut ru_files);
    }
    let doc_examples = collect_doc_examples(&root.join("docs"));

    let seen = rq_files.len() + doc_examples.len() + ru_files.len();
    println!(
        "corpus round-trip sweep: {} vendored/first-party .rq files + {} doc examples + {} \
         vendored .ru files = {seen} total",
        rq_files.len(),
        doc_examples.len(),
        ru_files.len()
    );
    // Measured at authoring time: 352 w3c-sparql11 + 242 w3c-sparql12 + 47
    // purrdf-extend .rq files, plus 11 fenced ```sparql blocks in the book's
    // temporal-arithmetic and LATERAL (SEP-0006) sections (the only file
    // under docs/ that has any) — 652 .rq/doc total — plus 148 w3c-sparql11 +
    // 23 w3c-sparql12 + 0 purrdf-extend .ru files — 171 .ru total — 823
    // combined. A corpus that stops loading (a moved suite root, a broken
    // walk) silently sweeps far less than this and must fail loudly instead.
    const N: usize = 652 + 171;
    assert!(
        seen >= N,
        "the corpus round-trip sweep saw only {seen} items, expected at least {N} — a corpus \
         that stops loading is a silent regression, not a smaller sweep"
    );

    assert_eq!(
        XFAIL.len(),
        0,
        "keep this in sync with the XFAIL array literal's length"
    );
    assert_eq!(
        RU_XFAIL.len(),
        0,
        "keep this in sync with the RU_XFAIL array literal's length"
    );

    // Several W3C fixtures reference a relative IRI (`<ng-01.ttl>`, `<g>`, …)
    // that only resolves against the manifest's own per-test base — with NO
    // base at all, those are a bare (and here, legitimate) "invalid IRI"
    // parse error, which would otherwise misclassify a perfectly
    // ROUND-TRIPPABLE query as `unparseable` and quietly shrink this sweep's
    // real coverage. The exact base value is immaterial to what this sweep
    // checks (an absolute IRI baked into the FIRST parse round-trips
    // byte-for-byte regardless of which absolute IRI it resolved to; the
    // serialized text carries it already-resolved, so the RE-parse inside
    // `roundtrip` needs no base at all) — `example.org`, per AGENTS.md's
    // fixture convention.
    let parser = SparqlParser::new().with_base_iri("https://example.org/corpus/");
    let mut unparseable = 0usize;
    let mut xfail_matched: std::collections::HashSet<&str, purrdf_hash::fixed::FixedState> =
        std::collections::HashSet::with_hasher(purrdf_hash::fixed::FixedState::new());
    let mut failures = Vec::new();
    // The `.ru` lane's counters, declared here rather than at that loop
    // because a doc example that parses as an UPDATE is routed into them
    // below, before the `.ru` files themselves are swept.
    let mut unparseable_ru = 0usize;
    let mut ru_xfail_matched: std::collections::HashSet<&str, purrdf_hash::fixed::FixedState> =
        std::collections::HashSet::with_hasher(purrdf_hash::fixed::FixedState::new());

    let mut items: Vec<(String, String)> = Vec::with_capacity(seen);
    for path in &rq_files {
        let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
        let label = path
            .strip_prefix(&root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        items.push((label, text));
    }
    // A doc example is a query OR an update (the guide teaches both), so route
    // it by what it parses as — never by assuming the query form. Anything
    // that is neither falls through to the `unparseable` tally exactly as a
    // `.rq` file would.
    let mut doc_updates = 0usize;
    for (label, text) in doc_examples {
        if parser.parse_query(&text).is_ok() {
            items.push((label, text));
            continue;
        }
        let Ok(update) = parser.parse_update(&text) else {
            unparseable += 1;
            continue;
        };
        doc_updates += 1;
        let xfail_entry = RU_XFAIL.iter().find(|(path, _)| *path == label);
        match (roundtrip_update(&update), xfail_entry) {
            (Ok(()), None) => {}
            (Ok(()), Some((path, construct))) => failures.push(format!(
                "{path}: RU_XFAIL entry for {construct:?} round-trips cleanly now — remove the \
                 ledger entry"
            )),
            (Err(msg), None) => failures.push(format!("{label}: {msg}")),
            (Err(_), Some((path, construct))) => {
                ru_xfail_matched.insert(path);
                let _ = construct;
            }
        }
    }
    println!("doc examples routed to the UPDATE lane: {doc_updates}");

    for (label, text) in &items {
        let Ok(query) = parser.parse_query(text) else {
            // Outside this sweep's contract — see the module doc's
            // "Corpus items that do not even parse" section.
            unparseable += 1;
            continue;
        };
        let xfail_entry = XFAIL.iter().find(|(path, _)| path == label);
        match (roundtrip(&query), xfail_entry) {
            (Ok(()), None) => {}
            (Ok(()), Some((path, construct))) => failures.push(format!(
                "{path}: XFAIL entry for {construct:?} round-trips cleanly now — remove the \
                 ledger entry"
            )),
            (Err(msg), None) => failures.push(format!("{label}: {msg}")),
            (Err(_), Some((path, construct))) => {
                xfail_matched.insert(path);
                let _ = construct;
            }
        }
    }

    assert_eq!(
        xfail_matched.len(),
        XFAIL.len(),
        "an XFAIL entry never matched any swept, parseable item — a dead ledger row \
         (matched: {xfail_matched:?})"
    );

    println!(
        "unparseable .rq/doc-example (skipped, outside the serializer's contract — see module \
         doc): {unparseable}"
    );
    assert!(
        unparseable <= MAX_UNPARSEABLE_RQ,
        "unparseable .rq/doc-example count {unparseable} exceeds the measured ceiling \
         MAX_UNPARSEABLE_RQ = {MAX_UNPARSEABLE_RQ} — either new negative-syntax .rq fixtures \
         landed in the vendored corpora (raise the constant deliberately) or a parser \
         regression is rejecting queries it used to accept (fix it); a sweep that silently \
         skips more items proves less, not the same"
    );

    // The `.ru` (UPDATE) side of the sweep: parse → `Display` → re-parse →
    // compare, exactly as above but through `roundtrip_update`/`RU_XFAIL`.
    // Genuinely negative-syntax `.ru` fixtures (W3C `NegativeUpdateSyntaxTest`
    // manifests among the vendored `syntax-update-*` directories) are expected
    // and counted the same way the `.rq` side counts its own unparseable
    // items — never asserted on, just accounted for below.
    for path in &ru_files {
        let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
        let label = path
            .strip_prefix(&root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");
        let Ok(update) = parser.parse_update(&text) else {
            unparseable_ru += 1;
            continue;
        };
        let xfail_entry = RU_XFAIL.iter().find(|(path, _)| *path == label);
        match (roundtrip_update(&update), xfail_entry) {
            (Ok(()), None) => {}
            (Ok(()), Some((path, construct))) => failures.push(format!(
                "{path}: RU_XFAIL entry for {construct:?} round-trips cleanly now — remove the \
                 ledger entry"
            )),
            (Err(msg), None) => failures.push(format!("{label}: {msg}")),
            (Err(_), Some((path, construct))) => {
                ru_xfail_matched.insert(path);
                let _ = construct;
            }
        }
    }

    assert_eq!(
        ru_xfail_matched.len(),
        RU_XFAIL.len(),
        "an RU_XFAIL entry never matched any swept, parseable update item (.ru file or \
         update-shaped doc example) — a dead ledger row (matched: {ru_xfail_matched:?})"
    );

    println!(
        "unparseable .ru (skipped, outside the serializer's contract — negative update syntax \
         tests among others): {unparseable_ru}"
    );
    assert!(
        unparseable_ru <= MAX_UNPARSEABLE_RU,
        "unparseable .ru count {unparseable_ru} exceeds the measured ceiling \
         MAX_UNPARSEABLE_RU = {MAX_UNPARSEABLE_RU} — either new negative-syntax .ru fixtures \
         landed in the vendored corpora (raise the constant deliberately) or a parser \
         regression is rejecting updates it used to accept (fix it); a sweep that silently \
         skips more items proves less, not the same"
    );

    assert!(
        failures.is_empty(),
        "{} corpus item(s) failed the round-trip sweep:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

#[test]
fn contextual_application_normalization_preserves_policy_and_aliases_every_slot() {
    use purrdf_sparql_algebra::algebra::{ApplicationPolicy, OptionalApplication};

    let a = Variable::hidden("a");
    let b = Variable::hidden("b");
    let marker = Variable::hidden("marker");
    let caller = Variable::new("caller");
    let alpha = Variable::new("alpha");
    let beta = Variable::new("beta");
    let gamma = Variable::new("gamma");
    let leaf = |variable| GraphPattern::Values {
        variables: vec![variable],
        bindings: vec![vec![None]],
    };
    let join = |left, right| GraphPattern::Join {
        left: Child::new(left),
        right: Child::new(right),
    };
    let operand = join(leaf(a.clone()), join(leaf(b.clone()), leaf(caller.clone())));
    let original = GraphPattern::Apply {
        left: Child::new(operand.clone()),
        right: Child::new(operand),
        policy: Box::new(ApplicationPolicy {
            dataset_required: true,
            row_pipeline: false,
            reduced_adjacent: true,
            group_domain: Some(vec![a.clone(), b.clone(), caller.clone()].into_boxed_slice()),
            inputs: vec![(a.clone(), b.clone()), (b.clone(), caller.clone())],
            optional: Some(OptionalApplication {
                retry_inputs: vec![(b.clone(), a.clone()), (marker.clone(), caller.clone())],
                forget_marker: marker.clone(),
            }),
        }),
    };
    let aliases = BTreeMap::from([
        (a, alpha.clone()),
        (b, beta.clone()),
        (marker, gamma.clone()),
    ]);
    let normalized = join(
        join(leaf(alpha.clone()), leaf(beta.clone())),
        leaf(caller.clone()),
    );
    let expected = GraphPattern::Apply {
        left: Child::new(normalized.clone()),
        right: Child::new(normalized),
        policy: Box::new(ApplicationPolicy {
            dataset_required: true,
            row_pipeline: false,
            reduced_adjacent: true,
            group_domain: Some(
                vec![alpha.clone(), beta.clone(), caller.clone()].into_boxed_slice(),
            ),
            inputs: vec![
                (alpha.clone(), beta.clone()),
                (beta.clone(), caller.clone()),
            ],
            optional: Some(OptionalApplication {
                retry_inputs: vec![(beta, alpha), (gamma.clone(), caller)],
                forget_marker: gamma,
            }),
        }),
    };
    assert_eq!(normalize_join_assoc(&original, &aliases), expected);
    assert_eq!(normalize_join_assoc(&expected, &BTreeMap::new()), expected);
}

#[test]
fn hidden_alpha_oracle_refuses_collisions_splits_merges_and_ordinary_renames() {
    let original = GraphPattern::Bgp {
        patterns: vec![
            TriplePattern {
                subject: TermPattern::Variable(Variable::hidden("a")),
                predicate: NamedNodePattern::NamedNode(patterns::example("p")),
                object: TermPattern::Variable(Variable::hidden("b")),
            },
            TriplePattern {
                subject: TermPattern::Variable(Variable::hidden("a")),
                predicate: NamedNodePattern::NamedNode(patterns::example("q")),
                object: TermPattern::Variable(Variable::new("caller")),
            },
        ],
    };
    let expected = normalize_join_assoc(
        &original,
        &BTreeMap::from([
            (Variable::hidden("a"), Variable::new("alpha")),
            (Variable::hidden("b"), Variable::new("beta")),
        ]),
    );
    let aliases = hidden_aliases(&original, &expected, &BTreeSet::new())
        .expect("bijective hidden-only aliases");
    assert_eq!(normalize_join_assoc(&original, &aliases), expected);
    for case in 0..4 {
        let mut bad = expected.clone();
        let GraphPattern::Bgp { patterns } = &mut bad else {
            panic!("BGP");
        };
        match case {
            0 => patterns[0].subject = TermPattern::Variable(Variable::new("caller")),
            1 => patterns[0].object = TermPattern::Variable(Variable::new("alpha")),
            2 => patterns[1].subject = TermPattern::Variable(Variable::new("other")),
            3 => patterns[1].object = TermPattern::Variable(Variable::new("other")),
            _ => unreachable!(),
        }
        assert!(
            hidden_aliases(&original, &bad, &BTreeSet::new()).is_err(),
            "case {case}"
        );
    }
    let mut changed_constant = expected;
    let GraphPattern::Bgp { patterns } = &mut changed_constant else {
        panic!("BGP");
    };
    patterns[0].predicate = NamedNodePattern::NamedNode(patterns::example("different"));
    let aliases = hidden_aliases(&original, &changed_constant, &BTreeSet::new())
        .expect("names alone still biject");
    assert_ne!(normalize_join_assoc(&original, &aliases), changed_constant);
}

#[test]
fn update_carrier_aliases_reserve_template_only_variables_in_every_slot() {
    let parser = SparqlParser::new();
    for template in [
        "?__purrdf_hidden_0 <http://example.org/new> ?o",
        "?o ?__purrdf_hidden_0 <http://example.org/new>",
        "<<( ?__purrdf_hidden_0 <http://example.org/new> ?o )>> <http://example.org/marked> true",
        "<<( ?o ?__purrdf_hidden_0 <http://example.org/new> )>> <http://example.org/marked> true",
        "<<( ?o <http://example.org/new> ?__purrdf_hidden_0 )>> <http://example.org/marked> true",
        "GRAPH ?__purrdf_hidden_0 { ?o <http://example.org/marked> true }",
    ] {
        for keyword in ["INSERT", "DELETE"] {
            let source = parser.parse_update(&format!(
                "{keyword} {{ {template} }} WHERE {{ ?s (<http://example.org/p>|<http://example.org/q>)/<http://example.org/r> ?o }}"
            )).expect("an update template and alternative parse");
            roundtrip_update(&source)
                .expect("all output variables stay exact and hidden aliases stay fresh");
            let text = source.to_string();
            assert!(text.contains("?___purrdf_hidden_0"), "{text}");
            let captured = parser
                .parse_update(&text.replace("?___purrdf_hidden_0", "?__purrdf_hidden_0"))
                .expect("a syntactically legal carrier that captures an unbound template name");
            assert!(
                update_aliases(&source, &captured).is_err(),
                "template-only capture is refused"
            );
        }
    }
}

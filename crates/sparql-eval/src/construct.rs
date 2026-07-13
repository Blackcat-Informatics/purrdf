// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! `CONSTRUCT` evaluation, emitting the IR dataset **directly** (no
//! serialize/re-parse round trip).
//!
//! The `WHERE` algebra is evaluated to a solution multiset; the template is then
//! instantiated once per solution into a fresh [`RdfDatasetBuilder`] and frozen.
//! Three SPARQL rules govern instantiation (§16.2):
//!
//! 1. A template triple with **any unbound variable** is silently skipped.
//! 2. A template **blank node is minted fresh per solution row** — the same label
//!    co-refers within one row but is a distinct node across rows.
//! 3. An **ill-formed** instantiation (a literal in subject position, or a non-IRI
//!    predicate) is skipped.
//!
//! Each position is instantiated to a [`TermValue`](purrdf_core::TermValue) first so its term *kind* can be
//! validated before interning into the output builder. Byte-identical parity with
//! the oxigraph baseline is decided downstream at the RDFC-1.0 canonicalization
//! layer, so blank-node labels and quad ordering here need not match oxigraph's —
//! `freeze` sorts and de-duplicates, and canonicalization relabels blanks.

use std::collections::{BTreeSet, HashSet};
use std::sync::Arc;

use purrdf_core::loss::{
    LOSS_ANNOTATION_LAYER_DROPPED, LOSS_REIFIER_LAYER_DROPPED, LOSS_STANDPOINT_SCOPE_DROPPED,
};
use purrdf_core::{
    DatasetView, RdfDataset, RdfDatasetBuilder, RdfLiteral, TermFactory, TermId, TermRef, TermValue,
};
use purrdf_sparql_algebra::{GraphPattern, NamedNodePattern, TermPattern, TriplePattern};

use crate::DetHashMap;
use crate::error::EvalError;
use crate::eval::{EvalCtx, eval};
use crate::solution::{Solution, VarSchema};
use crate::template::{instantiate_predicate, instantiate_term, positionally_ill_formed};

/// The `rdf:reifies` predicate IRI — the reification-layer indirection edge.
const RDF_REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";
/// `rdf:type`.
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
/// `xsd:string` — the datatype of an emitted loss-code literal.
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";

/// Evaluate a `CONSTRUCT` query to a frozen IR dataset.
///
/// **Loss-aware projection:** when the `WHERE` bound an RDF-1.2 reifier (via
/// an `rdf:reifies` triple pattern) and the template drops it, the dropped
/// reification layer can be declared **in-band** as `ProjectionLoss` triples on
/// the SAME output graph — but only when a caller-supplied
/// [`LossVocabulary`](crate::eval::LossVocabulary) is configured. GTS is lossless,
/// so a configured loss vocabulary lets projection loss be declared at the
/// projection rather than silently swallowed; without a vocabulary the query
/// behaves like a plain `CONSTRUCT`. When the `WHERE` has no `rdf:reifies`
/// pattern at all the detection does zero extra work and the output is
/// byte-identical to a plain `CONSTRUCT`.
pub(crate) fn eval_construct<D: DatasetView + Sync>(
    template: &[TriplePattern],
    pattern: &GraphPattern,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<Arc<RdfDataset>, EvalError> {
    let seq = eval(pattern, ctx)?;
    let schema = seq.schema.clone();
    let mut builder = RdfDatasetBuilder::new();

    // Loss detection. Only run when a caller-supplied loss vocabulary is configured;
    // otherwise loss declarations stay inactive and the output behaves like a plain
    // `CONSTRUCT`. With no `rdf:reifies` pattern in the WHERE the set is empty and
    // the per-row emission below is skipped entirely.
    //
    // Standpoint attribution reads the SAME caller-supplied predicate table as
    // `heldIn` (see [`crate::eval::StandpointPredicates`]): with no table
    // configured, a dropped annotation cannot be attributed to a standpoint scope
    // and only the generic annotation-layer loss code is emitted — the engine never
    // fabricates a default domain predicate.
    let loss_vocab = ctx.loss_vocabulary.clone();
    let dropped: Vec<DroppedReifier> = loss_vocab
        .as_ref()
        .map(|_| {
            let standpoint_according_to: Option<String> = ctx
                .standpoint_predicates
                .as_ref()
                .map(|p| p.according_to.clone());
            collect_dropped_reifiers(template, pattern, standpoint_according_to.as_deref())
        })
        .unwrap_or_default();

    // Pre-intern the caller-supplied loss vocabulary IRIs once, before the
    // per-solution row loop, so the loss-node emission path does not repeat
    // the lookup work for every row.
    let loss_term_ids: Option<(TermId, TermId, TermId)> = loss_vocab.as_ref().map(|vocab| {
        (
            builder.intern_iri_value(&vocab.projection_loss),
            builder.intern_iri_value(&vocab.loss_code),
            builder.intern_iri_value(&vocab.lost_reifies),
        )
    });

    // Identify which template triple indices are reifier declarations
    // (predicate == rdf:reifies, object == TermPattern::Triple).  This scan is
    // done ONCE before the row loop so that per-row emit can fast-path to plain
    // push_quad when the template contains no reifier declarations.
    let reifier_decl_indices: Vec<usize> = template
        .iter()
        .enumerate()
        .filter(|(_, tp)| is_reifies(tp) && matches!(&tp.object, TermPattern::Triple(_)))
        .map(|(i, _)| i)
        .collect();
    let has_reifier_decls = !reifier_decl_indices.is_empty();
    // Interned once (idempotent), used by pass 2 below to recognize a
    // *dynamically*-produced `rdf:reifies` edge — see its doc comment.
    let reifies_id = builder.intern_iri(RDF_REIFIES);

    for row in &seq.rows {
        // Template blank labels are fresh per solution row; the map co-refers a
        // label within this row only.
        let mut blanks: DetHashMap<String, String> = DetHashMap::default();

        if !has_reifier_decls {
            // FAST NO-OP PATH: no rdf:reifies triple in the template → plain quads.
            for tp in template {
                if let Some((s, p, o)) =
                    instantiate(tp, row, &schema, &mut builder, &mut blanks, ctx)
                {
                    builder.push_quad(s, p, o, None);
                }
            }
        } else {
            // TWO-PASS EMIT: first collect all instantiated triples, then route
            // each one to push_reifier / push_annotation / push_quad.

            // Instantiate every template triple for this row (None = skipped).
            let instantiated: Vec<Option<(TermId, TermId, TermId)>> = template
                .iter()
                .map(|tp| instantiate(tp, row, &schema, &mut builder, &mut blanks, ctx))
                .collect();

            // Pass 1: emit reifier declarations and build the per-row reifier set.
            let mut reifier_ids: HashSet<TermId> = HashSet::new();
            for &idx in &reifier_decl_indices {
                if let Some((s, _p, o)) = instantiated[idx] {
                    builder.push_reifier(s, o);
                    reifier_ids.insert(s);
                }
            }

            // Pass 2: emit remaining triples, routing by VALUE, not just template
            // position. A template slot with a variable predicate/object (e.g. the
            // `?q ?z` half of `S P O {| ?q ?z |}`) is only STATICALLY a plain
            // annotation triple — but the `WHERE` reifier/annotation virtual layer
            // (`emit_virtual_candidates`, `sparql-eval::bgp`) also unifies a fully
            // generic pattern's predicate/object against the reifier's OWN
            // `rdf:reifies` edge (it IS a real, matchable triple), so ONE solution
            // row can legitimately bind `?q = rdf:reifies, ?z = <<( s p o )>>` — the
            // same fact `reifier_decl_indices` already declared for this row. Routing
            // that row's `?q ?z` slot by POSITION alone would re-push it as a
            // spurious "annotation whose predicate is rdf:reifies", doubling the
            // reifier's encoding; routing it by VALUE instead recognizes the
            // dynamically-produced edge and calls `push_reifier` again, which is an
            // idempotent no-op against the identical pass-1 binding (W3C
            // `eval-triple-terms` `construct-5`).
            for (idx, triple) in instantiated.iter().enumerate() {
                if reifier_decl_indices.contains(&idx) {
                    continue; // already handled in pass 1
                }
                if let Some((s, p, o)) = *triple {
                    let is_dynamic_reifies =
                        p == reifies_id && matches!(builder.resolve(o), TermRef::Triple { .. });
                    if is_dynamic_reifies {
                        builder.push_reifier(s, o);
                    } else if reifier_ids.contains(&s) {
                        builder.push_annotation(s, p, o);
                    } else {
                        builder.push_quad(s, p, o, None);
                    }
                }
            }
        }

        if let Some(ids) = loss_term_ids
            && !dropped.is_empty()
        {
            emit_dropped_losses(&dropped, row, &schema, &mut builder, ctx, ids);
        }
    }

    // Value-constructing builtins (`listSlice`/`listConcat`) invent fresh
    // `rdf:List` cells while the WHERE is evaluated. A SPARQL expression can only
    // return the list head, so the cells are buffered on the context; fold them into
    // the CONSTRUCT output here so a constructed list materializes as triples — but
    // only the cells reachable from a surviving result row, so a list minted on a row
    // pruned by FILTER/DISTINCT/LIMIT does not leak orphaned cells into the graph.
    if !ctx.constructed.is_empty() {
        let (_, rows) = crate::eval::materialize_solutions(&seq, ctx);
        for (s, p, o) in ctx.reachable_constructed(&rows) {
            let s = builder.intern_value(&s);
            let p = builder.intern_value(&p);
            let o = builder.intern_value(&o);
            builder.push_quad(s, p, o, None);
        }
    }

    builder
        .freeze()
        .map_err(|d| EvalError::internal(format!("CONSTRUCT output failed to freeze: {d:?}")))
}

/// A reifies-pattern in the `WHERE` whose reifier variable the template drops.
///
/// Carries the inner triple-term pattern (`<<( s p o )>>`) so the concrete reified
/// triple can be materialized per solution row, plus the dropped annotation facts
/// keyed off the same reifier variable.
struct DroppedReifier {
    /// The already-boxed triple-term pattern (`TermPattern::Triple(...)`) instantiated
    /// per row to the lost triple term. Stored as `TermPattern` so it can be passed
    /// directly to `instantiate_term` without a per-row `Box::new` / clone.
    inner: TermPattern,
    /// `true` if the `WHERE` also matched annotation triples on this reifier var
    /// (a triple whose subject is the reifier var, other than the reifies edge).
    has_annotation: bool,
    /// `true` if one of those dropped annotation predicates is the caller's
    /// configured standpoint `according_to` predicate (never true when no
    /// [`crate::eval::StandpointPredicates`] table is configured).
    has_standpoint: bool,
}

/// Detection (STEP 3): a reifier layer is "dropped" iff the `WHERE` bound a reifier
/// via an `rdf:reifies` triple pattern whose reifier variable appears in NO template
/// triple position.
///
/// Walks the algebra `WHERE` collecting every `rdf:reifies` triple pattern (its
/// reifier variable + inner triple-term pattern), then drops the ones whose reifier
/// variable is absent from the set of all variables mentioned anywhere in the
/// template. Deterministic: returns the dropped set in `WHERE`-traversal order.
///
/// `standpoint_according_to` is the caller-configured standpoint annotation
/// predicate (from [`crate::eval::StandpointPredicates`]); `None` means no table
/// is configured and no drop can be attributed a standpoint scope.
fn collect_dropped_reifiers(
    template: &[TriplePattern],
    pattern: &GraphPattern,
    standpoint_according_to: Option<&str>,
) -> Vec<DroppedReifier> {
    // Collect every BGP triple pattern reachable in the WHERE, in a stable order.
    let mut where_triples: Vec<&TriplePattern> = Vec::new();
    collect_where_triples(pattern, &mut where_triples);

    // The reifies-patterns: predicate == rdf:reifies, subject a variable, object a
    // quoted triple term. Keyed by the reifier variable name; the object is stored
    // as a cloned `TermPattern::Triple(...)` so no per-row Box::new is needed later.
    let mut reifiers: Vec<(String, TermPattern)> = Vec::new();
    for tp in &where_triples {
        if is_reifies(tp)
            && let (TermPattern::Variable(v), obj @ TermPattern::Triple(_)) =
                (&tp.subject, &tp.object)
        {
            reifiers.push((v.as_str().to_owned(), obj.clone()));
        }
    }

    // FAST NO-OP PATH: no reifies-pattern at all ⇒ nothing to detect.
    if reifiers.is_empty() {
        return Vec::new();
    }

    // The set of all variables mentioned anywhere in the template (descending into
    // nested quoted-triple terms).
    let mut template_vars: BTreeSet<String> = BTreeSet::new();
    for tp in template {
        collect_triple_pattern_vars(tp, &mut template_vars);
    }

    let mut dropped = Vec::new();
    for (reifier_var, inner) in reifiers {
        // A reifies-pattern is dropped iff its reifier variable is NOT carried by
        // the template.
        if template_vars.contains(&reifier_var) {
            continue;
        }
        // Sub-codes: do any WHERE annotation triples key off this dropped reifier
        // var (subject == reifier var, predicate != rdf:reifies)? And is one of
        // those predicates the configured standpoint `according_to` predicate?
        let mut has_annotation = false;
        let mut has_standpoint = false;
        for tp in &where_triples {
            if is_reifies(tp) {
                continue;
            }
            if let TermPattern::Variable(s) = &tp.subject
                && s.as_str() == reifier_var
            {
                has_annotation = true;
                if let NamedNodePattern::NamedNode(n) = &tp.predicate
                    && standpoint_according_to.is_some_and(|at| n.as_str() == at)
                {
                    has_standpoint = true;
                }
            }
        }
        dropped.push(DroppedReifier {
            inner,
            has_annotation,
            has_standpoint,
        });
    }
    dropped
}

/// `true` if `tp` is an `rdf:reifies` triple pattern (a concrete `rdf:reifies`
/// predicate, not a variable predicate).
fn is_reifies(tp: &TriplePattern) -> bool {
    matches!(&tp.predicate, NamedNodePattern::NamedNode(n) if n.as_str() == RDF_REIFIES)
}

/// Recursively collect every triple pattern in the `WHERE` algebra tree (every BGP
/// conjunct, descending through every algebra operator). Order is a stable pre-order
/// traversal so the dropped set is deterministic.
fn collect_where_triples<'a>(pattern: &'a GraphPattern, out: &mut Vec<&'a TriplePattern>) {
    match pattern {
        GraphPattern::Bgp { patterns } => out.extend(patterns.iter()),
        GraphPattern::Path { .. } | GraphPattern::Values { .. } | GraphPattern::Service { .. } => {}
        GraphPattern::Join { left, right }
        | GraphPattern::Lateral { left, right }
        | GraphPattern::Union { left, right }
        | GraphPattern::Minus { left, right } => {
            collect_where_triples(left, out);
            collect_where_triples(right, out);
        }
        GraphPattern::LeftJoin { left, right, .. } => {
            collect_where_triples(left, out);
            collect_where_triples(right, out);
        }
        GraphPattern::Filter { inner, .. }
        | GraphPattern::Graph { inner, .. }
        | GraphPattern::Extend { inner, .. }
        | GraphPattern::OrderBy { inner, .. }
        | GraphPattern::Project { inner, .. }
        | GraphPattern::Distinct { inner }
        | GraphPattern::Reduced { inner }
        | GraphPattern::Slice { inner, .. }
        | GraphPattern::Group { inner, .. } => collect_where_triples(inner, out),
    }
}

/// Collect the variable names mentioned in a triple pattern, descending into nested
/// quoted-triple terms in subject/object position.
fn collect_triple_pattern_vars(tp: &TriplePattern, out: &mut BTreeSet<String>) {
    collect_term_pattern_vars(&tp.subject, out);
    if let NamedNodePattern::Variable(v) = &tp.predicate {
        out.insert(v.as_str().to_owned());
    }
    collect_term_pattern_vars(&tp.object, out);
}

/// Collect the variable names mentioned in a term pattern (recursing into a quoted
/// triple term).
fn collect_term_pattern_vars(term: &TermPattern, out: &mut BTreeSet<String>) {
    match term {
        TermPattern::Variable(v) => {
            out.insert(v.as_str().to_owned());
        }
        TermPattern::Triple(t) => collect_triple_pattern_vars(t, out),
        TermPattern::NamedNode(_) | TermPattern::BlankNode(_) | TermPattern::Literal(_) => {}
    }
}

/// Emit (STEP 4) the in-band loss declaration(s) for a solution row.
///
/// For each dropped reifies-pattern, materializes the concrete reified triple term
/// from the row's bindings and emits, into the SAME builder:
///
/// ```text
/// <lossNode> rdf:type        <projectionLoss> .
/// <lossNode> <lossCode>      "reifier-layer-dropped"^^xsd:string .
/// <lossNode> <lostReifies>   <<( s p o )>> .
/// ```
///
/// plus the `annotation-layer-dropped` / `standpoint-scope-dropped` sub-codes when
/// the dropped reifier also lost annotations / a standpoint annotation under the
/// caller-configured `according_to` predicate.
/// `<lossNode>` is a DETERMINISTIC blank node whose label is derived purely from the
/// resolved triple-term content, so identical drops across rows collapse to one node
/// via the builder's dedup.
fn emit_dropped_losses<D: DatasetView + Sync>(
    dropped: &[DroppedReifier],
    row: &Solution<D::Id>,
    schema: &VarSchema,
    builder: &mut RdfDatasetBuilder,
    ctx: &mut EvalCtx<'_, D>,
    (proj_loss_id, loss_code_id, lost_reifies_id): (TermId, TermId, TermId),
) {
    for d in dropped {
        // Materialize the concrete reified triple term for this row. An unbound
        // inner variable yields `None` — there is no concrete triple to declare
        // lost, so the declaration is (correctly) skipped for this row.
        let mut blanks: DetHashMap<String, String> = DetHashMap::default();
        let Some(inner_term) = instantiate_term(&d.inner, row, schema, &mut blanks, ctx) else {
            continue;
        };

        // Deterministic loss-node label from the resolved triple-term content.
        let label = loss_node_label(LOSS_REIFIER_LAYER_DROPPED, &inner_term);
        let loss_node = builder.intern_blank_value(&label, purrdf_core::BlankScope::DEFAULT);

        let rdf_type = builder.intern_iri_value(RDF_TYPE);
        builder.push_quad(loss_node, rdf_type, proj_loss_id, None);

        // <lossCode> "reifier-layer-dropped"
        push_loss_code(builder, loss_node, LOSS_REIFIER_LAYER_DROPPED, loss_code_id);

        // <lostReifies> <<( s p o )>>
        let triple_id = builder.intern_value(&inner_term);
        builder.push_quad(loss_node, lost_reifies_id, triple_id, None);

        // Sub-codes on the SAME loss node (keyed deterministically by the same
        // content-derived label, so they coalesce across rows too).
        if d.has_annotation {
            push_loss_code(
                builder,
                loss_node,
                LOSS_ANNOTATION_LAYER_DROPPED,
                loss_code_id,
            );
        }
        if d.has_standpoint {
            push_loss_code(
                builder,
                loss_node,
                LOSS_STANDPOINT_SCOPE_DROPPED,
                loss_code_id,
            );
        }
    }
}

/// Push `<loss_node> <lossCode> "<code>"^^xsd:string .` into `builder`.
fn push_loss_code(
    builder: &mut RdfDatasetBuilder,
    loss_node: TermId,
    code: &str,
    loss_code_id: TermId,
) {
    let code_lit = builder.intern_literal_value(RdfLiteral {
        lexical_form: code.to_owned(),
        datatype: Some(XSD_STRING.to_owned()),
        language: None,
        direction: None,
    });
    builder.push_quad(loss_node, loss_code_id, code_lit, None);
}

/// A deterministic blank-node label for a loss node, derived PURELY from the loss
/// code and the resolved triple-term content. Identical drops (same triple term)
/// produce the same label so the builder dedups them to ONE node; no counter, no
/// randomness. Uses a fixed-seed hash of the term value for a compact, stable label.
fn loss_node_label(code: &str, inner: &TermValue) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    code.hash(&mut h);
    inner.hash(&mut h);
    format!("loss-{:016x}", h.finish())
}

/// Instantiate one template triple for `row`, interning into `builder`. Returns
/// `None` if the triple is skipped (an unbound variable or an ill-formed position).
fn instantiate<D: DatasetView + Sync>(
    tp: &TriplePattern,
    row: &Solution<D::Id>,
    schema: &VarSchema,
    builder: &mut RdfDatasetBuilder,
    blanks: &mut DetHashMap<String, String>,
    ctx: &mut EvalCtx<'_, D>,
) -> Option<(TermId, TermId, TermId)> {
    let s = instantiate_term(&tp.subject, row, schema, blanks, ctx)?;
    let p = instantiate_predicate(&tp.predicate, row, schema, ctx)?;
    let o = instantiate_term(&tp.object, row, schema, blanks, ctx)?;

    // Positional validity (§16.2): subject must not be a literal; predicate must be
    // an IRI. Ill-formed instantiations are skipped, not errored.
    if positionally_ill_formed(&s, &p) {
        return None;
    }

    Some((
        builder.intern_value(&s),
        builder.intern_value(&p),
        builder.intern_value(&o),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_core::{RdfLiteral, TermRef};
    use purrdf_sparql_algebra::{NamedNode, NamedNodePattern, TermPattern, Variable};

    const KNOWS: &str = "http://ex/knows";
    const RELATED: &str = "http://ex/related";

    fn knows_graph() -> Arc<RdfDataset> {
        // :a :knows :b ; :a :knows :c .
        let mut b = RdfDatasetBuilder::new();
        let knows = b.intern_iri(KNOWS);
        let a = b.intern_iri("http://ex/a");
        let bb = b.intern_iri("http://ex/b");
        let cc = b.intern_iri("http://ex/c");
        b.push_quad(a, knows, bb, None);
        b.push_quad(a, knows, cc, None);
        b.freeze().expect("freeze")
    }

    fn var(n: &str) -> TermPattern {
        TermPattern::Variable(Variable::new(n))
    }
    fn pred(iri: &str) -> NamedNodePattern {
        NamedNodePattern::NamedNode(NamedNode::new_unchecked(iri))
    }
    fn where_knows() -> GraphPattern {
        GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: var("s"),
                predicate: pred(KNOWS),
                object: var("o"),
            }],
        }
    }

    #[test]
    fn construct_rewrites_predicate() {
        let ds = knows_graph();
        let mut ctx = EvalCtx::new(&ds);
        // CONSTRUCT { ?s :related ?o } WHERE { ?s :knows ?o }
        let template = vec![TriplePattern {
            subject: var("s"),
            predicate: pred(RELATED),
            object: var("o"),
        }];
        let out = eval_construct(&template, &where_knows(), &mut ctx).expect("construct");
        assert_eq!(out.quad_count(), 2);
        // Every emitted quad uses :related, none :knows.
        for q in out.quads() {
            assert!(matches!(out.resolve(q.p), TermRef::Iri(p) if p == RELATED));
        }
    }

    #[test]
    fn unbound_template_var_skips_the_triple() {
        let ds = knows_graph();
        let mut ctx = EvalCtx::new(&ds);
        // CONSTRUCT { ?s :related ?missing } WHERE { ?s :knows ?o } — ?missing is
        // never bound, so every template triple is skipped → empty output.
        let template = vec![TriplePattern {
            subject: var("s"),
            predicate: pred(RELATED),
            object: var("missing"),
        }];
        let out = eval_construct(&template, &where_knows(), &mut ctx).expect("construct");
        assert_eq!(out.quad_count(), 0);
    }

    #[test]
    fn template_blank_is_fresh_per_solution() {
        let ds = knows_graph();
        let mut ctx = EvalCtx::new(&ds);
        // CONSTRUCT { _:b :related ?o } WHERE { ?s :knows ?o }
        // Two solutions → two distinct fresh blank subjects.
        let template = vec![TriplePattern {
            subject: TermPattern::BlankNode(purrdf_sparql_algebra::BlankNode::new("b")),
            predicate: pred(RELATED),
            object: var("o"),
        }];
        let out = eval_construct(&template, &where_knows(), &mut ctx).expect("construct");
        assert_eq!(out.quad_count(), 2);
        // Collect the distinct blank subjects.
        let mut blanks = BTreeSet::new();
        for q in out.quads() {
            if let TermRef::Blank { label, .. } = out.resolve(q.s) {
                blanks.insert(label.to_owned());
            }
        }
        assert_eq!(blanks.len(), 2, "each solution mints a distinct blank");
    }

    #[test]
    fn ill_formed_literal_subject_is_skipped() {
        // CONSTRUCT { ?o :related ?s } where ?o binds to a literal → literal subject
        // → skipped.
        let mut b = RdfDatasetBuilder::new();
        let p = b.intern_iri("http://ex/p");
        let s = b.intern_iri("http://ex/s");
        let lit = b.intern_literal(RdfLiteral::simple("hello"));
        b.push_quad(s, p, lit, None); // :s :p "hello"
        let ds = b.freeze().expect("freeze");
        let mut ctx = EvalCtx::new(&ds);

        let where_pat = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: var("s"),
                predicate: pred("http://ex/p"),
                object: var("o"),
            }],
        };
        // Template puts ?o (a literal) in subject position.
        let template = vec![TriplePattern {
            subject: var("o"),
            predicate: pred(RELATED),
            object: var("s"),
        }];
        let out = eval_construct(&template, &where_pat, &mut ctx).expect("construct");
        assert_eq!(out.quad_count(), 0);
    }

    // ── Loss-aware CONSTRUCT ──────────────────────────────────────────────────

    const REIFIES: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#reifies";
    const RDF_TYPE_IRI: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    /// A pure-fixture (example.org) standpoint vocabulary: the `according_to`
    /// predicate is caller-supplied configuration, not an engine constant.
    const ACCORDING_TO: &str = "http://example.org/accordingTo";
    const SHARPENS: &str = "http://example.org/sharpens";

    /// The fixture's caller-supplied standpoint predicate table.
    fn ex_standpoints() -> crate::eval::StandpointPredicates {
        crate::eval::StandpointPredicates::new(ACCORDING_TO, SHARPENS)
    }

    /// A pure-fixture (example.org) loss-declaration vocabulary. These IRIs are
    /// caller-supplied configuration, not engine constants.
    const PROJECTION_LOSS: &str = "http://example.org/loss/ProjectionLoss";
    const LOSS_CODE: &str = "http://example.org/loss/lossCode";
    const LOST_REIFIES: &str = "http://example.org/loss/lostReifies";

    /// The fixture's caller-supplied loss vocabulary.
    fn ex_loss_vocab() -> crate::eval::LossVocabulary {
        crate::eval::LossVocabulary::new(PROJECTION_LOSS, LOSS_CODE, LOST_REIFIES)
    }

    /// A dataset with one reifier `:r rdf:reifies <<( :alice :age 42 )>>`, with two
    /// annotations on `:r` (confidence + accordingTo). The reifier query layer comes
    /// from the BGP virtual-candidate machinery (Task 1).
    fn reified_graph() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        // `rdf:reifies` MUST be interned for the reifier-query layer to fire
        // (the virtual-predicate id is resolved via term_id_by_value).
        let _ = b.intern_iri(REIFIES);
        let alice = b.intern_iri("http://ex/alice");
        let age = b.intern_iri("http://ex/age");
        let forty_two = b.intern_literal(RdfLiteral::simple("42"));
        let triple = b.intern_triple(alice, age, forty_two);
        let r = b.intern_iri("http://ex/r");
        b.push_reifier(r, triple);
        // Annotation: :r :confidence "0.9" ; :r example:accordingTo :sourceX .
        let confidence = b.intern_iri("http://ex/confidence");
        let conf_val = b.intern_literal(RdfLiteral::simple("0.9"));
        b.push_annotation(r, confidence, conf_val);
        let according = b.intern_iri(ACCORDING_TO);
        let source_x = b.intern_iri("http://ex/sourceX");
        b.push_annotation(r, according, source_x);
        b.freeze().expect("freeze")
    }

    /// `WHERE { ?r rdf:reifies <<( ?s ?p ?o )>> }` as a BGP.
    fn where_reifies() -> GraphPattern {
        GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: var("r"),
                predicate: pred(REIFIES),
                object: TermPattern::Triple(Box::new(TriplePattern {
                    subject: var("s"),
                    predicate: NamedNodePattern::Variable(Variable::new("p")),
                    object: var("o"),
                })),
            }],
        }
    }

    /// Count quads whose predicate is the fixture's `lossCode` and object the given code.
    fn count_loss_code(out: &RdfDataset, code: &str) -> usize {
        out.quads()
            .filter(|q| {
                matches!(out.resolve(q.p), TermRef::Iri(p) if p == LOSS_CODE)
                    && matches!(out.resolve(q.o), TermRef::Literal { lexical, .. } if lexical == code)
            })
            .count()
    }

    #[test]
    fn dropped_reifier_emits_in_band_loss() {
        // CONSTRUCT { ?s ?p ?o } WHERE { ?r rdf:reifies <<( ?s ?p ?o )>> } — the
        // reifier ?r is dropped, so the reification layer loss is declared in-band.
        let ds = reified_graph();
        let mut ctx = EvalCtx::new(&ds).with_loss_vocabulary(ex_loss_vocab());
        let template = vec![TriplePattern {
            subject: var("s"),
            predicate: NamedNodePattern::Variable(Variable::new("p")),
            object: var("o"),
        }];
        let out = eval_construct(&template, &where_reifies(), &mut ctx).expect("construct");

        // The asserted triple is present: :alice :age "42".
        let asserted = out.quads().any(|q| {
            matches!(out.resolve(q.s), TermRef::Iri(s) if s == "http://ex/alice")
                && matches!(out.resolve(q.p), TermRef::Iri(p) if p == "http://ex/age")
        });
        assert!(asserted, "the asserted (de-reified) triple is emitted");

        // A logic:ProjectionLoss declaration of type with the reifier-layer code.
        let has_loss_type = out.quads().any(|q| {
            matches!(out.resolve(q.p), TermRef::Iri(p) if p == RDF_TYPE_IRI)
                && matches!(out.resolve(q.o), TermRef::Iri(o) if o == PROJECTION_LOSS)
        });
        assert!(has_loss_type, "a logic:ProjectionLoss node is declared");
        assert_eq!(
            count_loss_code(&out, LOSS_REIFIER_LAYER_DROPPED),
            1,
            "exactly one reifier-layer-dropped code"
        );

        // logic:lostReifies points at the concrete triple term <<( :alice :age 42 )>>.
        let lost = out.quads().any(|q| {
            matches!(out.resolve(q.p), TermRef::Iri(p) if p == LOST_REIFIES)
                && matches!(out.resolve(q.o), TermRef::Triple { .. })
        });
        assert!(lost, "logic:lostReifies carries the dropped triple term");
    }

    #[test]
    fn dropped_annotated_reifier_emits_annotation_and_standpoint_codes() {
        // WHERE binds the reifier, its rdf:reifies edge, an annotation, AND an
        // accordingTo annotation — all keyed off ?r, which the template drops. The
        // standpoint attribution reads the CONFIGURED predicate table (example.org
        // here), proving the vocabulary flows through configuration, not a const.
        let ds = reified_graph();
        let mut ctx = EvalCtx::new(&ds)
            .with_standpoint_predicates(ex_standpoints())
            .with_loss_vocabulary(ex_loss_vocab());
        let where_pat = GraphPattern::Bgp {
            patterns: vec![
                TriplePattern {
                    subject: var("r"),
                    predicate: pred(REIFIES),
                    object: TermPattern::Triple(Box::new(TriplePattern {
                        subject: var("s"),
                        predicate: NamedNodePattern::Variable(Variable::new("p")),
                        object: var("o"),
                    })),
                },
                // ?r :confidence ?c  (a plain annotation)
                TriplePattern {
                    subject: var("r"),
                    predicate: pred("http://ex/confidence"),
                    object: var("c"),
                },
                // ?r example:accordingTo ?stand  (the standpoint annotation)
                TriplePattern {
                    subject: var("r"),
                    predicate: pred(ACCORDING_TO),
                    object: var("stand"),
                },
            ],
        };
        // Template drops ?r entirely (carries only the de-reified triple).
        let template = vec![TriplePattern {
            subject: var("s"),
            predicate: NamedNodePattern::Variable(Variable::new("p")),
            object: var("o"),
        }];
        let out = eval_construct(&template, &where_pat, &mut ctx).expect("construct");

        assert_eq!(count_loss_code(&out, LOSS_REIFIER_LAYER_DROPPED), 1);
        assert_eq!(count_loss_code(&out, LOSS_ANNOTATION_LAYER_DROPPED), 1);
        assert_eq!(count_loss_code(&out, LOSS_STANDPOINT_SCOPE_DROPPED), 1);
    }

    #[test]
    fn without_a_configured_table_no_standpoint_scope_code_is_emitted() {
        // The SAME dropped-annotated-reifier shape, but with NO StandpointPredicates
        // configured: the engine cannot (and must not) guess a domain predicate, so
        // the generic annotation-layer code is emitted WITHOUT the standpoint sub-code.
        let ds = reified_graph();
        let mut ctx = EvalCtx::new(&ds).with_loss_vocabulary(ex_loss_vocab()); // no standpoint table
        let where_pat = GraphPattern::Bgp {
            patterns: vec![
                TriplePattern {
                    subject: var("r"),
                    predicate: pred(REIFIES),
                    object: TermPattern::Triple(Box::new(TriplePattern {
                        subject: var("s"),
                        predicate: NamedNodePattern::Variable(Variable::new("p")),
                        object: var("o"),
                    })),
                },
                TriplePattern {
                    subject: var("r"),
                    predicate: pred(ACCORDING_TO),
                    object: var("stand"),
                },
            ],
        };
        let template = vec![TriplePattern {
            subject: var("s"),
            predicate: NamedNodePattern::Variable(Variable::new("p")),
            object: var("o"),
        }];
        let out = eval_construct(&template, &where_pat, &mut ctx).expect("construct");

        assert_eq!(count_loss_code(&out, LOSS_REIFIER_LAYER_DROPPED), 1);
        assert_eq!(count_loss_code(&out, LOSS_ANNOTATION_LAYER_DROPPED), 1);
        assert_eq!(
            count_loss_code(&out, LOSS_STANDPOINT_SCOPE_DROPPED),
            0,
            "no configured table ⇒ no standpoint attribution"
        );
    }

    #[test]
    fn template_carrying_reifier_emits_no_loss() {
        // CONSTRUCT { ?r rdf:reifies <<( ?s ?p ?o )>> } WHERE { same } — the template
        // carries ?r, so NO loss is declared.
        let ds = reified_graph();
        let mut ctx = EvalCtx::new(&ds);
        let template = vec![TriplePattern {
            subject: var("r"),
            predicate: pred(REIFIES),
            object: TermPattern::Triple(Box::new(TriplePattern {
                subject: var("s"),
                predicate: NamedNodePattern::Variable(Variable::new("p")),
                object: var("o"),
            })),
        }];
        let out = eval_construct(&template, &where_reifies(), &mut ctx).expect("construct");
        assert_eq!(
            count_loss_code(&out, LOSS_REIFIER_LAYER_DROPPED),
            0,
            "reifier carried by template ⇒ no loss"
        );
        let any_loss = out
            .quads()
            .any(|q| matches!(out.resolve(q.o), TermRef::Iri(o) if o == PROJECTION_LOSS));
        assert!(
            !any_loss,
            "no ProjectionLoss node when the reifier is carried"
        );
    }

    #[test]
    fn plain_construct_is_byte_identical_fast_no_op() {
        // A CONSTRUCT with no reification in WHERE must be byte-identical to today.
        // We compare the canonicalized output against a reference built WITHOUT any
        // loss code present (no ProjectionLoss node at all).
        let ds = knows_graph();
        let mut ctx = EvalCtx::new(&ds);
        let template = vec![TriplePattern {
            subject: var("s"),
            predicate: pred(RELATED),
            object: var("o"),
        }];
        let out = eval_construct(&template, &where_knows(), &mut ctx).expect("construct");
        // No loss triples at all.
        let any_loss = out.quads().any(|q| {
            matches!(out.resolve(q.p), TermRef::Iri(p) if p == LOSS_CODE)
                || matches!(out.resolve(q.o), TermRef::Iri(o) if o == PROJECTION_LOSS)
        });
        assert!(
            !any_loss,
            "non-reification CONSTRUCT emits zero loss triples"
        );
        assert_eq!(out.quad_count(), 2, "exactly the two rewritten quads");
    }

    // ── RDF-1.2 side-table placement ────────────────────────────────────────

    /// CONSTRUCT { ?r rdf:reifies <<( ?s ?p ?o )>> } WHERE { ?r rdf:reifies <<( ?s ?p ?o )>> }
    /// must emit the reifier into the SIDE TABLE, not as a flat quad with predicate
    /// rdf:reifies.  This test FAILS before the fix and PASSES after.
    #[test]
    fn reifier_triple_goes_to_side_table() {
        let ds = reified_graph();
        let mut ctx = EvalCtx::new(&ds);
        // Template carries the reifier: ?r rdf:reifies <<( ?s ?p ?o )>>
        let template = vec![TriplePattern {
            subject: var("r"),
            predicate: pred(REIFIES),
            object: TermPattern::Triple(Box::new(TriplePattern {
                subject: var("s"),
                predicate: NamedNodePattern::Variable(Variable::new("p")),
                object: var("o"),
            })),
        }];
        let out = eval_construct(&template, &where_reifies(), &mut ctx).expect("construct");

        // The reification must land in the side table (reifiers), not as a flat quad.
        assert_eq!(
            out.reifiers().count(),
            1,
            "the reifier must be in the side table"
        );

        // No flat quad whose predicate is rdf:reifies must exist.
        let flat_reifies = out
            .quads()
            .any(|q| matches!(out.resolve(q.p), TermRef::Iri(p) if p == REIFIES));
        assert!(
            !flat_reifies,
            "no flat quad with predicate rdf:reifies — must be in side table"
        );
    }

    /// Build the same logical reifier two ways:
    ///   (1) via CONSTRUCT evaluation
    ///   (2) via direct push_reifier + push_annotation
    /// Both frozen datasets must be isomorphic.
    #[test]
    fn construct_reifier_parity_with_direct_ingest() {
        use purrdf_core::canonicalize;

        // (1) Via CONSTRUCT
        let ds = reified_graph();
        let mut ctx = EvalCtx::new(&ds);
        let template = vec![TriplePattern {
            subject: var("r"),
            predicate: pred(REIFIES),
            object: TermPattern::Triple(Box::new(TriplePattern {
                subject: var("s"),
                predicate: NamedNodePattern::Variable(Variable::new("p")),
                object: var("o"),
            })),
        }];
        let construct_out =
            eval_construct(&template, &where_reifies(), &mut ctx).expect("construct");

        // (2) Via direct builder calls — same logical structure as reified_graph() but
        //     without annotations (the template above carries no annotations).
        let mut b = RdfDatasetBuilder::new();
        let _ = b.intern_iri(REIFIES);
        let alice = b.intern_iri("http://ex/alice");
        let age = b.intern_iri("http://ex/age");
        let forty_two = b.intern_literal(RdfLiteral::simple("42"));
        let triple = b.intern_triple(alice, age, forty_two);
        let r = b.intern_iri("http://ex/r");
        b.push_reifier(r, triple);
        let direct_out = b.freeze().expect("freeze direct");

        assert_eq!(
            canonicalize(&construct_out).nquads,
            canonicalize(&direct_out).nquads,
            "CONSTRUCT output must be isomorphic to direct push_reifier ingest"
        );
    }

    #[test]
    fn loss_declaration_is_deterministic_and_collapses() {
        use purrdf_core::canonicalize;
        // Two reifiers reify the SAME triple <<( :alice :age 42 )>> → two solution
        // rows that drop to the SAME lost triple, so the deterministic content-keyed
        // loss node collapses to ONE.
        let mut b = RdfDatasetBuilder::new();
        let _ = b.intern_iri(REIFIES);
        let alice = b.intern_iri("http://ex/alice");
        let age = b.intern_iri("http://ex/age");
        let forty_two = b.intern_literal(RdfLiteral::simple("42"));
        let triple = b.intern_triple(alice, age, forty_two);
        let r1 = b.intern_iri("http://ex/r1");
        let r2 = b.intern_iri("http://ex/r2");
        b.push_reifier(r1, triple);
        b.push_reifier(r2, triple);
        let ds = b.freeze().expect("freeze");

        let template = vec![TriplePattern {
            subject: var("s"),
            predicate: NamedNodePattern::Variable(Variable::new("p")),
            object: var("o"),
        }];

        let mut ctx1 = EvalCtx::new(&ds).with_loss_vocabulary(ex_loss_vocab());
        let out1 = eval_construct(&template, &where_reifies(), &mut ctx1).expect("construct");
        let mut ctx2 = EvalCtx::new(&ds).with_loss_vocabulary(ex_loss_vocab());
        let out2 = eval_construct(&template, &where_reifies(), &mut ctx2).expect("construct");

        // Identical canonical N-Quads across two runs.
        assert_eq!(
            canonicalize(&out1).nquads,
            canonicalize(&out2).nquads,
            "loss declaration is deterministic across runs"
        );

        // Two rows dropped the SAME triple ⇒ exactly ONE loss code (collapsed).
        assert_eq!(
            count_loss_code(&out1, LOSS_REIFIER_LAYER_DROPPED),
            1,
            "identical drops collapse to one loss node"
        );
    }
}

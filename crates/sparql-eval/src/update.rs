// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! SPARQL 1.1 **UPDATE** evaluation over a [`MutableDataset`].
//!
//! [`eval_update`] applies a parsed [`Update`] to a copy-on-write
//! [`MutableDataset`] in request order — each operation observes the effects of the
//! earlier ones through the shared `m`. The engine seam ([`engine`](crate::engine))
//! drives this; the read query path is unchanged.
//!
//! # Doctrine / boundary decisions
//!
//! - **Implicit graph existence.** This is a quad store: a named graph *exists* iff
//!   it holds at least one quad. There is no empty-graph registry, so `CREATE GRAPH`
//!   is a no-op success and `CLEAR` ≡ `DROP` (both just remove every quad of the
//!   target — the only observable state a graph has). `CLEAR`/`DROP`/`CREATE` SILENT
//!   never errors here (there is no missing-graph condition to fail on).
//! - **Snapshot per WHERE op + value-space round-trip.** A `DELETE/INSERT … WHERE`
//!   evaluates its `WHERE` against a *frozen snapshot* of the current effective set
//!   (`m.freeze()`), because the evaluator reads a concrete [`RdfDataset`]. Each
//!   solution term is resolved to a dataset-independent [`TermValue`] (the template
//!   helpers do this), so the resulting quads stay valid after the snapshot is
//!   dropped and are applied back to `m` by value. DELETE is applied before INSERT
//!   (SPARQL §3.1.3), per solution row.
//! - **`WITH` / `USING` active dataset.** `WITH <g>` scopes the `WHERE` default graph
//!   to `g` (and is the default target for template quads). `USING` / `USING NAMED`
//!   build a custom `WHERE` active dataset (§3.1.3): when present they replace `WITH`'s
//!   effect on the `WHERE` (but `WITH` still targets the templates). A `USING` IRI that
//!   names no graph contributes nothing (never an error).
//! - **Blank nodes.** `INSERT DATA` blanks are minted fresh, ONE shared blank-map
//!   for the whole op (they co-refer within the op). `DELETE DATA` is blank-free (a
//!   parser invariant). Template (`DELETE`/`INSERT … WHERE`) blanks are minted fresh
//!   per solution row, exactly like `CONSTRUCT`. All of this mints from ONE
//!   monotonic counter threaded across the whole request ([`eval_update`]'s
//!   `bnode_counter`), never reset between operations, so a `_:b` label in one
//!   operation is a distinct blank from the same label in another operation
//!   (SPARQL 1.1 Update §4.1.1 / §19.6).
//! - **`LOAD` host seam.** The core is network-free. `LOAD <iri>` needs a host
//!   [`GraphResolver`] to fetch + parse the source into a frozen dataset; with no
//!   resolver, `LOAD` hard-fails unless `SILENT`.

use std::sync::Arc;

use purrdf_core::{
    DatasetMut, GraphMatchValue, MutableDataset, QuadValues, RdfDataset, RdfDiagnostic, TermValue,
};
use purrdf_sparql_algebra::{
    GraphTarget, GraphUpdateOperation, NamedNodePattern, QuadPattern, Update, UsingClause,
};

use crate::convert::named_node_to_value;
use crate::dataset_spec::ActiveDataset;
use crate::eval::{eval, BgpOrderCache, EvalCtx, StandpointPredicates};
use crate::solution::{Solution, VarSchema};
use crate::template::{
    instantiate_ground_term, instantiate_predicate, instantiate_term, positionally_ill_formed,
};
use crate::DetHashMap;

/// The engine-level WHERE-evaluation config threaded into UPDATE, mirroring the
/// query path's `EvalCtx` build (order cache + standpoint predicate table) so a
/// `DELETE/INSERT … WHERE` evaluates identically to a `SELECT`.
pub(crate) struct UpdateEvalConfig<'e> {
    pub(crate) standpoint_predicates: Option<&'e StandpointPredicates>,
    pub(crate) order_cache: &'e BgpOrderCache,
}

/// Host seam for SPARQL `LOAD <iri>`: resolves a source IRI to a frozen dataset.
///
/// The evaluator core is **network-free** (it builds clean for wasm and pulls no
/// HTTP/parse stack). A host that wants `LOAD` to dereference real documents injects
/// a resolver: it is responsible for fetching the IRI and parsing the response into
/// a frozen [`RdfDataset`]. Without a resolver, `LOAD` hard-fails (unless `SILENT`).
pub trait GraphResolver {
    /// Resolve `iri` to a frozen dataset, or a diagnostic on fetch/parse failure.
    fn resolve(&self, iri: &str) -> Result<Arc<RdfDataset>, RdfDiagnostic>;
}

/// Apply a parsed [`Update`] to `m` in request order.
///
/// Returns `Ok(())` on success; a specific [`RdfDiagnostic`] code on the boundary
/// conditions (`LOAD` with no resolver, a bad re-key destination, an internal eval
/// error). `resolver` supplies the `LOAD` host seam (see [`GraphResolver`]); pass
/// `None` to make any non-`SILENT` `LOAD` a hard error.
// The in-crate caller is the engine UPDATE seam (`engine::update`).
pub(crate) fn eval_update(
    update: &Update,
    m: &mut MutableDataset,
    resolver: Option<&dyn GraphResolver>,
    cfg: &UpdateEvalConfig<'_>,
) -> Result<(), RdfDiagnostic> {
    // A single monotonic counter threaded across EVERY operation in this request, so
    // a synthetic blank label minted by operation N can never collide with one minted
    // by operation N+1 — even though each operation's own `_:b` → label map starts
    // empty. Per SPARQL 1.1 Update §4.1.1 / §19.6, a blank-node label is scoped to the
    // single operation (and, inside an INSERT/DELETE … WHERE, freshly per solution
    // row): `_:b` in one operation is a DIFFERENT blank node from `_:b` in another
    // operation of the same request. Resetting the counter per-operation (the
    // previous behaviour) let two operations mint the same synthetic label, so their
    // blanks silently unified in the shared store.
    let mut bnode_counter: u64 = 0;
    for op in &update.operations {
        apply_operation(op, m, resolver, cfg, &mut bnode_counter)?;
    }
    Ok(())
}

/// Apply one update operation to `m`. `bnode_counter` is the request-wide monotonic
/// blank-mint counter (see [`eval_update`]) — never reset between operations.
fn apply_operation(
    op: &GraphUpdateOperation,
    m: &mut MutableDataset,
    resolver: Option<&dyn GraphResolver>,
    cfg: &UpdateEvalConfig<'_>,
    bnode_counter: &mut u64,
) -> Result<(), RdfDiagnostic> {
    match op {
        GraphUpdateOperation::InsertData { data } => insert_data(data, m, bnode_counter),
        GraphUpdateOperation::DeleteData { data } => delete_data(data, m, bnode_counter),
        GraphUpdateOperation::DeleteInsert {
            delete,
            insert,
            with,
            using,
            pattern,
        } => delete_insert(
            DeleteInsertSpec {
                delete,
                insert,
                with: with.as_ref(),
                using,
                pattern,
            },
            m,
            cfg,
            bnode_counter,
        ),
        GraphUpdateOperation::Load {
            silent,
            source,
            destination,
        } => load(*silent, source.as_str(), destination, m, resolver),
        // CLEAR ≡ DROP in a quad store with implicit graph existence (see module docs).
        GraphUpdateOperation::Clear { target, .. } | GraphUpdateOperation::Drop { target, .. } => {
            clear_target(target, m);
            Ok(())
        }
        // Graph existence is implicit, so CREATE has nothing to register: no-op success.
        GraphUpdateOperation::Create { .. } => Ok(()),
        GraphUpdateOperation::Add {
            source,
            destination,
            ..
        } => graph_op_add(source, destination, m),
        GraphUpdateOperation::Move {
            source,
            destination,
            ..
        } => graph_op_move(source, destination, m),
        GraphUpdateOperation::Copy {
            source,
            destination,
            ..
        } => graph_op_copy(source, destination, m),
    }
}

// ── INSERT DATA / DELETE DATA ────────────────────────────────────────────────

/// `INSERT DATA`: instantiate each quad (variable-free by parser invariant) with ONE
/// shared blank-map (blanks co-refer within the op) and insert the result.
///
/// DATA never queries the dataset, so it takes the snapshot-free ground path: no
/// `m.freeze()` (which would compact the whole base+delta for nothing) and no
/// `EvalCtx`. Blanks mint from the request-wide `counter` (see [`eval_update`]), so
/// this op's labels never collide with another operation's.
fn insert_data(
    data: &[QuadPattern],
    m: &mut MutableDataset,
    counter: &mut u64,
) -> Result<(), RdfDiagnostic> {
    let mut blanks: DetHashMap<String, String> = DetHashMap::default();
    for qp in data {
        if let Some(q) = instantiate_ground_quad(qp, &mut blanks, counter) {
            m.insert(q);
        }
    }
    Ok(())
}

/// `DELETE DATA`: instantiate each quad (variable-free AND blank-free — parser
/// guaranteed) and remove the result. Snapshot-free, like [`insert_data`].
fn delete_data(
    data: &[QuadPattern],
    m: &mut MutableDataset,
    counter: &mut u64,
) -> Result<(), RdfDiagnostic> {
    let mut blanks: DetHashMap<String, String> = DetHashMap::default();
    for qp in data {
        if let Some(q) = instantiate_ground_quad(qp, &mut blanks, counter) {
            m.remove(&q);
        }
    }
    Ok(())
}

// ── DELETE / INSERT … WHERE ──────────────────────────────────────────────────

/// The DELETE/INSERT/WITH/USING/WHERE fields of a `GraphUpdateOperation::DeleteInsert`,
/// bundled so [`delete_insert`] stays under clippy's argument-count ceiling.
#[derive(Clone, Copy)]
struct DeleteInsertSpec<'a> {
    delete: &'a [QuadPattern],
    insert: &'a [QuadPattern],
    with: Option<&'a purrdf_sparql_algebra::NamedNode>,
    using: &'a [UsingClause],
    pattern: &'a purrdf_sparql_algebra::GraphPattern,
}

/// `DELETE { ... } INSERT { ... } WHERE { ... }` and its shorthands.
fn delete_insert(
    spec: DeleteInsertSpec<'_>,
    m: &mut MutableDataset,
    cfg: &UpdateEvalConfig<'_>,
    bnode_counter: &mut u64,
) -> Result<(), RdfDiagnostic> {
    let DeleteInsertSpec {
        delete,
        insert,
        with,
        using,
        pattern,
    } = spec;
    // The WITH graph is the default target for delete/insert quads whose own
    // QuadPattern.graph is None (template target — independent of the WHERE dataset).
    let with_value = with.map(named_node_to_value);

    let snap = m.freeze()?;
    let mut ctx = EvalCtx::new(&snap);
    ctx = ctx.with_order_cache(cfg.order_cache);
    if let Some(preds) = cfg.standpoint_predicates {
        ctx = ctx.with_standpoint_predicates(preds.clone());
    }
    // Seed the WHERE/template context from the request-wide counter (never reset
    // between operations — see `eval_update`) and hand it back below, so blanks
    // minted by this operation (template blanks, `BNODE()`, `rdf:List` cells) stay
    // disjoint from every other operation's in the same request.
    ctx.bnode_counter = *bnode_counter;

    // Scope the WHERE active dataset (§3.1.3): USING (if present) builds a custom
    // dataset and replaces WITH's effect on the WHERE; otherwise WITH scopes the WHERE
    // default graph; otherwise the dataset under mutation. An absent USING/WITH graph
    // contributes nothing (matches nothing) — never an error.
    ctx.active_dataset = if !using.is_empty() {
        ActiveDataset::from_using(using, &snap)
    } else if let Some(g) = &with_value {
        ActiveDataset::with_default_graph(&snap, g)
    } else {
        ActiveDataset::store_default()
    };

    let seq = eval(pattern, &mut ctx)
        .map_err(|e| RdfDiagnostic::error("native-sparql-update-eval", e.to_string()))?;
    let schema = seq.schema.clone();

    // Collect the mutations BEFORE touching `m`, so the snapshot stays valid for the
    // value resolution. DELETE before INSERT per row (SPARQL §3.1.3).
    let mut to_remove = Vec::new();
    let mut to_insert = Vec::new();
    // Blank-label maps are reset PER ROW (template blanks co-refer within a row, are
    // distinct across rows) but the allocation is hoisted: `.clear()` reuses the
    // capacity instead of allocating a fresh map for every solution.
    let mut del_blanks: DetHashMap<String, String> = DetHashMap::default();
    let mut ins_blanks: DetHashMap<String, String> = DetHashMap::default();
    for row in &seq.rows {
        del_blanks.clear();
        for qp in delete {
            if let Some(q) = instantiate_quad_with_default(
                qp,
                row,
                &schema,
                &mut del_blanks,
                &mut ctx,
                with_value.as_ref(),
            ) {
                to_remove.push(q);
            }
        }
        ins_blanks.clear();
        for qp in insert {
            if let Some(q) = instantiate_quad_with_default(
                qp,
                row,
                &schema,
                &mut ins_blanks,
                &mut ctx,
                with_value.as_ref(),
            ) {
                to_insert.push(q);
            }
        }
    }
    *bnode_counter = ctx.bnode_counter;
    drop(ctx);
    drop(snap);

    for q in &to_remove {
        m.remove(q);
    }
    for q in to_insert {
        m.insert(q);
    }
    Ok(())
}

// ── LOAD ─────────────────────────────────────────────────────────────────────

/// `LOAD [SILENT] <iri> [INTO GRAPH <iri>]`.
fn load(
    silent: bool,
    source: &str,
    destination: &GraphTarget,
    m: &mut MutableDataset,
    resolver: Option<&dyn GraphResolver>,
) -> Result<(), RdfDiagnostic> {
    let Some(resolver) = resolver else {
        if silent {
            return Ok(());
        }
        return Err(RdfDiagnostic::error(
            "native-sparql-load-no-resolver",
            format!("LOAD <{source}> needs a GraphResolver host seam, none was provided"),
        ));
    };
    let loaded = match resolver.resolve(source) {
        Ok(ds) => ds,
        Err(e) => {
            if silent {
                return Ok(());
            }
            return Err(e);
        }
    };

    // Re-key each loaded quad's graph to the destination (Default → None,
    // Named(g) → Some(g)). Enumerate the loaded dataset in value space.
    let dest = graph_target_value(destination)?;
    let view = MutableDataset::new(loaded);
    let quads = view.quads_for_pattern(None, None, None, GraphMatchValue::Any);
    for q in quads {
        m.insert(rekey_graph(q, dest.as_ref()));
    }
    Ok(())
}

// ── CLEAR / DROP ─────────────────────────────────────────────────────────────

/// Remove every quad of `target` from `m` (CLEAR ≡ DROP — see module docs).
fn clear_target(target: &GraphTarget, m: &mut MutableDataset) {
    let quads = quads_of_target(target, m);
    for q in &quads {
        m.remove(q);
    }
}

// ── ADD / MOVE / COPY ────────────────────────────────────────────────────────

/// `ADD <source> TO <dest>`: insert source quads re-keyed to dest; dest is NOT
/// cleared and source is NOT removed.
fn graph_op_add(
    source: &GraphTarget,
    destination: &GraphTarget,
    m: &mut MutableDataset,
) -> Result<(), RdfDiagnostic> {
    // SPARQL §3.2.5: ADD where source ≡ destination is a no-op.
    if source == destination {
        return Ok(());
    }
    let src = quads_of_target(source, m);
    let dest = graph_target_value(destination)?;
    for q in src {
        m.insert(rekey_graph(q, dest.as_ref()));
    }
    Ok(())
}

/// `COPY <source> TO <dest>`: clear dest, then insert source quads re-keyed to dest.
fn graph_op_copy(
    source: &GraphTarget,
    destination: &GraphTarget,
    m: &mut MutableDataset,
) -> Result<(), RdfDiagnostic> {
    // SPARQL §3.2.4: COPY where source ≡ destination is a no-op.
    if source == destination {
        return Ok(());
    }
    let dest = graph_target_value(destination)?;
    let src = quads_of_target(source, m);
    clear_target(destination, m);
    for q in src {
        m.insert(rekey_graph(q, dest.as_ref()));
    }
    Ok(())
}

/// `MOVE <source> TO <dest>`: clear dest, insert source quads re-keyed to dest, then
/// remove the source quads.
fn graph_op_move(
    source: &GraphTarget,
    destination: &GraphTarget,
    m: &mut MutableDataset,
) -> Result<(), RdfDiagnostic> {
    // SPARQL §3.2.6: MOVE where source ≡ destination is a no-op. This guard is also
    // a correctness requirement, not just an optimization: with source == dest the
    // trailing source-removal below would re-suppress the just-inserted quads and
    // empty the graph.
    if source == destination {
        return Ok(());
    }
    let dest = graph_target_value(destination)?;
    let src = quads_of_target(source, m);
    clear_target(destination, m);
    for q in &src {
        m.insert(rekey_graph(q.clone(), dest.as_ref()));
    }
    for q in &src {
        m.remove(q);
    }
    Ok(())
}

// ── shared helpers ───────────────────────────────────────────────────────────

/// Instantiate a **variable-free** `DATA` quad (`INSERT DATA` / `DELETE DATA`) into a
/// concrete [`QuadValues`] with no dataset/snapshot. `None` if the triple is
/// positionally ill-formed (§16.2) or — a parser-invariant guard — any position holds
/// a variable. The graph slot is the explicit `GRAPH g { … }` wrapper, else the
/// default graph (DATA has no `WITH`). Blanks mint from the shared `counter`.
fn instantiate_ground_quad(
    qp: &QuadPattern,
    blanks: &mut DetHashMap<String, String>,
    counter: &mut u64,
) -> Option<QuadValues> {
    let s = instantiate_ground_term(&qp.triple.subject, blanks, counter)?;
    let p = match &qp.triple.predicate {
        NamedNodePattern::NamedNode(n) => named_node_to_value(n),
        NamedNodePattern::Variable(_) => return None,
    };
    let o = instantiate_ground_term(&qp.triple.object, blanks, counter)?;
    if positionally_ill_formed(&s, &p) {
        return None;
    }
    let g = match &qp.graph {
        Some(NamedNodePattern::NamedNode(n)) => Some(named_node_to_value(n)),
        Some(NamedNodePattern::Variable(_)) => return None,
        None => None,
    };
    Some(QuadValues { s, p, o, g })
}

/// Instantiate one solution-driven `QuadPattern` (subject/pred/object + optional
/// graph) into a concrete [`QuadValues`], with a `default_graph` (the WITH graph) used
/// when the pattern's own graph slot is `None`. `None` if any position holds an unbound
/// variable, or the result is positionally ill-formed (literal subject / non-IRI
/// predicate), or the graph slot is a variable bound to a non-IRI.
fn instantiate_quad_with_default(
    qp: &QuadPattern,
    row: &Solution,
    schema: &VarSchema,
    blanks: &mut DetHashMap<String, String>,
    ctx: &mut EvalCtx<'_>,
    default_graph: Option<&TermValue>,
) -> Option<QuadValues> {
    let s = instantiate_term(&qp.triple.subject, row, schema, blanks, ctx)?;
    let p = instantiate_predicate(&qp.triple.predicate, row, schema, ctx)?;
    let o = instantiate_term(&qp.triple.object, row, schema, blanks, ctx)?;

    // Positional validity (§16.2 / template rules): a literal subject or a non-IRI
    // predicate is ill-formed → skip the quad (do not error).
    if positionally_ill_formed(&s, &p) {
        return None;
    }

    // Graph slot: explicit pattern graph → else the WITH default → else None.
    let g = match &qp.graph {
        Some(NamedNodePattern::NamedNode(n)) => Some(named_node_to_value(n)),
        Some(NamedNodePattern::Variable(v)) => {
            let term = schema.index_of(v).and_then(|c| row[c])?;
            let value = ctx.scratch.value_of(ctx.dataset, term);
            // A graph name must be an IRI; a non-IRI binding makes the quad
            // ill-formed → skip.
            if !matches!(value, TermValue::Iri(_)) {
                return None;
            }
            Some(value)
        }
        None => default_graph.cloned(),
    };

    Some(QuadValues { s, p, o, g })
}

/// Re-key a quad's graph slot to `dest` (`None` = default graph).
fn rekey_graph(q: QuadValues, dest: Option<&TermValue>) -> QuadValues {
    QuadValues {
        s: q.s,
        p: q.p,
        o: q.o,
        g: dest.cloned(),
    }
}

/// The destination graph VALUE of a graph target (`Default` → `None`, `Named` → the
/// IRI value). Only `Default`/`Named` are valid as a re-key destination (LOAD's
/// destination and ADD/MOVE/COPY operands are `GraphOrDefault`); a `NamedGraphs`/`All`
/// target is meaningless as a single destination — the parser never produces it in
/// these positions, so reaching it is a hard error (no silent coercion to default).
fn graph_target_value(target: &GraphTarget) -> Result<Option<TermValue>, RdfDiagnostic> {
    match target {
        GraphTarget::Default => Ok(None),
        GraphTarget::Named(n) => Ok(Some(named_node_to_value(n))),
        GraphTarget::NamedGraphs | GraphTarget::All => Err(RdfDiagnostic::error(
            "native-sparql-update-bad-destination",
            "an ADD/MOVE/COPY/LOAD destination must be DEFAULT or a single named GRAPH, \
             not NAMED or ALL",
        )),
    }
}

/// Every effective quad of a graph target, as owned value-quads.
///
/// `Default` → the default graph; `Named(g)` → that one named graph; `NamedGraphs`
/// → all named graphs (every quad whose graph slot is `Some`); `All` → every quad.
fn quads_of_target(target: &GraphTarget, m: &MutableDataset) -> Vec<QuadValues> {
    match target {
        GraphTarget::Default => m.quads_for_pattern(None, None, None, GraphMatchValue::Default),
        GraphTarget::Named(n) => {
            let g = named_node_to_value(n);
            m.quads_for_pattern(None, None, None, GraphMatchValue::Named(&g))
        }
        GraphTarget::All => m.quads_for_pattern(None, None, None, GraphMatchValue::Any),
        GraphTarget::NamedGraphs => m
            .quads_for_pattern(None, None, None, GraphMatchValue::Any)
            .into_iter()
            .filter(|q| q.g.is_some())
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use purrdf_core::{RdfDatasetBuilder, RdfLiteral};
    use purrdf_sparql_algebra::SparqlParser;

    const EX: &str = "http://ex/";

    fn iri(local: &str) -> TermValue {
        TermValue::Iri(format!("{EX}{local}"))
    }

    fn parse(text: &str) -> Update {
        SparqlParser::new()
            .parse_update(&format!("PREFIX ex: <{EX}>\n{text}"))
            .expect("update parses")
    }

    /// A fresh mutable dataset over the given default-graph (s,p,o) IRI triples.
    fn mut_with(triples: &[(&str, &str, &str)]) -> MutableDataset {
        let mut b = RdfDatasetBuilder::new();
        for (s, p, o) in triples {
            let s = b.intern_iri(&format!("{EX}{s}"));
            let p = b.intern_iri(&format!("{EX}{p}"));
            let o = b.intern_iri(&format!("{EX}{o}"));
            b.push_quad(s, p, o, None);
        }
        MutableDataset::new(b.freeze().expect("freeze base"))
    }

    /// The effective quads as a comparable set of value tuples.
    fn quad_set(m: &MutableDataset) -> std::collections::BTreeSet<String> {
        m.quads_for_pattern(None, None, None, GraphMatchValue::Any)
            .iter()
            .map(|q| format!("{:?}|{:?}|{:?}|{:?}", q.s, q.p, q.o, q.g))
            .collect()
    }

    fn run(text: &str, m: &mut MutableDataset) {
        let cache = BgpOrderCache::default();
        let cfg = UpdateEvalConfig {
            standpoint_predicates: None,
            order_cache: &cache,
        };
        eval_update(&parse(text), m, None, &cfg).expect("update applies");
    }

    #[test]
    fn insert_data_adds_quad() {
        let mut m = mut_with(&[]);
        run("INSERT DATA { ex:a ex:p ex:b }", &mut m);
        let frozen = m.freeze().expect("freeze");
        assert_eq!(frozen.quad_count(), 1);
        assert!(frozen.term_id_by_value(&iri("a")).is_some());
    }

    #[test]
    fn insert_data_blank_node_mints_a_blank() {
        let mut m = mut_with(&[]);
        run("INSERT DATA { _:x ex:p ex:b . _:x ex:q ex:c }", &mut m);
        let frozen = m.freeze().expect("freeze");
        // Two quads, sharing ONE minted blank subject (co-reference within the op).
        assert_eq!(frozen.quad_count(), 2);
        let mut blanks = std::collections::BTreeSet::new();
        for q in frozen.quads() {
            if let purrdf_core::TermRef::Blank { label, .. } = frozen.resolve(q.s) {
                blanks.insert(label.to_owned());
            }
        }
        assert_eq!(blanks.len(), 1, "the two quads share one minted blank");
    }

    #[test]
    fn delete_data_removes_quad() {
        let mut m = mut_with(&[("a", "p", "b"), ("a", "p", "c")]);
        run("DELETE DATA { ex:a ex:p ex:b }", &mut m);
        let set = quad_set(&m);
        assert_eq!(set.len(), 1);
        assert!(!m.contains(&QuadValues::triple(iri("a"), iri("p"), iri("b"))));
        assert!(m.contains(&QuadValues::triple(iri("a"), iri("p"), iri("c"))));
    }

    #[test]
    fn delete_where_removes_matches() {
        let mut m = mut_with(&[("a", "p", "b"), ("a", "p", "c"), ("a", "q", "d")]);
        run("DELETE WHERE { ?s ex:p ?o }", &mut m);
        // Only the two ex:p quads go; the ex:q quad survives.
        assert_eq!(quad_set(&m).len(), 1);
        assert!(m.contains(&QuadValues::triple(iri("a"), iri("q"), iri("d"))));
    }

    #[test]
    fn delete_insert_modify_round_trips_a_where_bound_value() {
        // The inserted quad's OBJECT is a WHERE-bound value (?o). It must survive the
        // snapshot → mutable round-trip (value-space resolution).
        let mut m = mut_with(&[("a", "p", "b")]);
        run(
            "DELETE { ?s ex:p ?o } INSERT { ?s ex:q ?o } WHERE { ?s ex:p ?o }",
            &mut m,
        );
        // (a,p,b) gone, (a,q,b) present — and b is the WHERE-bound object value.
        assert!(!m.contains(&QuadValues::triple(iri("a"), iri("p"), iri("b"))));
        assert!(m.contains(&QuadValues::triple(iri("a"), iri("q"), iri("b"))));
    }

    #[test]
    fn insert_only_modify_keeps_source() {
        let mut m = mut_with(&[("a", "p", "b")]);
        run("INSERT { ?s ex:q ?o } WHERE { ?s ex:p ?o }", &mut m);
        assert!(m.contains(&QuadValues::triple(iri("a"), iri("p"), iri("b"))));
        assert!(m.contains(&QuadValues::triple(iri("a"), iri("q"), iri("b"))));
    }

    #[test]
    fn clear_default_empties_default_graph() {
        let mut m = mut_with(&[("a", "p", "b"), ("a", "p", "c")]);
        run("CLEAR DEFAULT", &mut m);
        assert!(quad_set(&m).is_empty());
    }

    #[test]
    fn drop_named_graph_removes_its_quads() {
        // A base with a default-graph quad and a named-graph quad.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri(&format!("{EX}a"));
        let p = b.intern_iri(&format!("{EX}p"));
        let o = b.intern_iri(&format!("{EX}b"));
        let g = b.intern_iri(&format!("{EX}g"));
        b.push_quad(s, p, o, None);
        b.push_quad(s, p, o, Some(g));
        let mut m = MutableDataset::new(b.freeze().expect("freeze"));

        run("DROP GRAPH ex:g", &mut m);
        // The named-graph quad is gone; the default-graph quad survives.
        let remaining = m.quads_for_pattern(None, None, None, GraphMatchValue::Any);
        assert_eq!(remaining.len(), 1);
        assert!(remaining[0].g.is_none());
    }

    #[test]
    fn create_graph_is_a_noop() {
        let mut m = mut_with(&[("a", "p", "b")]);
        run("CREATE GRAPH ex:g", &mut m);
        // No change — graph existence is implicit.
        assert_eq!(quad_set(&m).len(), 1);
    }

    #[test]
    fn add_copies_source_into_dest_keeping_both() {
        // ADD default TO GRAPH ex:g — default-graph quads are copied into ex:g and
        // the default graph is untouched.
        let mut m = mut_with(&[("a", "p", "b")]);
        run("ADD DEFAULT TO GRAPH ex:g", &mut m);
        let all = m.quads_for_pattern(None, None, None, GraphMatchValue::Any);
        assert_eq!(all.len(), 2, "default kept + named copy added");
        assert_eq!(all.iter().filter(|q| q.g.is_none()).count(), 1);
        assert_eq!(all.iter().filter(|q| q.g == Some(iri("g"))).count(), 1);
    }

    #[test]
    fn move_clears_source_after_copy() {
        let mut m = mut_with(&[("a", "p", "b")]);
        run("MOVE DEFAULT TO GRAPH ex:g", &mut m);
        let all = m.quads_for_pattern(None, None, None, GraphMatchValue::Any);
        assert_eq!(all.len(), 1, "source emptied, dest has the one quad");
        assert_eq!(all[0].g, Some(iri("g")));
    }

    #[test]
    fn copy_replaces_dest_then_fills_it() {
        // Seed ex:g with a stale quad; COPY default → ex:g must clear ex:g first.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri(&format!("{EX}a"));
        let p = b.intern_iri(&format!("{EX}p"));
        let o = b.intern_iri(&format!("{EX}b"));
        let stale = b.intern_iri(&format!("{EX}stale"));
        let g = b.intern_iri(&format!("{EX}g"));
        b.push_quad(s, p, o, None); // default (a,p,b)
        b.push_quad(stale, p, o, Some(g)); // ex:g (stale,p,b)
        let mut m = MutableDataset::new(b.freeze().expect("freeze"));

        run("COPY DEFAULT TO GRAPH ex:g", &mut m);
        let in_g: Vec<_> = m
            .quads_for_pattern(None, None, None, GraphMatchValue::Named(&iri("g")))
            .into_iter()
            .collect();
        assert_eq!(in_g.len(), 1, "dest cleared then filled from source");
        assert_eq!(in_g[0].s, iri("a"), "stale quad gone, source quad present");
    }

    #[test]
    fn move_self_to_self_preserves_the_graph() {
        // MOVE GRAPH ex:g TO GRAPH ex:g is a no-op (SPARQL §3.2.6). Without the
        // same-graph guard the suppression-delta double-remove would empty ex:g.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri(&format!("{EX}a"));
        let p = b.intern_iri(&format!("{EX}p"));
        let o = b.intern_iri(&format!("{EX}b"));
        let g = b.intern_iri(&format!("{EX}g"));
        b.push_quad(s, p, o, Some(g));
        let mut m = MutableDataset::new(b.freeze().expect("freeze"));

        run("MOVE GRAPH ex:g TO GRAPH ex:g", &mut m);
        let in_g = m.quads_for_pattern(None, None, None, GraphMatchValue::Named(&iri("g")));
        assert_eq!(in_g.len(), 1, "self-MOVE preserves the graph's quad");
        assert_eq!(in_g[0].s, iri("a"));

        // The same guard makes self-COPY and self-ADD no-ops too.
        run("COPY GRAPH ex:g TO GRAPH ex:g", &mut m);
        run("ADD GRAPH ex:g TO GRAPH ex:g", &mut m);
        let still = m.quads_for_pattern(None, None, None, GraphMatchValue::Named(&iri("g")));
        assert_eq!(still.len(), 1, "self COPY/ADD leave the graph unchanged");
    }

    #[test]
    fn graph_op_to_named_or_all_destination_is_a_hard_error() {
        // The parser only ever produces DEFAULT/GRAPH destinations, but if a NAMED/ALL
        // destination ever reaches a single-graph re-key it is a hard error, not a
        // silent coercion to the default graph.
        let mut m = mut_with(&[("a", "p", "b")]);
        let upd = Update {
            operations: vec![GraphUpdateOperation::Move {
                silent: false,
                source: GraphTarget::Default,
                destination: GraphTarget::All,
            }],
            base_iri: None,
        };
        let cache = BgpOrderCache::default();
        let cfg = UpdateEvalConfig {
            standpoint_predicates: None,
            order_cache: &cache,
        };
        let err = eval_update(&upd, &mut m, None, &cfg).unwrap_err();
        assert_eq!(err.code, "native-sparql-update-bad-destination");
        // The base is untouched (the error aborts before any mutation lands here, and
        // the engine seam's branch/freeze guarantees atomicity at the request level).
        assert_eq!(quad_set(&m).len(), 1);
    }

    // ── LOAD ─────────────────────────────────────────────────────────────────

    struct TestResolver {
        ds: Arc<RdfDataset>,
    }
    impl GraphResolver for TestResolver {
        fn resolve(&self, _iri: &str) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
            Ok(self.ds.clone())
        }
    }

    fn loadable() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri(&format!("{EX}loaded"));
        let p = b.intern_iri(&format!("{EX}p"));
        let o = b.intern_literal(RdfLiteral::simple("v"));
        b.push_quad(s, p, o, None);
        b.freeze().expect("freeze loadable")
    }

    #[test]
    fn load_with_resolver_imports_into_default_graph() {
        let mut m = mut_with(&[]);
        let resolver = TestResolver { ds: loadable() };
        let cache = BgpOrderCache::default();
        let cfg = UpdateEvalConfig {
            standpoint_predicates: None,
            order_cache: &cache,
        };
        eval_update(&parse("LOAD ex:doc"), &mut m, Some(&resolver), &cfg).expect("load");
        let frozen = m.freeze().expect("freeze");
        assert_eq!(frozen.quad_count(), 1);
        assert!(frozen.term_id_by_value(&iri("loaded")).is_some());
    }

    #[test]
    fn load_into_named_graph_rekeys_to_destination() {
        let mut m = mut_with(&[]);
        let resolver = TestResolver { ds: loadable() };
        let cache = BgpOrderCache::default();
        let cfg = UpdateEvalConfig {
            standpoint_predicates: None,
            order_cache: &cache,
        };
        eval_update(
            &parse("LOAD ex:doc INTO GRAPH ex:g"),
            &mut m,
            Some(&resolver),
            &cfg,
        )
        .expect("load into");
        let all = m.quads_for_pattern(None, None, None, GraphMatchValue::Any);
        assert_eq!(all.len(), 1);
        assert_eq!(
            all[0].g,
            Some(iri("g")),
            "re-keyed to the destination graph"
        );
    }

    #[test]
    fn load_without_resolver_is_a_hard_error() {
        let mut m = mut_with(&[]);
        let cache = BgpOrderCache::default();
        let cfg = UpdateEvalConfig {
            standpoint_predicates: None,
            order_cache: &cache,
        };
        let err = eval_update(&parse("LOAD ex:doc"), &mut m, None, &cfg).unwrap_err();
        assert_eq!(err.code, "native-sparql-load-no-resolver");
    }

    #[test]
    fn load_silent_without_resolver_is_a_noop_ok() {
        let mut m = mut_with(&[("a", "p", "b")]);
        let cache = BgpOrderCache::default();
        let cfg = UpdateEvalConfig {
            standpoint_predicates: None,
            order_cache: &cache,
        };
        eval_update(&parse("LOAD SILENT ex:doc"), &mut m, None, &cfg).expect("silent load no-ops");
        assert_eq!(quad_set(&m).len(), 1, "unchanged");
    }

    // ── USING ─────────────────────────────────────────────────────────────────

    /// A base with the same (a,p,b) triple in the default graph and in ex:g, plus a
    /// decoy (a,p,c) only in ex:g.
    fn base_default_and_named() -> MutableDataset {
        let mut b = RdfDatasetBuilder::new();
        let a = b.intern_iri(&format!("{EX}a"));
        let p = b.intern_iri(&format!("{EX}p"));
        let bb = b.intern_iri(&format!("{EX}b"));
        let cc = b.intern_iri(&format!("{EX}c"));
        let g = b.intern_iri(&format!("{EX}g"));
        b.push_quad(a, p, bb, None); // default (a,p,b)
        b.push_quad(a, p, bb, Some(g)); // ex:g (a,p,b)
        b.push_quad(a, p, cc, Some(g)); // ex:g (a,p,c)
        MutableDataset::new(b.freeze().expect("freeze"))
    }

    #[test]
    fn using_scopes_where_to_the_named_graph() {
        // USING ex:g folds ex:g into the WHERE default graph: the DELETE template
        // (default-graph target) removes whatever the WHERE bound from ex:g. The WHERE
        // sees ex:g's (a,p,b)+(a,p,c), so the default-graph (a,p,b) is deleted but the
        // default graph's other triples (none here) and ex:g itself are not the target.
        let mut m = base_default_and_named();
        // DELETE the default-graph quad whose object the WHERE bound from ex:g.
        run(
            "DELETE { ex:a ex:p ?o } USING ex:g WHERE { ex:a ex:p ?o }",
            &mut m,
        );
        // The WHERE matched ?o ∈ {b, c} in ex:g; the DELETE removed (a,p,b) and (a,p,c)
        // from the DEFAULT graph. Default had only (a,p,b) → gone; (a,p,c) wasn't there.
        assert!(!m.contains(&QuadValues::triple(iri("a"), iri("p"), iri("b"))));
        // ex:g is untouched (USING only scopes the WHERE, not the delete target).
        let in_g = m.quads_for_pattern(None, None, None, GraphMatchValue::Named(&iri("g")));
        assert_eq!(
            in_g.len(),
            2,
            "ex:g is the WHERE source, not the delete target"
        );
    }

    #[test]
    fn using_named_restricts_graph_var_in_where() {
        // USING NAMED ex:g makes ex:g (and only ex:g) addressable by GRAPH ?g in the
        // WHERE; the default graph of the WHERE is empty (no plain USING).
        let mut m = base_default_and_named();
        run(
            "INSERT { ex:hit ex:in ?g } USING NAMED ex:g WHERE { GRAPH ?g { ex:a ex:p ex:b } }",
            &mut m,
        );
        // ?g bound to ex:g (the only named graph in the USING NAMED set) → one insert.
        assert!(m.contains(&QuadValues::triple(iri("hit"), iri("in"), iri("g"))));
    }

    #[test]
    fn using_nonexistent_graph_matches_nothing() {
        // USING <absent> → the WHERE default graph is empty → no solutions → no-op,
        // not an error.
        let mut m = mut_with(&[("a", "p", "b")]);
        run(
            "DELETE { ex:a ex:p ?o } USING ex:absent WHERE { ex:a ex:p ?o }",
            &mut m,
        );
        // Nothing matched in the empty WHERE dataset → the base is unchanged.
        assert!(m.contains(&QuadValues::triple(iri("a"), iri("p"), iri("b"))));
    }

    #[test]
    fn with_scopes_where_and_targets_the_named_graph() {
        // WITH ex:g: the WHERE matches in ex:g, and the delete/insert quads (no
        // explicit graph) target ex:g too. Seed a quad in ex:g and a decoy in the
        // default graph with the same s/p/o; only the ex:g one is rewritten.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri(&format!("{EX}a"));
        let p = b.intern_iri(&format!("{EX}p"));
        let o = b.intern_iri(&format!("{EX}b"));
        let g = b.intern_iri(&format!("{EX}g"));
        b.push_quad(s, p, o, None); // default-graph decoy (a,p,b)
        b.push_quad(s, p, o, Some(g)); // ex:g (a,p,b)
        let mut m = MutableDataset::new(b.freeze().expect("freeze"));

        run(
            "WITH ex:g DELETE { ?s ex:p ?o } INSERT { ?s ex:q ?o } WHERE { ?s ex:p ?o }",
            &mut m,
        );

        // The default-graph decoy is untouched (WITH scoped the WHERE to ex:g).
        assert!(m.contains(&QuadValues::triple(iri("a"), iri("p"), iri("b"))));
        // In ex:g: (a,p,b) gone, (a,q,b) present, both keyed to ex:g (the WITH graph).
        assert!(m.contains(&QuadValues::quad(iri("a"), iri("q"), iri("b"), iri("g"))));
        let in_g = m.quads_for_pattern(None, None, None, GraphMatchValue::Named(&iri("g")));
        assert_eq!(in_g.len(), 1);
        assert_eq!(in_g[0].p, iri("q"));
    }

    #[test]
    fn later_operation_sees_earlier_effect() {
        // INSERT then DELETE WHERE in one request: the DELETE must see the inserted
        // quad (operations apply in order over the shared `m`).
        let mut m = mut_with(&[]);
        run(
            "INSERT DATA { ex:a ex:p ex:b } ; DELETE WHERE { ?s ex:p ?o }",
            &mut m,
        );
        assert!(
            quad_set(&m).is_empty(),
            "the second op saw the first's insert"
        );
    }
}

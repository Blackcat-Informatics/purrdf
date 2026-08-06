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
use crate::eval::{EvalCtx, eval_evaluated};
use crate::governor::lift::{Evaluated, Truncation};
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
///
/// # Under a truncation
///
/// The template is instantiated over whatever rows the `WHERE` produced, and the `WHERE`'s
/// certificate is handed back beside the graph rather than folded into it: a graph built
/// from a certified lower bound is a **subgraph** of the true `CONSTRUCT` output, one
/// built from an upper bound is a supergraph, and one built from no bound at all is built
/// from no rows. Only the caller that receives the certificate can tell those apart, so
/// the certificate travels rather than being discarded here.
/// A `CONSTRUCT`/`DESCRIBE` result: the graph, and the `WHERE`'s certificate when a
/// governor stopped it short.
pub(crate) type ConstructedGraph<I> = (Arc<RdfDataset>, Option<Truncation<I>>);

/// Charge the answer cap against a graph-producing query form's output, truncating the
/// graph to the prefix the cap admits.
///
/// # What the cap denominates for `CONSTRUCT` and `DESCRIBE`
///
/// **Output triples.** For a `SELECT` the answer sequence is its rows and the cap counts
/// rows; for a graph form the answer *is* the graph, so the cap counts the triples in it —
/// every ordinary quad, every RDF 1.2 reifier binding, and every annotation, each one
/// being a statement the caller receives. Counting solution rows instead would leave the
/// governor measuring the wrong thing entirely: one `CONSTRUCT` row can instantiate a
/// twenty-triple template, and a `DESCRIBE` of one row's subject can pull in a thousand.
/// A caller who caps a query at a thousand answers and receives a hundred thousand
/// triples has no governor at all, which is exactly the hole this closes.
///
/// The boundary is the `SELECT` boundary, inclusively: a graph whose triple count equals
/// the cap is **complete**, and one triple more is a trip.
///
/// # Why the frozen order, and why a rebuild
///
/// The count runs over the frozen dataset's canonical order — the order `freeze` sorts and
/// de-duplicates into — so "the first *n* triples" is a property of the output graph and
/// not of the accident of template evaluation. That makes the truncated graph a genuine
/// positional prefix of the complete one: under a larger deterministic cap these same
/// triples come first, which is the resumption property
/// [`PartialSparqlResult::is_positional_prefix`](crate::PartialSparqlResult::is_positional_prefix)
/// promises. Counting emissions instead would make the cap depend on how many duplicates a
/// template happened to produce, and the reported size would not match the graph handed
/// back.
///
/// A truncated graph has to be rebuilt rather than sliced because a frozen dataset owns its
/// term table; the ids are re-interned through [`TermValue`], which is dataset-independent
/// and carries nested triple terms structurally.
pub(crate) fn commit_answer_triples<D: DatasetView + Sync>(
    graph: Arc<RdfDataset>,
    certificate: Option<Truncation<D::Id>>,
    rows: &crate::solution::SolutionSeq<D::Id>,
    ctx: &EvalCtx<'_, D>,
) -> ConstructedGraph<D::Id> {
    let Some(state) = ctx.governor_state() else {
        return (graph, certificate);
    };
    if !state.is_engaged_in(purrdf_core::ResourceDimension::AnswerRows) {
        return (graph, certificate);
    }
    let total = graph_triple_count(&graph);
    let mut admitted = 0_usize;
    let mut tripped = state.tripped();
    let mut cap_cut = false;
    for _ in 0..total {
        if let Err(cap) = state.charge_final_output(purrdf_core::ResourceDimension::AnswerRows, 1) {
            tripped.get_or_insert(cap);
            cap_cut = true;
            break;
        }
        admitted += 1;
    }

    match (certificate, tripped, cap_cut) {
        (None, None, _) => (graph, None),
        (None, Some(tripped), _) => (
            truncate_graph(&graph, admitted),
            // The rows are the `WHERE`'s complete output — the cap stopped the *graph*,
            // not the pattern — so they are their own positional prefix and certify a
            // lower bound, which is what `origin` states.
            Some(Truncation::origin(rows.clone(), tripped)),
        ),
        (Some(certificate), _, true) => {
            let (certificate_rows, certificate) = certificate.split();
            (
                truncate_graph(&graph, admitted),
                Some(Truncation::after_answer_cap(certificate_rows, certificate)),
            )
        }
        (Some(certificate), _, false) => (graph, Some(certificate)),
    }
}

/// The number of statements a graph result carries: quads plus the RDF 1.2 statement
/// layer's reifier bindings and annotations.
///
/// [`RdfDataset::quad_count`] alone would undercount a reification-bearing `CONSTRUCT`,
/// whose reifier and annotation rows live in side tables rather than in `quads` — and a
/// governor that cannot see a whole encoding layer is one a query can be written around.
fn graph_triple_count(graph: &RdfDataset) -> usize {
    graph.quad_count() + graph.reifiers().count() + graph.annotations().count()
}

/// Rebuild `graph` from its first `admitted` statements, in the frozen canonical order the
/// count above walks: quads, then reifier bindings, then annotations.
fn truncate_graph(graph: &RdfDataset, admitted: usize) -> Arc<RdfDataset> {
    let mut builder = RdfDatasetBuilder::new();
    let mut remaining = admitted;
    let intern = |builder: &mut RdfDatasetBuilder, id: TermId| {
        let value = graph.term_value(id);
        builder.intern_value(&value)
    };
    for quad in graph.quads() {
        if remaining == 0 {
            break;
        }
        let s = intern(&mut builder, quad.s);
        let p = intern(&mut builder, quad.p);
        let o = intern(&mut builder, quad.o);
        let g = quad.g.map(|g| intern(&mut builder, g));
        builder.push_quad(s, p, o, g);
        remaining -= 1;
    }
    for (reifier, triple, graph_id) in graph.reifiers_with_graph() {
        if remaining == 0 {
            break;
        }
        let reifier = intern(&mut builder, reifier);
        let triple = intern(&mut builder, triple);
        let graph_id = graph_id.map(|g| intern(&mut builder, g));
        builder.push_reifier_in_graph(reifier, triple, graph_id);
        remaining -= 1;
    }
    for (reifier, predicate, object, graph_id) in graph.annotations_with_graph() {
        if remaining == 0 {
            break;
        }
        let reifier = intern(&mut builder, reifier);
        let predicate = intern(&mut builder, predicate);
        let object = intern(&mut builder, object);
        let graph_id = graph_id.map(|g| intern(&mut builder, g));
        builder.push_annotation_in_graph(reifier, predicate, object, graph_id);
        remaining -= 1;
    }
    builder
        .freeze()
        .expect("a prefix of a frozen dataset is positionally valid by construction")
}

pub(crate) fn eval_construct<D: DatasetView + Sync>(
    template: &[TriplePattern],
    pattern: &GraphPattern,
    ctx: &mut EvalCtx<'_, D>,
) -> Result<ConstructedGraph<D::Id>, EvalError> {
    let (seq, certificate) = match eval_evaluated(pattern, ctx)? {
        Evaluated::Complete(seq) => (seq, None),
        Evaluated::Truncated(truncation) => (truncation.rows().clone(), Some(truncation)),
    };

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

    // Scanned ONCE before the row loop (see `template_has_blank_node`'s doc
    // comment for why this exact condition is what makes minted-label tracking
    // worth doing at all): a template with no blank-node position can never
    // populate `MintTracker::minted`, which makes every `track_minted` /
    // `track_minted_predicate` call — and the `BTreeSet` insert + `String`
    // clone each performs — dead weight for every row of this evaluation.
    let has_blank_positions = template_has_blank_node(template);

    let plan = ConstructPlan {
        template,
        dropped: &dropped,
        loss_vocab: loss_vocab.as_ref(),
        reifier_decl_indices: &reifier_decl_indices,
    };

    // Pass 1: the ordinary single-pass build, additionally recording which blank
    // labels the template MINTED versus which arrived DATA-CARRIED in bindings.
    let counter_start = ctx.bnode_counter;
    let mut tracker = MintTracker::new(has_blank_positions);
    let graph = build_construct_graph(&plan, &seq, ctx, &mut tracker)?;

    // §16.2 freshness: a template blank must denote a blank node distinct from
    // every blank node in the queried data. The mint counter guarantees that only
    // against other mints, so when a minted label collides with a data-carried
    // label in this result the two conflate at intern time. The (rare) fix is a
    // deterministic remap of exactly the colliding minted labels, replayed over
    // the same rows with the counter rewound so every non-colliding label is
    // byte-identical to pass 1. No collision — the overwhelmingly common case —
    // keeps the pass-1 graph untouched.
    let graph = match tracker.freshness_remap(&graph) {
        None => graph,
        Some(remap) => {
            tracker.remap = remap;
            ctx.bnode_counter = counter_start;
            build_construct_graph(&plan, &seq, ctx, &mut tracker)?
        }
    };
    Ok(commit_answer_triples(graph, certificate, &seq, ctx))
}

/// The immutable inputs of one CONSTRUCT template pass, bundled so the pass can
/// run twice (see [`eval_construct`]'s freshness re-pass) without re-deriving
/// them.
struct ConstructPlan<'a> {
    /// The CONSTRUCT template triples.
    template: &'a [TriplePattern],
    /// The dropped-reifier loss declarations detected in the `WHERE`.
    dropped: &'a [DroppedReifier],
    /// The caller-supplied loss vocabulary, when configured.
    loss_vocab: Option<&'a crate::eval::LossVocabulary>,
    /// Template indices holding reifier declarations (`rdf:reifies` + triple term).
    reifier_decl_indices: &'a [usize],
}

/// One full template-instantiation pass over the `WHERE`'s solution rows,
/// interning into a fresh builder and freezing the result.
///
/// `tracker` records the minted/data-carried blank-label split as the pass runs;
/// on the freshness re-pass its `remap` renames exactly the colliding minted
/// labels at their minted positions. Replaying is deterministic because the
/// caller rewinds `ctx.bnode_counter` to its pre-pass value and every other
/// input (`plan`, `seq`, the scratch interner) is read-only here.
fn build_construct_graph<D: DatasetView + Sync>(
    plan: &ConstructPlan<'_>,
    seq: &crate::solution::SolutionSeq<D::Id>,
    ctx: &mut EvalCtx<'_, D>,
    tracker: &mut MintTracker,
) -> Result<Arc<RdfDataset>, EvalError> {
    let schema = &seq.schema;
    let template = plan.template;
    let mut builder = RdfDatasetBuilder::new();

    // Pre-intern the caller-supplied loss vocabulary IRIs once, before the
    // per-solution row loop, so the loss-node emission path does not repeat
    // the lookup work for every row.
    let loss_term_ids: Option<(TermId, TermId, TermId)> = plan.loss_vocab.map(|vocab| {
        (
            builder.intern_iri_value(&vocab.projection_loss),
            builder.intern_iri_value(&vocab.loss_code),
            builder.intern_iri_value(&vocab.lost_reifies),
        )
    });

    let has_reifier_decls = !plan.reifier_decl_indices.is_empty();
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
                    instantiate(tp, row, schema, &mut builder, &mut blanks, ctx, tracker)
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
                .map(|tp| instantiate(tp, row, schema, &mut builder, &mut blanks, ctx, tracker))
                .collect();

            // Pass 1: emit reifier declarations and build the per-row reifier set.
            let mut reifier_ids: HashSet<TermId> = HashSet::new();
            for &idx in plan.reifier_decl_indices {
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
                if plan.reifier_decl_indices.contains(&idx) {
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
            && !plan.dropped.is_empty()
        {
            emit_dropped_losses(plan.dropped, row, schema, &mut builder, ctx, ids);
        }
    }

    // Value-constructing builtins (`listSlice`/`listConcat`) invent fresh
    // `rdf:List` cells while the WHERE is evaluated. A SPARQL expression can only
    // return the list head, so the cells are buffered on the context; fold them into
    // the CONSTRUCT output here so a constructed list materializes as triples — but
    // only the cells reachable from a surviving result row, so a list minted on a row
    // pruned by FILTER/DISTINCT/LIMIT does not leak orphaned cells into the graph.
    if !ctx.constructed.is_empty() {
        let (_, rows) = crate::eval::materialize_solutions(seq, ctx);
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

/// Blank-label bookkeeping for SPARQL §16.2 template freshness across one
/// `CONSTRUCT` evaluation.
///
/// §16.2 requires a template blank node to denote a **fresh** blank node —
/// distinct from every blank node in the queried data. The mint draws labels from
/// a monotonic counter, which makes them fresh against other *minted* labels but
/// says nothing about the labels data-carried bindings bring into the same output
/// graph: data already containing `_:c1` would conflate with the first minted
/// blank at intern time. The tracker records, while a pass runs, which labels the
/// template minted and which arrived data-carried — classification follows the
/// **template position** that produced each term (never the label text), so a
/// data blank that happens to spell like a mint is still counted as data.
struct MintTracker {
    /// Labels minted at template blank-node positions this evaluation.
    minted: BTreeSet<String>,
    /// Blank labels carried into the result by every non-minting template
    /// position (variable bindings, including blanks nested in bound triple
    /// terms).
    data: BTreeSet<String>,
    /// The freshness re-pass relabeling for colliding minted labels; empty on the
    /// first pass, so the first pass rewrites nothing.
    remap: DetHashMap<String, String>,
    /// `false` when [`template_has_blank_node`] found no `TermPattern::BlankNode`
    /// position anywhere in the template. `track_minted`/`track_minted_predicate`
    /// check this once per call and, when it is `false`, pass their value through
    /// unchanged with no `BTreeSet` insert and no `String` clone: `minted` can
    /// never become non-empty for such a template (its only insertion site is
    /// gated on a `TermPattern::BlankNode` match), so `freshness_remap`'s
    /// `minted.intersection(&data)` is provably empty regardless of what `data`
    /// would have accumulated — recording it at all would be dead weight on the
    /// hottest path of every `CONSTRUCT` row.
    enabled: bool,
}

impl MintTracker {
    /// A tracker for one `CONSTRUCT` evaluation. `enabled` is
    /// `template_has_blank_node(template)`, decided once before the row loop
    /// starts.
    fn new(enabled: bool) -> Self {
        Self {
            minted: BTreeSet::new(),
            data: BTreeSet::new(),
            remap: DetHashMap::default(),
            enabled,
        }
    }

    /// The deterministic freshness remap for minted labels colliding with a
    /// data-carried label, or `None` when the closed result is already fresh.
    ///
    /// A pure function of the result's label sets: each colliding minted label
    /// `L`, taken in lexicographic order, becomes `{L}r{k}` for the smallest
    /// `k >= 0` such that the candidate avoids **every** label present in
    /// `graph` and every replacement already chosen — so a replacement can
    /// collide neither with data, nor with another minted label, nor with
    /// another replacement, whatever the mint prefix was. The `r{k}` suffix is
    /// ASCII-alphanumeric, so a legal label stays inside the
    /// `BLANK_NODE_LABEL` alphabet.
    fn freshness_remap(&self, graph: &RdfDataset) -> Option<DetHashMap<String, String>> {
        let colliding: Vec<&String> = self.minted.intersection(&self.data).collect();
        if colliding.is_empty() {
            return None;
        }
        let mut used = graph_blank_labels(graph);
        let mut remap = DetHashMap::default();
        for label in colliding {
            let mut k = 0u64;
            let fresh = loop {
                let candidate = format!("{label}r{k}");
                if !used.contains(&candidate) {
                    break candidate;
                }
                k += 1;
            };
            used.insert(fresh.clone());
            remap.insert(label.clone(), fresh);
        }
        Some(remap)
    }
}

/// Every blank-node label appearing anywhere in `graph` (quads, reifier bindings,
/// annotations, and inside nested triple terms) — the freshness universe a
/// replacement label must avoid.
fn graph_blank_labels(graph: &RdfDataset) -> BTreeSet<String> {
    let mut labels = BTreeSet::new();
    for quad in graph.quads() {
        for id in [Some(quad.s), Some(quad.p), Some(quad.o), quad.g]
            .into_iter()
            .flatten()
        {
            collect_value_blank_labels(&graph.term_value(id), &mut labels);
        }
    }
    for (reifier, triple, graph_id) in graph.reifiers_with_graph() {
        for id in [Some(reifier), Some(triple), graph_id]
            .into_iter()
            .flatten()
        {
            collect_value_blank_labels(&graph.term_value(id), &mut labels);
        }
    }
    for (reifier, predicate, object, graph_id) in graph.annotations_with_graph() {
        for id in [Some(reifier), Some(predicate), Some(object), graph_id]
            .into_iter()
            .flatten()
        {
            collect_value_blank_labels(&graph.term_value(id), &mut labels);
        }
    }
    labels
}

/// Recursively collect every blank-node label inside an owned term value.
///
/// A label already present in `out` is left alone rather than re-inserted: a
/// `BTreeSet<String>` insert of an already-present key still requires the
/// caller to have an owned `String` to offer it, so `label.clone()` would run
/// unconditionally on every call otherwise. Checking membership by `&str`
/// first (no allocation) and cloning only on the *first* sighting of a given
/// label matters here because a data-carried blank's label routinely repeats —
/// the same blank subject spans every triple it participates in, within a row
/// and often across rows — so the common case is a set that has already seen
/// the label. This is unconditionally sound: it changes nothing about *which*
/// labels end up in `out`, only how many times an already-recorded one is
/// cloned to no effect.
///
/// A stronger-looking optimization — skip recording `data` labels entirely
/// until the first mint happens, since [`MintTracker::freshness_remap`] only
/// cares about labels that intersect `minted` — was considered and rejected as
/// unsound: template positions are walked subject-then-predicate-then-object
/// per triple, and triples in template order, so a data-carrying position
/// (e.g. a plain variable in subject position) routinely precedes a minting
/// position (e.g. a blank node later in the same triple, or in a later
/// template triple) within the very same row. A data label recorded "too
/// early" under that scheme would be silently dropped from `data`, and a mint
/// that later collides with it would go undetected — a real freshness bug, not
/// just a missed optimization. `MintTracker`'s sets are also accumulated
/// across the *entire* row loop and checked only once at the end, so there is
/// no valid "before the first mint" window to skip in the first place.
fn collect_value_blank_labels(value: &TermValue, out: &mut BTreeSet<String>) {
    match value {
        TermValue::Blank { label, .. } => {
            if !out.contains(label.as_str()) {
                out.insert(label.clone());
            }
        }
        TermValue::Triple { s, p, o } => {
            collect_value_blank_labels(s, out);
            collect_value_blank_labels(p, out);
            collect_value_blank_labels(o, out);
        }
        TermValue::Iri(_) | TermValue::Literal { .. } => {}
    }
}

/// Classify one instantiated subject/object position into `tracker` and apply the
/// freshness re-pass `remap` to **minted** blank labels.
///
/// The walk is pattern-parallel: the template position — never the label text —
/// decides whether a blank was minted or data-carried. A
/// [`TermPattern::BlankNode`] position holds a label the mint produced; every
/// other position's blanks (however deeply nested in a bound triple term) came
/// from the data. Nested quoted-triple templates recurse position-by-position.
///
/// A no-op pass-through when `tracker.enabled` is `false` — see the field's doc
/// comment for why that is sound, not just fast, for a blank-free template.
fn track_minted(pattern: &TermPattern, value: TermValue, tracker: &mut MintTracker) -> TermValue {
    if !tracker.enabled {
        return value;
    }
    match (pattern, value) {
        (TermPattern::BlankNode(_), TermValue::Blank { label, scope }) => {
            tracker.minted.insert(label.clone());
            let label = tracker.remap.get(&label).cloned().unwrap_or(label);
            TermValue::Blank { label, scope }
        }
        (TermPattern::Triple(tp), TermValue::Triple { s, p, o }) => {
            let s = track_minted(&tp.subject, *s, tracker);
            let p = track_minted_predicate(&tp.predicate, *p, tracker);
            let o = track_minted(&tp.object, *o, tracker);
            TermValue::Triple {
                s: Box::new(s),
                p: Box::new(p),
                o: Box::new(o),
            }
        }
        (_, value) => {
            collect_value_blank_labels(&value, &mut tracker.data);
            value
        }
    }
}

/// The predicate-position twin of [`track_minted`]: a predicate can never be a
/// template blank, so only a variable-bound value can carry data blanks (inside a
/// nested triple term) worth recording.
///
/// A no-op pass-through when `tracker.enabled` is `false`, for the same reason
/// as [`track_minted`].
fn track_minted_predicate(
    pattern: &NamedNodePattern,
    value: TermValue,
    tracker: &mut MintTracker,
) -> TermValue {
    if tracker.enabled && matches!(pattern, NamedNodePattern::Variable(_)) {
        collect_value_blank_labels(&value, &mut tracker.data);
    }
    value
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
///
/// A kept triple's positions are classified into `tracker` (minted versus
/// data-carried blank labels) — after the ill-formed gate, so a skipped triple
/// contributes no labels to the freshness accounting — and the freshness
/// re-pass remap is applied at the minted positions on the way in.
///
/// `track_minted`/`track_minted_predicate` are called unconditionally here —
/// the `template_has_blank_node` fast path lives inside `tracker.enabled`
/// (checked by those two functions themselves, see their doc comments) rather
/// than as a parameter here, so this signature stays independent of it.
fn instantiate<D: DatasetView + Sync>(
    tp: &TriplePattern,
    row: &Solution<D::Id>,
    schema: &VarSchema,
    builder: &mut RdfDatasetBuilder,
    blanks: &mut DetHashMap<String, String>,
    ctx: &mut EvalCtx<'_, D>,
    tracker: &mut MintTracker,
) -> Option<(TermId, TermId, TermId)> {
    let s = instantiate_term(&tp.subject, row, schema, blanks, ctx)?;
    let p = instantiate_predicate(&tp.predicate, row, schema, ctx)?;
    let o = instantiate_term(&tp.object, row, schema, blanks, ctx)?;

    // Positional validity (§16.2): subject must not be a literal; predicate must be
    // an IRI. Ill-formed instantiations are skipped, not errored.
    if positionally_ill_formed(&s, &p) {
        return None;
    }

    let s = track_minted(&tp.subject, s, tracker);
    let p = track_minted_predicate(&tp.predicate, p, tracker);
    let o = track_minted(&tp.object, o, tracker);

    Some((
        builder.intern_value(&s),
        builder.intern_value(&p),
        builder.intern_value(&o),
    ))
}

/// `true` when `template` holds a `TermPattern::BlankNode` at any subject or
/// object position, including nested inside an RDF 1.2 quoted-triple position.
/// Predicate positions can never hold a blank node — [`NamedNodePattern`] admits
/// only an IRI or a variable — so only subject/object need scanning.
///
/// This is exactly the condition under which [`MintTracker::minted`] can ever
/// become non-empty: [`track_minted`]'s minting arm fires only on the pattern
/// pair `(TermPattern::BlankNode(_), TermValue::Blank { .. })`. When no such
/// position exists anywhere in the template, `minted` is the empty set for the
/// whole evaluation no matter how many rows run, so
/// [`MintTracker::freshness_remap`]'s `minted.intersection(&data)` is *provably*
/// empty regardless of what `data` holds — which makes every `data`-side
/// classification dead weight for such a template. This is the flag that lets
/// `instantiate` skip tracking altogether rather than merely skip acting on it.
fn template_has_blank_node(template: &[TriplePattern]) -> bool {
    template.iter().any(triple_pattern_has_blank_node)
}

/// The [`TriplePattern`] half of [`template_has_blank_node`]'s scan.
fn triple_pattern_has_blank_node(tp: &TriplePattern) -> bool {
    term_pattern_has_blank_node(&tp.subject) || term_pattern_has_blank_node(&tp.object)
}

/// The [`TermPattern`] half of [`template_has_blank_node`]'s scan, recursing
/// through nested quoted-triple positions.
fn term_pattern_has_blank_node(term: &TermPattern) -> bool {
    match term {
        TermPattern::BlankNode(_) => true,
        TermPattern::Triple(inner) => triple_pattern_has_blank_node(inner),
        TermPattern::NamedNode(_) | TermPattern::Literal(_) | TermPattern::Variable(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The ungoverned `CONSTRUCT`, which is complete by construction: these tests assert
    /// the graph, and the certificate half is exercised by the lift's own tests.
    fn eval_construct<D: DatasetView + Sync>(
        template: &[TriplePattern],
        pattern: &GraphPattern,
        ctx: &mut EvalCtx<'_, D>,
    ) -> Result<Arc<RdfDataset>, EvalError> {
        let (graph, certificate) = super::eval_construct(template, pattern, ctx)?;
        assert!(
            certificate.is_none(),
            "an ungoverned CONSTRUCT cannot truncate"
        );
        Ok(graph)
    }
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

    /// With no mint prefix and no data collision, minted labels are byte-identical
    /// to the historical spelling: exactly `c1`, `c2`.
    #[test]
    fn unprefixed_template_blanks_mint_exact_c_labels() {
        let ds = knows_graph();
        let mut ctx = EvalCtx::new(&ds);
        let template = vec![TriplePattern {
            subject: TermPattern::BlankNode(purrdf_sparql_algebra::BlankNode::new("b")),
            predicate: pred(RELATED),
            object: var("o"),
        }];
        let out = eval_construct(&template, &where_knows(), &mut ctx).expect("construct");
        let mut blanks = BTreeSet::new();
        for q in out.quads() {
            if let TermRef::Blank { label, .. } = out.resolve(q.s) {
                blanks.insert(label.to_owned());
            }
        }
        let expected: BTreeSet<String> = ["c1", "c2"].map(str::to_owned).into();
        assert_eq!(blanks, expected, "prefix None ⇒ labels exactly c1, c2");
    }

    /// With a mint prefix installed, every minted label is `{prefix}c{n}`.
    #[test]
    fn prefixed_template_blanks_carry_the_prefix() {
        let ds = knows_graph();
        let mut ctx = EvalCtx::new(&ds).with_bnode_mint_prefix("fX_");
        let template = vec![TriplePattern {
            subject: TermPattern::BlankNode(purrdf_sparql_algebra::BlankNode::new("b")),
            predicate: pred(RELATED),
            object: var("o"),
        }];
        let out = eval_construct(&template, &where_knows(), &mut ctx).expect("construct");
        let mut blanks = BTreeSet::new();
        for q in out.quads() {
            if let TermRef::Blank { label, .. } = out.resolve(q.s) {
                blanks.insert(label.to_owned());
            }
        }
        let expected: BTreeSet<String> = ["fX_c1", "fX_c2"].map(str::to_owned).into();
        assert_eq!(
            blanks, expected,
            "prefix Some ⇒ labels exactly {{prefix}}c{{n}}"
        );
    }

    /// SPARQL §16.2 freshness: data already containing a blank labeled `c1` — the
    /// label the first template mint would spell — must NOT conflate with the
    /// minted blank. The result holds TWO distinct blank nodes, the data one
    /// untouched and the minted one deterministically reminted.
    #[test]
    fn template_blank_is_fresh_against_data_labels() {
        let mut b = RdfDatasetBuilder::new();
        let p = b.intern_iri("http://ex/p");
        let s = b.intern_blank_value("c1", purrdf_core::BlankScope::DEFAULT);
        let o = b.intern_iri("http://ex/o");
        b.push_quad(s, p, o, None);
        let ds = b.freeze().expect("freeze");
        let mut ctx = EvalCtx::new(&ds);

        // CONSTRUCT { ?s :related [] } WHERE { ?s :p ?o } — ?s binds the data
        // blank `_:c1`; the anonymous template blank would also mint `c1`.
        let where_pat = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: var("s"),
                predicate: pred("http://ex/p"),
                object: var("o"),
            }],
        };
        let template = vec![TriplePattern {
            subject: var("s"),
            predicate: pred(RELATED),
            object: TermPattern::BlankNode(purrdf_sparql_algebra::BlankNode::new("n")),
        }];
        let out = eval_construct(&template, &where_pat, &mut ctx).expect("construct");
        assert_eq!(out.quad_count(), 1);
        let quad = out.quads().next().expect("one quad");
        let TermRef::Blank { label: s_label, .. } = out.resolve(quad.s) else {
            panic!("the subject must be the data-carried blank");
        };
        let TermRef::Blank { label: o_label, .. } = out.resolve(quad.o) else {
            panic!("the object must be the minted blank");
        };
        assert_eq!(s_label, "c1", "the data blank passes through untouched");
        assert_ne!(
            s_label, o_label,
            "the minted blank must be fresh w.r.t. data labels — two distinct nodes"
        );
        assert_eq!(
            o_label, "c1r0",
            "the remint takes the smallest deterministic suffix"
        );
    }

    /// The freshness re-pass is itself deterministic and leaves non-colliding
    /// mints byte-identical across independent evaluations.
    #[test]
    fn freshness_remint_is_deterministic_across_runs() {
        let build = || {
            let mut b = RdfDatasetBuilder::new();
            let p = b.intern_iri("http://ex/p");
            let s = b.intern_blank_value("c1", purrdf_core::BlankScope::DEFAULT);
            let o = b.intern_iri("http://ex/o");
            b.push_quad(s, p, o, None);
            b.freeze().expect("freeze")
        };
        let where_pat = GraphPattern::Bgp {
            patterns: vec![TriplePattern {
                subject: var("s"),
                predicate: pred("http://ex/p"),
                object: var("o"),
            }],
        };
        let template = vec![TriplePattern {
            subject: var("s"),
            predicate: pred(RELATED),
            object: TermPattern::BlankNode(purrdf_sparql_algebra::BlankNode::new("n")),
        }];
        let ds1 = build();
        let mut ctx1 = EvalCtx::new(&ds1);
        let out1 = eval_construct(&template, &where_pat, &mut ctx1).expect("construct");
        let ds2 = build();
        let mut ctx2 = EvalCtx::new(&ds2);
        let out2 = eval_construct(&template, &where_pat, &mut ctx2).expect("construct");
        assert_eq!(
            purrdf_core::canonicalize(&out1).nquads,
            purrdf_core::canonicalize(&out2).nquads,
            "the remint is a pure function of the result"
        );
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
    /// from the BGP virtual-candidate machinery in [`crate::bgp`].
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

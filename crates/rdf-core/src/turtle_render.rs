// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The canonical, review-friendly Turtle **renderer** over the purrdf IR —
//! the kernel half of the on-disk normalizer.
//!
//! [`render`] takes a frozen [`RdfDataset`] and a prefix set and produces canonical
//! Turtle text. It is a pure function of the graph: blank/triple object ordering is
//! derived from subtree CONTENT (never from `TermId` interning order), so the output
//! is idempotent and independent of how the terms were interned. The
//! text *parser* (`canonical_turtle` / `ingest`) lives in `purrdf`; this kernel half
//! depends only on the IR, so it builds for `wasm32` and is the canonical-Turtle
//! authority for the wasm-clean compiler (the correspondence EDOAL lowering).
//!
//! The output improves on rdflib's `longturtle`:
//!
//! - **Inline blank nodes** `[ … ]` for once-referenced anonymous nodes.
//! - **RDF collection** `( … )` syntax for well-formed `rdf:List`s.
//! - **`a`-first**, predicates then objects sorted, one object per line.
//! - **Native literal syntax** where lossless; `"""…"""` for multi-line.
//! - **Deterministic, idempotent** blank labels: inline where possible; a rare
//!   shared/cyclic blank gets a structural-signature-derived `_:bN` label.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use purrdf_hash::fnv;

use crate::model::RdfTextDirection;
use crate::{DatasetView, FastMap, GraphMatch, RdfDataset, TermId, TermRef};
use purrdf_iri::{PrefixMap, contract_where};
use purrdf_lex::literal_escape::{self, Carrier};
use purrdf_lex::term_syntax::{TRIPLE_TERM_CLOSE, TRIPLE_TERM_OPEN, write_iri};

use purrdf_iri::vocab::rdf::NS as RDF;
use purrdf_xsd::datatype::XSD_NS as XSD;

fn rdf(local: &str) -> String {
    format!("{RDF}{local}")
}
fn xsd(local: &str) -> String {
    format!("{XSD}{local}")
}

/// Render a frozen dataset as canonical, review-friendly Turtle. `prefixes` supplies
/// the candidate prefix bindings; only those actually used appear in the header.
pub fn render(dataset: &RdfDataset, prefixes: &[(String, String)]) -> String {
    // Longest-namespace-first so the most specific prefix wins on abbreviation.
    let mut prefixes: Vec<(String, String)> = prefixes.to_vec();
    prefixes.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(&b.0)));
    Renderer::new(dataset, prefixes).render()
}

/// A predicate→objects map (both ordered) for one subject.
type Props = BTreeMap<TermId, BTreeSet<ObjKey>>;

struct Renderer<'a> {
    dataset: &'a RdfDataset,
    prefixes: Vec<(String, String)>,
    /// The same bindings, for CURIE compaction.
    prefix_map: PrefixMap,
    /// Each subject's properties.
    by_subject: FastMap<TermId, Props>,
    /// Times each blank `TermId` appears as an object.
    object_refs: FastMap<TermId, usize>,
    /// `_:bN` labels for shared/cyclic blanks that cannot inline.
    shared_labels: FastMap<TermId, String>,
    /// Prefixes actually used during rendering.
    used_prefixes: RefCell<BTreeSet<String>>,
    /// The well-known predicate ids, or `None` when the term table has no such IRI.
    rdf_type: Option<TermId>,
    rdf_reifies: Option<TermId>,
    rdf_rest: Option<TermId>,
}

impl<'a> Renderer<'a> {
    fn new(dataset: &'a RdfDataset, prefixes: Vec<(String, String)>) -> Self {
        // Phase 1: collect each subject's predicate→object multiset as raw `TermId`s,
        // and count blank object references. We deliberately do NOT order objects yet:
        // the ordering of blank/triple objects is a pure function of their subtree
        // CONTENT (computed in phase 2), never of `TermId` interning order — that is
        // what makes the render idempotent regardless of how the parser interned terms.
        let mut raw: FastMap<TermId, BTreeMap<TermId, Vec<TermId>>> = FastMap::default();
        let mut object_refs: FastMap<TermId, usize> = FastMap::default();
        // The RDF 1.2 statement layer (reifier bindings + annotations) lives in SIDE
        // TABLES, not `quads` — so the canonical renderer must fold in `reifier_quads`
        // (`<reifier> rdf:reifies << s p o >>`) and `annotation_quads`
        // (`<reifier> <pred> <value>`) alongside the base quads, or it silently drops the
        // whole statement layer (bug 2). Folding them in here — rather than a
        // separate emission pass — keeps a reifier subject's `rdf:reifies` edge and its
        // annotations on ONE flat top-level subject (`<reifier> rdf:reifies << s p o >> ;
        // <ann> <v> .`), which round-trips idempotently: the quoted triple term renders as
        // `<< s p o >>` (never as a nested blank), so re-parsing reproduces the same side
        // tables without growing the graph.
        let rows = dataset
            .quads()
            .map(|q| (q.s, q.p, q.o))
            .chain(dataset.reifier_quads().map(|q| (q.s, q.p, q.o)))
            .chain(dataset.annotation_quads().map(|q| (q.s, q.p, q.o)));
        for (s, p, o) in rows {
            raw.entry(s).or_default().entry(p).or_default().push(o);
            if matches!(dataset.resolve(o), TermRef::Blank { .. }) {
                *object_refs.entry(o).or_default() += 1;
            }
        }

        // Phase 2: a content-derived ordering key for every blank/triple term, walked
        // recursively over `raw` (the sorted (predicate, object-key) pairs of a blank's
        // properties), bounded against cycles. Grounded objects use their own lexical
        // key, so the result distinguishes sibling restrictions by their actual content
        // (`owl:onProperty`/`owl:someValuesFrom`/…) rather than by interning order.
        let content = ContentKeys::new(dataset, &raw);

        // Materialize the ordered `Props`: grounded objects keep their lexical key,
        // blank/triple objects sort by the content key computed above.
        let by_subject: FastMap<TermId, Props> = raw
            .iter()
            .map(|(&s, preds)| {
                let props: Props = preds
                    .iter()
                    .map(|(&p, objs)| {
                        let set: BTreeSet<ObjKey> = objs
                            .iter()
                            .map(|&o| ObjKey::new(dataset, o, &content))
                            .collect();
                        (p, set)
                    })
                    .collect();
                (s, props)
            })
            .collect();

        // Deterministic labels for blanks that cannot inline (referenced 0 or >1
        // times as an object), ordered by a structural signature so the labeling is
        // idempotent and stable under graph isomorphism for non-symmetric graphs.
        let mut shared: Vec<TermId> = by_subject
            .keys()
            .copied()
            .chain(object_refs.keys().copied())
            .filter(|id| matches!(dataset.resolve(*id), TermRef::Blank { .. }))
            .filter(|id| object_refs.get(id).copied().unwrap_or(0) != 1)
            .collect();
        shared.sort();
        shared.dedup();
        let sigs = blank_signatures(dataset, &by_subject, &shared);
        // Order labels by the structural-signature hash; on a hash tie fall back to the
        // fuller content key (still pure graph content), NEVER to `id.index()`, so the
        // `_:bN` numbering is idempotent under any interning order.
        shared.sort_by_cached_key(|id| (sigs.get(id).copied().unwrap_or(0), content.key_for(*id)));
        let shared_labels = shared
            .into_iter()
            .enumerate()
            .map(|(i, id)| (id, format!("_:b{i}")))
            .collect();

        let prefix_map = prefixes.iter().cloned().collect();
        let mut r = Self {
            dataset,
            prefixes,
            prefix_map,
            by_subject,
            object_refs,
            shared_labels,
            used_prefixes: RefCell::new(BTreeSet::new()),
            rdf_type: None,
            rdf_reifies: None,
            rdf_rest: None,
        };
        // Resolve the well-known predicate ids by scanning the term table (they may
        // be absent, in which case the sentinel never matches a real predicate).
        r.rdf_type = r.find_iri(&rdf("type"));
        r.rdf_reifies = r.find_iri(&rdf("reifies"));
        r.rdf_rest = r.find_iri(&rdf("rest"));
        r
    }

    /// The `TermId` of an interned IRI, or `None` if the term table has no such IRI.
    fn find_iri(&self, iri: &str) -> Option<TermId> {
        for i in 0..self.dataset.term_count() {
            let id = TermId::from_index(i as u32);
            if let TermRef::Iri(v) = self.dataset.resolve(id)
                && v == iri
            {
                return Some(id);
            }
        }
        None
    }

    fn is_inline_bnode(&self, id: TermId) -> bool {
        matches!(self.dataset.resolve(id), TermRef::Blank { .. })
            && self.object_refs.get(&id).copied().unwrap_or(0) == 1
    }

    fn render(&self) -> String {
        let mut tops: Vec<TermId> = self
            .by_subject
            .keys()
            .copied()
            .filter(|id| !self.is_inline_bnode(*id))
            .collect();
        tops.sort_by_cached_key(|id| self.subject_sort_key(*id));

        let mut body = String::new();
        for (i, subj) in tops.iter().enumerate() {
            if i > 0 {
                body.push('\n');
            }
            body.push_str(&self.subject_label(*subj, 0));
            body.push('\n');
            self.render_props(*subj, 1, &mut body, true);
        }

        let used = self.used_prefixes.borrow();
        let mut header = String::new();
        for (p, ns) in &self.prefixes {
            if used.contains(p) {
                let _ = writeln!(header, "@prefix {p}: <{ns}> .");
            }
        }
        if header.is_empty() {
            body
        } else {
            format!("{header}\n{body}")
        }
    }

    fn render_props(&self, subj: TermId, depth: usize, out: &mut String, top: bool) {
        let indent = "    ".repeat(depth);
        let Some(props) = self.by_subject.get(&subj) else {
            return;
        };
        // `a` (rdf:type) first, then `rdf:reifies` — the reifier's defining edge must
        // precede its annotations so a parser that folds a reifier's sibling triples as
        // annotations sees the `rdf:reifies` binding first (idempotent round trip
        // and the canonical committed form `<r> rdf:reifies << s p o >> ; <ann> .`).
        let mut preds: Vec<TermId> = props.keys().copied().collect();
        preds.sort_by_cached_key(|p| {
            (
                Some(*p) != self.rdf_type,
                Some(*p) != self.rdf_reifies,
                self.iri_of(*p),
            )
        });

        let last_pred = preds.len().saturating_sub(1);
        for (pi, pred) in preds.iter().enumerate() {
            let objs: Vec<&ObjKey> = props[pred].iter().collect();
            let pred_str = if Some(*pred) == self.rdf_type {
                "a".to_string()
            } else {
                self.term_label(*pred)
            };
            let last_obj = objs.len().saturating_sub(1);
            for (oi, obj) in objs.iter().enumerate() {
                // First object sits on the predicate line (indent `depth`);
                // continuation objects sit one level deeper, so a nested `[ … ]`
                // closes in alignment with its own opening line.
                let obj_depth = if oi == 0 { depth } else { depth + 1 };
                let rendered = self.render_object(obj.id, obj_depth);
                let terminator = if pi == last_pred && oi == last_obj {
                    if top { " ." } else { " ;" }
                } else if oi == last_obj {
                    " ;"
                } else {
                    " ,"
                };
                if oi == 0 {
                    let _ = writeln!(out, "{indent}{pred_str} {rendered}{terminator}");
                } else {
                    let _ = writeln!(out, "{indent}    {rendered}{terminator}");
                }
            }
        }
    }

    fn render_object(&self, id: TermId, depth: usize) -> String {
        match self.dataset.resolve(id) {
            TermRef::Iri(iri) => self.iri(iri),
            TermRef::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => self.literal(lexical, datatype, language, direction),
            TermRef::Blank { .. } => {
                if self.is_inline_bnode(id) {
                    if let Some(list) = self.try_collection(id) {
                        return self.render_collection(&list, depth);
                    }
                    self.render_inline_bnode(id, depth)
                } else {
                    self.shared_labels
                        .get(&id)
                        .cloned()
                        .unwrap_or_else(|| "[]".to_string())
                }
            }
            TermRef::Triple { s, p, o } => {
                // RDF-1.2 TRIPLE TERM: `<<( s p o )>>`. The parens matter — the bare
                // `<< s p o >>` form is a *reifying triple* that ALSO asserts `s p o`
                // (and mints a reifier), so re-parsing it would grow the graph and break
                // the `rdf:reifies` object. A triple term denotes the triple without
                // asserting it, exactly as the gts codec serializer emits it.
                format!(
                    "{TRIPLE_TERM_OPEN} {} {} {} {TRIPLE_TERM_CLOSE}",
                    self.render_object(s, depth),
                    self.term_label(p),
                    self.render_object(o, depth)
                )
            }
        }
    }

    fn render_inline_bnode(&self, id: TermId, depth: usize) -> String {
        if self.by_subject.get(&id).is_none_or(BTreeMap::is_empty) {
            return "[]".to_string();
        }
        let mut inner = String::new();
        self.render_props(id, depth + 1, &mut inner, false);
        let close_indent = "    ".repeat(depth);
        format!("[\n{inner}{close_indent}]")
    }

    /// A well-formed `rdf:List` headed by `id` ([`DatasetView::rdf_list_strict`])
    /// whose every cell is an inline blank carrying nothing but its `rdf:first`
    /// and `rdf:rest`, so the `( … )` form writes the whole of each cell. Returns
    /// the members.
    fn try_collection(&self, id: TermId) -> Option<Vec<TermId>> {
        let rdf_rest = self.rdf_rest?;
        let members = self.dataset.rdf_list_strict(id, GraphMatch::Any).ok()?;
        let mut cell = id;
        for _ in &members {
            let props = self.by_subject.get(&cell)?;
            if !self.is_inline_bnode(cell) || props.len() != 2 {
                return None;
            }
            cell = props.get(&rdf_rest)?.iter().next()?.id;
        }
        Some(members)
    }

    fn render_collection(&self, items: &[TermId], depth: usize) -> String {
        if items.is_empty() {
            return "()".to_string();
        }
        let rendered: Vec<String> = items
            .iter()
            .map(|t| self.render_object(*t, depth))
            .collect();
        format!("( {} )", rendered.join(" "))
    }

    // ── term formatting ──────────────────────────────────────────────────────

    /// The IRI string of an interned predicate/IRI term.
    fn iri_of(&self, id: TermId) -> String {
        match self.dataset.resolve(id) {
            TermRef::Iri(iri) => iri.to_owned(),
            _ => String::new(),
        }
    }

    /// A subject/predicate term's label (abbreviated IRI or shared blank label).
    fn term_label(&self, id: TermId) -> String {
        match self.dataset.resolve(id) {
            TermRef::Iri(iri) => self.iri(iri),
            TermRef::Blank { .. } => self
                .shared_labels
                .get(&id)
                .cloned()
                .unwrap_or_else(|| "[]".to_string()),
            _ => "[]".to_string(),
        }
    }

    /// A subject term's label. Like [`term_label`](Self::term_label) for an IRI or
    /// blank-node subject, but renders an RDF-1.2 quoted-triple subject via the
    /// `<< s p o >>` path instead of silently flattening it to `[]` (which would drop
    /// the asserted statement's subject identity from the graph).
    fn subject_label(&self, id: TermId, depth: usize) -> String {
        match self.dataset.resolve(id) {
            TermRef::Triple { .. } => self.render_object(id, depth),
            _ => self.term_label(id),
        }
    }

    fn iri(&self, iri: &str) -> String {
        if let Some((curie, prefix_len)) = self.curie(iri) {
            self.used_prefixes
                .borrow_mut()
                .insert(curie[..prefix_len].to_owned());
            return curie;
        }
        let mut out = String::with_capacity(iri.len() + 2);
        write_iri(iri, &mut out);
        out
    }

    /// `iri` as a Turtle prefixed name under the longest declared namespace
    /// whose local part [`is_valid_pn_local`] admits
    /// ([`purrdf_iri::contract_where`]; the empty prefix `:` included), and the
    /// prefix's length.
    fn curie(&self, iri: &str) -> Option<(String, usize)> {
        let curie = contract_where(iri, &self.prefix_map, |_, local| is_valid_pn_local(local))?;
        let prefix_len = curie.find(':')?;
        Some((curie, prefix_len))
    }

    fn literal(
        &self,
        lexical: &str,
        datatype: TermId,
        language: Option<&str>,
        direction: Option<RdfTextDirection>,
    ) -> String {
        if let Some(lang) = language {
            // RDF 1.2 base direction renders as `"text"@lang--ltr` / `--rtl`; a base
            // direction requires a language tag, so it only appears on this branch.
            return match direction {
                Some(dir) => format!("{}@{}--{}", quote(lexical), lang, dir.as_str()),
                None => format!("{}@{}", quote(lexical), lang),
            };
        }
        let dt = self.iri_of(datatype);
        if dt == xsd("string") {
            return quote(lexical);
        }
        if dt == xsd("boolean") && (lexical == "true" || lexical == "false") {
            return lexical.to_owned();
        }
        if dt == xsd("integer") && is_turtle_integer(lexical) {
            return lexical.to_owned();
        }
        if dt == xsd("decimal") && is_turtle_decimal(lexical) {
            return lexical.to_owned();
        }
        if dt == xsd("double") && is_turtle_double(lexical) {
            return lexical.to_owned();
        }
        format!("{}^^{}", quote(lexical), self.iri(&dt))
    }

    fn subject_sort_key(&self, id: TermId) -> (u8, String) {
        match self.dataset.resolve(id) {
            TermRef::Iri(iri) => (0, self.abbrev_for_sort(iri)),
            TermRef::Blank { .. } => (1, self.shared_labels.get(&id).cloned().unwrap_or_default()),
            _ => (2, String::new()),
        }
    }

    /// Abbreviation used only for ORDERING (does not record prefix usage).
    fn abbrev_for_sort(&self, iri: &str) -> String {
        self.curie(iri)
            .map_or_else(|| iri.to_owned(), |(curie, _)| curie)
    }
}

/// Content-derived ordering keys for every blank/triple term in the graph.
///
/// The key for a blank is a canonical string built from the sorted
/// `(predicate-iri, object-key)` pairs of its properties; the key for a triple term
/// is built from its `s`/`p`/`o` component keys. Both recurse through nested
/// blank/triple objects so the key is a pure function of the term's subtree CONTENT —
/// independent of `TermId` interning order — which is what makes the render idempotent.
/// Recursion is bounded by a `seen` set so cyclic blank graphs terminate (a back-edge
/// to an in-progress node renders as a fixed `^` marker), and a depth budget caps
/// pathological chains; ties under the budget are harmless because they only affect
/// sort order between structurally indistinguishable subtrees.
struct ContentKeys {
    keys: FastMap<TermId, String>,
}

impl ContentKeys {
    const MAX_DEPTH: usize = 40;

    fn new(dataset: &RdfDataset, raw: &FastMap<TermId, BTreeMap<TermId, Vec<TermId>>>) -> Self {
        // `keys` doubles as the memoization cache: every acyclic blank/triple term
        // `compute_content_key` fully resolves is inserted, so a single traversal from
        // the top-level subjects/objects populates keys for ALL reachable nested
        // blank/triple terms (not just `q.s`/`q.o`) and never recomputes a shared
        // subtree. Cyclic / depth-capped subtrees are deliberately left out (see the
        // `cacheable` flag in `compute_content_key`).
        let mut keys: FastMap<TermId, String> = FastMap::default();
        for q in dataset.quads() {
            for term in [q.s, q.o] {
                if matches!(
                    dataset.resolve(term),
                    TermRef::Blank { .. } | TermRef::Triple { .. }
                ) && !keys.contains_key(&term)
                {
                    let mut seen = BTreeSet::new();
                    compute_content_key(dataset, raw, term, &mut seen, 0, &mut keys);
                }
            }
        }
        Self { keys }
    }

    /// The content key of a blank/triple term (empty for grounded terms, which never
    /// consult this map).
    fn key_for(&self, id: TermId) -> String {
        self.keys.get(&id).cloned().unwrap_or_default()
    }
}

/// Recursively fold a term into a canonical content string. Grounded terms map to
/// their lexical form; blank/triple terms descend into their subtree.
///
/// Returns `(key, cacheable)`. `cacheable` is `false` iff the subtree hit a back-edge
/// or the depth budget — those `^` markers are entry-point-RELATIVE, so an enclosing
/// key that embeds one must not be memoized (it would otherwise return an
/// interning-order-dependent value when the same node is later reached from a different
/// root, reintroducing the very non-determinism this fold exists to remove). Only fully
/// resolved acyclic subtrees are written to `cache`; cyclic / over-budget blank graphs
/// fall through to the `id.index()` tiebreak in [`ObjKey`], exactly as before this fix.
fn compute_content_key(
    dataset: &RdfDataset,
    raw: &FastMap<TermId, BTreeMap<TermId, Vec<TermId>>>,
    id: TermId,
    seen: &mut BTreeSet<TermId>,
    depth: usize,
    cache: &mut FastMap<TermId, String>,
) -> (String, bool) {
    // A memoized key is always a fully-resolved acyclic blank/triple key (leaf terms are
    // never cached), so reusing it is sound and cannot be a live back-edge: a node is
    // only inserted AFTER `seen.remove`, hence a cached id is never simultaneously in
    // `seen`.
    if let Some(k) = cache.get(&id) {
        return (k.clone(), true);
    }
    match dataset.resolve(id) {
        TermRef::Iri(iri) => (format!("I{iri}"), true),
        TermRef::Literal {
            lexical,
            datatype,
            language,
            ..
        } => {
            let dt = match dataset.resolve(datatype) {
                TermRef::Iri(iri) => iri,
                _ => "",
            };
            (
                format!("L{dt}\u{1}{}\u{1}{lexical}", language.unwrap_or("")),
                true,
            )
        }
        TermRef::Blank { .. } => {
            if depth >= ContentKeys::MAX_DEPTH || !seen.insert(id) {
                // Back-edge or budget exhausted: a stable marker keeps the fold finite,
                // but the enclosing key is NOT safe to memoize.
                return ("^".to_string(), false);
            }
            let mut cacheable = true;
            let mut parts: Vec<String> = Vec::new();
            if let Some(props) = raw.get(&id) {
                for (&pred, objs) in props {
                    let pk = match dataset.resolve(pred) {
                        TermRef::Iri(iri) => iri,
                        _ => "",
                    };
                    // Sort the per-predicate object keys so the fold is order-free.
                    let mut oks: Vec<String> = objs
                        .iter()
                        .map(|&o| {
                            let (k, c) =
                                compute_content_key(dataset, raw, o, seen, depth + 1, cache);
                            cacheable &= c;
                            k
                        })
                        .collect();
                    oks.sort();
                    parts.push(format!("{pk}\u{2}{}", oks.join("\u{3}")));
                }
            }
            parts.sort();
            seen.remove(&id);
            let key = format!("B[{}]", parts.join("\u{4}"));
            if cacheable {
                cache.insert(id, key.clone());
            }
            (key, cacheable)
        }
        TermRef::Triple { s, p, o } => {
            let (sk, cs) = compute_content_key(dataset, raw, s, seen, depth + 1, cache);
            let (pk, cp) = compute_content_key(dataset, raw, p, seen, depth + 1, cache);
            let (ok, co) = compute_content_key(dataset, raw, o, seen, depth + 1, cache);
            let cacheable = cs && cp && co;
            let key = format!("T<{sk}\u{1}{pk}\u{1}{ok}>");
            if cacheable {
                cache.insert(id, key.clone());
            }
            (key, cacheable)
        }
    }
}

/// An object term keyed for deterministic sorting, carrying its `TermId`.
#[derive(Clone)]
struct ObjKey {
    id: TermId,
    key: (u8, String),
}

impl ObjKey {
    fn new(dataset: &RdfDataset, id: TermId, content: &ContentKeys) -> Self {
        let key = match dataset.resolve(id) {
            TermRef::Iri(iri) => (0, iri.to_owned()),
            TermRef::Literal {
                lexical,
                datatype,
                language,
                ..
            } => {
                let dt = match dataset.resolve(datatype) {
                    TermRef::Iri(iri) => iri.to_owned(),
                    _ => String::new(),
                };
                (
                    1,
                    format!("{dt}\u{1}{}\u{1}{lexical}", language.unwrap_or("")),
                )
            }
            // Blanks/triples sort after grounded terms, ordered by a CONTENT-derived
            // key (the recursive structural signature of their subtree) so sibling
            // inline blocks order by what they say, idempotently under any interning
            // order — never by `id.index()`.
            TermRef::Blank { .. } => (2, content.key_for(id)),
            TermRef::Triple { .. } => (3, content.key_for(id)),
        };
        Self { id, key }
    }
}

impl PartialEq for ObjKey {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl Eq for ObjKey {}
impl PartialOrd for ObjKey {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for ObjKey {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.key
            .cmp(&other.key)
            .then(self.id.index().cmp(&other.id.index()))
    }
}

/// A deterministic structural signature for each shared blank, used only to ORDER
/// their `_:bN` labels. A bounded refinement (two rounds): round 0 folds each
/// blank's grounded neighbourhood; round 1 folds neighbour signatures. Sufficient
/// for the non-symmetric blank graphs the authored ontology sources contain.
fn blank_signatures(
    dataset: &RdfDataset,
    by_subject: &FastMap<TermId, Props>,
    shared: &[TermId],
) -> FastMap<TermId, u64> {
    let ground = |id: TermId| -> u64 { ground_sig(dataset, id, 0) };
    let shared_set: BTreeSet<TermId> = shared.iter().copied().collect();
    let mut sig: FastMap<TermId, u64> = shared.iter().map(|&b| (b, 1)).collect();
    for round in 0..2 {
        let mut next = sig.clone();
        for &b in shared {
            let mut acc = round as u64 + 1;
            if let Some(props) = by_subject.get(&b) {
                for (pred, objs) in props {
                    let pg = ground(*pred);
                    for obj in objs {
                        let og = if shared_set.contains(&obj.id) {
                            sig.get(&obj.id).copied().unwrap_or(1)
                        } else {
                            ground(obj.id)
                        };
                        // Commutative fold across statements.
                        acc ^= pg.wrapping_mul(fnv::PRIME) ^ og.rotate_left(17);
                    }
                }
            }
            next.insert(b, acc);
        }
        sig = next;
    }
    sig
}

/// A grounded content hash for the blank-signature fold: IRIs/literals by their
/// lexical content, an RDF-1.2 quoted triple by its `(s, p, o)` content (so reifier
/// blanks that reify DIFFERENT statements get distinct signatures — without this they
/// tie and the `_:bN` labeling falls back to interning order), and a blank as 0 (its
/// own signature carries it). Depth-capped against pathological nested triple terms.
fn ground_sig(dataset: &RdfDataset, id: TermId, depth: usize) -> u64 {
    match dataset.resolve(id) {
        TermRef::Iri(iri) => fnv::fnv1a64(iri.as_bytes()),
        TermRef::Literal { lexical, .. } => fnv::fold(0x1000_0001, lexical.as_bytes()),
        TermRef::Triple { s, p, o } if depth < 8 => {
            let s = ground_sig(dataset, s, depth + 1);
            let p = ground_sig(dataset, p, depth + 1);
            let o = ground_sig(dataset, o, depth + 1);
            0x3000_0001u64 ^ s.wrapping_mul(fnv::PRIME) ^ p.rotate_left(11) ^ o.rotate_left(23)
        }
        _ => 0, // blank, or a triple deeper than the cap: carried by its own signature
    }
}

// ── lexical helpers ──────────────────────────────────────────────────────────

/// Whether `local` is a CURIE local name safe to emit unescaped (so `prefix:local` is
/// valid Turtle). This is a conservative ASCII subset of the `PN_LOCAL` grammar:
/// - the first char is alphanumeric or `_`;
/// - interior chars are alphanumeric or `_`/`-`/`.`;
/// - the last char must not be `.` (a trailing `.` would read as the statement
///   terminator), and `.` is disallowed in a single-char local;
/// - `~` is disallowed (it is only legal in Turtle as the escape `\~`, never bare).
///
/// Anything outside this set falls back to the `<...>` absolute form at the call site.
fn is_valid_pn_local(local: &str) -> bool {
    let mut chars = local.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_alphanumeric() || first == '_') {
        return false;
    }
    if local.ends_with('.') {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

/// A string literal's quoted body: a value holding a LINE FEED as a Turtle
/// long string (`"""…"""`, the [`TurtleLong`](Carrier::TurtleLong) carrier,
/// which keeps the line breaks readable), every other value as a short string
/// (the [`Xml`](Carrier::Xml) carrier). Both carriers escape the C1 controls,
/// because this rendering feeds the CL-dialect carrier, whose payload is
/// embedded in an XML text node where raw C1 is normalized on read.
fn quote(value: &str) -> String {
    let (delimiter, carrier) = if value.contains('\n') {
        ("\"\"\"", Carrier::TurtleLong)
    } else {
        ("\"", Carrier::Xml)
    };
    let mut out = String::with_capacity(value.len() + 2 * delimiter.len());
    out.push_str(delimiter);
    literal_escape::write(value, carrier, &mut out);
    out.push_str(delimiter);
    out
}

fn is_turtle_integer(v: &str) -> bool {
    let s = v.strip_prefix(['+', '-']).unwrap_or(v);
    !s.is_empty()
        && s.bytes().all(|b| b.is_ascii_digit())
        && (s.len() == 1 || s.as_bytes()[0] != b'0')
}

fn is_turtle_decimal(v: &str) -> bool {
    let s = v.strip_prefix(['+', '-']).unwrap_or(v);
    match s.split_once('.') {
        Some((a, b)) => {
            !b.is_empty()
                && a.bytes().all(|c| c.is_ascii_digit())
                && b.bytes().all(|c| c.is_ascii_digit())
        }
        None => false,
    }
}

fn is_turtle_double(v: &str) -> bool {
    let lower = v.to_ascii_lowercase();
    lower.contains('e') && lower.parse::<f64>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Turtle's empty prefix is a prefix, and a namespace whose local part is not a
    /// `PN_LOCAL` yields to a shorter one that gives a valid name; an IRI no namespace
    /// names validly stays bracketed.
    #[test]
    fn prefixed_names_take_the_empty_prefix_and_fall_back_to_a_valid_local_part() {
        use crate::ir::builder::RdfDatasetBuilder;
        let render_one = |s: &str, p: &str, o: &str, prefixes: &[(&str, &str)]| {
            let mut b = RdfDatasetBuilder::new();
            let (s, p, o) = (b.intern_iri(s), b.intern_iri(p), b.intern_iri(o));
            b.push_quad(s, p, o, None);
            let prefixes: Vec<(String, String)> = prefixes
                .iter()
                .map(|&(prefix, ns)| (prefix.to_owned(), ns.to_owned()))
                .collect();
            render(&b.freeze().unwrap(), &prefixes)
        };
        let prefixes = [("", "http://example.org/"), ("x", "http://example.org/ab")];

        // The empty prefix is a prefix.
        let out = render_one(
            "http://example.org/s",
            "http://example.org/p",
            "http://example.org/o",
            &prefixes,
        );
        assert!(
            out.contains("\n:s\n    :p :o ."),
            "the empty prefix compacts, got:\n{out}"
        );

        // Under the longest namespace `x:` the local part `.c` is no PN_LOCAL (it may
        // not start with `.`), so the shorter `:` namespace names it as `:ab.c`.
        let out = render_one(
            "http://example.org/ab.c",
            "http://example.org/abp",
            "http://example.org/o",
            &prefixes,
        );
        assert!(
            out.contains("\n:ab.c\n    x:p :o ."),
            "fallback to a valid local, got:\n{out}"
        );

        // A local part no namespace admits (it ends in `.`) stays bracketed.
        let out = render_one(
            "http://example.org/s",
            "http://example.org/p",
            "http://example.org/o.",
            &prefixes,
        );
        assert!(
            out.contains("<http://example.org/o.>"),
            "no valid local, got:\n{out}"
        );
    }

    #[test]
    fn renders_rdf12_reifier_flat() {
        use crate::ir::builder::RdfDatasetBuilder;
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("https://ex/s");
        let p = b.intern_iri("https://ex/p");
        let o = b.intern_iri("https://ex/o");
        let r = b.intern_iri("https://ex/r");
        let conf = b.intern_iri("https://ex/conf");
        b.push_quad(s, p, o, None);
        let tt = b.intern_triple(s, p, o);
        b.push_reifier(r, tt);
        b.push_annotation(r, conf, o);
        let ds = b.freeze().unwrap();
        let out = render(&ds, &[("rdf".to_string(), RDF.to_string())]);
        // The reifier renders FLAT — the quoted triple inline, the annotation alongside
        // — never nested into an intermediate `[ rdf:reifies … ]` blank.
        assert!(
            out.contains("rdf:reifies <<( <https://ex/s> <https://ex/p> <https://ex/o> )>>"),
            "reifier must render the quoted triple flat, got:\n{out}"
        );
        assert!(
            out.contains("<https://ex/conf> <https://ex/o>"),
            "the annotation must render on the reifier, got:\n{out}"
        );
        assert!(
            !out.contains("reifies [\n") && !out.contains("reifies [ "),
            "reifier must NOT nest into a blank, got:\n{out}"
        );
    }

    #[test]
    fn pn_local_accepts_plain_names() {
        assert!(is_valid_pn_local("foo"));
        assert!(is_valid_pn_local("foo-bar_0"));
        assert!(is_valid_pn_local("a.b"));
        assert!(is_valid_pn_local("_foo"));
    }

    #[test]
    fn pn_local_rejects_invalid_names() {
        assert!(!is_valid_pn_local("")); // empty
        assert!(!is_valid_pn_local("foo.")); // trailing dot
        assert!(!is_valid_pn_local(".")); // lone dot
        assert!(!is_valid_pn_local("foo~bar")); // tilde
        assert!(!is_valid_pn_local("-foo")); // leading dash not allowed
        assert!(!is_valid_pn_local(".foo")); // leading dot not allowed
    }

    #[test]
    fn quote_escapes_control_chars_short_string() {
        // NUL escapes as \uXXXX; backspace, form feed, tab and CR take their ECHARs.
        assert_eq!(quote("a\u{0}b"), "\"a\\u0000b\"");
        assert_eq!(quote("a\u{8}b"), "\"a\\bb\"");
        assert_eq!(quote("a\u{c}b"), "\"a\\fb\"");
        assert_eq!(quote("a\tb"), "\"a\\tb\"");
        // The XML carrier escapes the C1 block and the two noncharacters.
        assert_eq!(quote("a\u{85}\u{FFFF}"), "\"a\\u0085\\uFFFF\"");
        assert_eq!(quote("plain caf\u{e9}"), "\"plain caf\u{e9}\"");
    }

    #[test]
    fn a_long_string_escapes_only_the_quotes_that_would_close_it() {
        // A trailing `"` would merge into the closing delimiter.
        assert_eq!(quote("a\nb\""), "\"\"\"a\nb\\\"\"\"\"");
        // A lone interior quote rides raw; a run escapes all but its last.
        assert_eq!(quote("a\n\"b\"\"\"c"), "\"\"\"a\n\"b\\\"\\\"\"c\"\"\"");
        // TAB and CR take their ECHARs, the line feed stays raw.
        assert_eq!(quote("a\tb\r\nc"), "\"\"\"a\\tb\\r\nc\"\"\"");
        assert_eq!(
            quote("line one\nline two"),
            "\"\"\"line one\nline two\"\"\""
        );
    }

    #[test]
    fn a_list_with_two_rests_is_not_collapsed_into_collection_syntax() {
        let rdf_iri = |local: &str| format!("{RDF}{local}");
        let render_list = |extra_rest: bool| {
            let mut b = crate::RdfDatasetBuilder::new();
            let s = b.intern_iri("https://ex/s");
            let p = b.intern_iri("https://ex/p");
            let cell = b.intern_blank("c", crate::BlankScope::DEFAULT);
            let first = b.intern_iri(&rdf_iri("first"));
            let rest = b.intern_iri(&rdf_iri("rest"));
            let nil = b.intern_iri(&rdf_iri("nil"));
            let member = b.intern_iri("https://ex/m");
            b.push_quad(s, p, cell, None);
            b.push_quad(cell, first, member, None);
            b.push_quad(cell, rest, nil, None);
            if extra_rest {
                let other = b.intern_iri("https://ex/other");
                b.push_quad(cell, rest, other, None);
            }
            render(&b.freeze().unwrap(), &[])
        };
        assert!(render_list(false).contains("<https://ex/p> ( <https://ex/m> ) ."));
        let broken = render_list(true);
        assert!(!broken.contains("( <https://ex/m> )"), "{broken}");
        assert!(broken.contains("<https://ex/other>"), "{broken}");
    }

    #[test]
    fn quote_escapes_control_chars_in_triple_quoted() {
        // A newline forces the triple-quoted branch; a NUL must still be escaped, and
        // the newline stays literal.
        let out = quote("a\nb\u{0}c");
        assert!(
            out.starts_with("\"\"\"") && out.ends_with("\"\"\""),
            "{out}"
        );
        assert!(out.contains('\n'), "{out}");
        assert!(out.contains("\\u0000"), "{out}");
    }
}

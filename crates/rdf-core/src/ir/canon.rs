// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Native **full W3C RDFC-1.0** RDF Dataset Canonicalization, oxigraph-free.
//!
//! This module is the canonicalization authority for the purrdf family. It
//! replaces `oxrdf`'s `Dataset::canonicalize` ( oxigraph eviction) and
//! supersedes the simplified FNV signature comparator that `compare.rs` used to
//! carry: it implements the real algorithm — *Hash First Degree Quads* (§4.6),
//! initial canonical assignment (§4.4), and *Hash N-Degree Quads* (§4.8) with
//! *Hash Related Blank Node* (§4.7) and **permutation backtracking** — so it
//! resolves blank-node automorphisms instead of conceding a false negative.
//!
//! ## What it produces
//!
//! [`canonicalize`] assigns every blank node a **stable canonical label**
//! (`c14n0`, `c14n1`, …) purely from graph structure and emits the **canonical
//! N-Quads** form (lines bytewise-sorted, deduplicated). Two datasets are
//! RDF-isomorphic **iff** their canonical N-Quads strings are byte-equal — an
//! exact oracle (no false positives *and* no false negatives), which is what
//! [`super::compare::datasets_isomorphic`] is rebuilt on.
//!
//! ## SUBSUME + EXTEND: the RDF-1.2 overlay
//!
//! RDFC-1.0 is specified over triples/quads. purrdf's IR additionally carries a
//! **reifier** overlay (`reifier → triple-term` bindings) and an **annotation**
//! overlay (`reifier, predicate, object`), plus quoted **triple terms**. This
//! implementation folds all three into both the hashing and the canonical output
//! by normalizing every statement into a quad shape, using reserved
//! `urn:purrdf:rdfc:` sentinel IRIs that no real dataset IRI can occupy:
//!
//! - reifier `(r, t)` → `r <urn:purrdf:rdfc:reifies> t .` (`t` is the triple term)
//! - annotation `(r, p, o)` → `r p o <urn:purrdf:rdfc:annotation> .`
//!
//! Because the sentinels are disjoint from genuine quads, the **reifier COUNT**
//! and **annotation presence** stay observable in the canonical form — preserving
//! the lossless identity contract (two datasets differing only in reifier
//! count or an annotation compare UNEQUAL). RDFC-1.0 canonicalizes blank labels
//! **only**: literal lexical forms, datatypes, language tags and base directions
//! are emitted verbatim (`0.70` ≠ `0.7`, `@en--ltr` ≠ `@en--rtl`).
//!
//! ## Termination (poison guard)
//!
//! The n-degree search is NP-hard in the worst case (pathologically symmetric
//! blank graphs). Per the project no-optionality / hard-fail rule there is no
//! knob: a fixed `RDFC_CALL_LIMIT` bounds recursion and the routine `panic!`s
//! with a diagnostic on exhaustion rather than degrading.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use sha2::{Digest, Sha256, Sha384};

use super::dataset::{RdfDataset, TermRef};
use super::term::TermId;

/// `xsd:string` — the implicit datatype that N-Quads writes bare (no `^^<…>`).
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
/// Sentinel predicate for a reifier binding in the canonical form (§ overlay).
const SENTINEL_REIFIES: &str = "urn:purrdf:rdfc:reifies";
/// Sentinel graph for an annotation row in the canonical form (§ overlay).
const SENTINEL_ANNOTATION_GRAPH: &str = "urn:purrdf:rdfc:annotation";
/// The canonical blank-label prefix (`c14n0`, `c14n1`, …) mandated by RDFC-1.0.
const CANON_PREFIX: &str = "c14n";
/// The temporary-issuer prefix used inside the n-degree search (RDFC-1.0 §4.5/4.8).
const TEMP_PREFIX: &str = "b";
/// The fixed recursion/permutation call budget for the n-degree search. Generous
/// for every non-adversarial dataset; exhaustion means a pathologically symmetric
/// blank graph and is a hard `panic!` (no knob, no degraded fallback — `.goals`).
const RDFC_CALL_LIMIT: u64 = 1_000_000;

/// The RDFC-1.0 hash algorithm. SHA-256 is the default; SHA-384 is the spec's
/// alternative (RDFC-1.0 §3, exercised by W3C suite `test075`). EXTEND beyond
/// `oxrdf`, which only offered SHA-256.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CanonHash {
    /// SHA-256 (the RDFC-1.0 default).
    Sha256,
    /// SHA-384.
    Sha384,
}

/// A digest rendered as fixed-capacity lowercase ASCII hex (`Copy`, so it sorts and
/// keys a `BTreeMap` without heap allocation). Holds SHA-256 (64 hex chars) or
/// SHA-384 (96 hex chars); within one canonicalization every hash shares an
/// algorithm, hence a length.
#[derive(Clone, Copy)]
struct HashHex {
    buf: [u8; 96],
    len: u8,
}

impl HashHex {
    /// The hex digits as `&str` (always valid ASCII hex by construction).
    #[inline]
    fn as_str(&self) -> &str {
        // SAFETY: bytes `[0, len)` are ASCII hex digits written by `hex_of`.
        unsafe { std::str::from_utf8_unchecked(&self.buf[..self.len as usize]) }
    }
}

impl PartialEq for HashHex {
    fn eq(&self, other: &Self) -> bool {
        self.as_str() == other.as_str()
    }
}
impl Eq for HashHex {}
impl PartialOrd for HashHex {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for HashHex {
    fn cmp(&self, other: &Self) -> Ordering {
        self.as_str().cmp(other.as_str())
    }
}

/// Lowercase-hex a raw digest (32 bytes for SHA-256, 48 for SHA-384) into a [`HashHex`].
fn hex_of(digest: &[u8]) -> HashHex {
    let mut buf = [0u8; 96];
    const LUT: &[u8; 16] = b"0123456789abcdef";
    for (i, byte) in digest.iter().enumerate() {
        buf[2 * i] = LUT[(byte >> 4) as usize];
        buf[2 * i + 1] = LUT[(byte & 0x0f) as usize];
    }
    HashHex {
        buf,
        len: (digest.len() * 2) as u8,
    }
}

/// Hash `bytes` under the selected algorithm, returning its lowercase hex.
fn digest_hex(hash: CanonHash, bytes: &[u8]) -> HashHex {
    match hash {
        CanonHash::Sha256 => hex_of(&Sha256::digest(bytes)),
        CanonHash::Sha384 => hex_of(&Sha384::digest(bytes)),
    }
}

/// Hash a sequence of already-serialized lines under the selected algorithm, feeding
/// each line into one running digest (Hash First Degree Quads, §4.6).
fn hash_lines(hash: CanonHash, lines: &[String]) -> HashHex {
    match hash {
        CanonHash::Sha256 => {
            let mut h = Sha256::new();
            for line in lines {
                h.update(line.as_bytes());
            }
            hex_of(&h.finalize())
        }
        CanonHash::Sha384 => {
            let mut h = Sha384::new();
            for line in lines {
                h.update(line.as_bytes());
            }
            hex_of(&h.finalize())
        }
    }
}

/// The result of canonicalizing a dataset.
#[derive(Clone, Debug)]
pub struct Canonicalized {
    /// The canonical N-Quads document: every line `'\n'`-terminated, the set of
    /// lines sorted bytewise ascending and deduplicated. Blanks render as their
    /// canonical `_:c14nN` label. Includes the reified/annotated overlay (via the
    /// reserved `urn:purrdf:rdfc:` sentinels).
    pub nquads: String,
    /// Each blank [`TermId`] mapped to its canonical label (`"c14n0"`, …) WITHOUT
    /// the leading `_:`.
    pub labels: BTreeMap<TermId, Box<str>>,
}

/// Canonicalize `ds` under full W3C RDFC-1.0 (SHA-256, extended for the RDF-1.2
/// overlay).
///
/// Deterministic and oxigraph-free. Hard-`panic!`s only if the n-degree search
/// exceeds `RDFC_CALL_LIMIT` on a pathologically symmetric blank graph.
#[must_use]
pub fn canonicalize(ds: &RdfDataset) -> Canonicalized {
    canonicalize_with(ds, CanonHash::Sha256)
}

/// Canonicalize `ds` under full W3C RDFC-1.0 with an explicit hash algorithm
/// ([`CanonHash::Sha384`] is the spec's SHA-384 variant). See [`canonicalize`].
#[must_use]
pub fn canonicalize_with(ds: &RdfDataset, hash: CanonHash) -> Canonicalized {
    CanonState::new(ds, hash).run()
}

/// The count of distinct blank nodes in `ds` (incl. blanks nested inside triple
/// terms). A cheap structural pre-reject used by [`super::compare`].
#[must_use]
pub fn blank_count(ds: &RdfDataset) -> usize {
    let mut set: BTreeSet<TermId> = BTreeSet::new();
    collect_components(ds, &mut |comp| {
        comp.for_each_blank(ds, &mut |b| {
            set.insert(b);
        });
    });
    set.len()
}

/// A statement normalized to a quad shape for uniform hashing and serialization.
/// Predicate/graph slots may be a reserved sentinel IRI (the overlay rows).
#[derive(Clone, Copy)]
enum Component {
    /// A genuine dataset quad.
    Quad {
        s: TermId,
        p: TermId,
        o: TermId,
        g: Option<TermId>,
    },
    /// A reifier binding `r <urn:purrdf:rdfc:reifies> t` in graph `g` (`None` =
    /// default graph — the graph slot then stays empty, byte-identical to the
    /// pre-graph-dimension form).
    Reifier {
        r: TermId,
        t: TermId,
        g: Option<TermId>,
    },
    /// An annotation `r p o` in the reserved annotation graph, itself scoped to graph
    /// `g` (`None` = default graph).
    Annotation {
        r: TermId,
        p: TermId,
        o: TermId,
        g: Option<TermId>,
    },
}

/// One quad slot: a dataset term, a synthetic sentinel IRI (overlay predicate) that
/// has no [`TermId`], or the annotation-overlay graph marker (the reserved annotation
/// sentinel plus the annotation's own named graph, if any).
#[derive(Clone, Copy)]
enum Slot {
    Term(TermId),
    Sentinel(&'static str),
    /// The annotation overlay's graph position: the reserved annotation sentinel and,
    /// for a named-graph annotation, the graph term. `None` renders exactly as the
    /// bare sentinel (byte-identical to the default-graph form); `Some(g)` appends the
    /// real graph term so a named-graph annotation stays lossless and distinct from a
    /// genuine quad. The graph term keeps its [`TermId`] so a blank-node graph still
    /// participates in canonical labeling.
    AnnotationGraph(Option<TermId>),
}

impl Component {
    /// The four quad slots `(s, p, o, g)` of this component in canonical shape.
    fn slots(self) -> (Slot, Slot, Slot, Option<Slot>) {
        match self {
            Self::Quad { s, p, o, g } => (
                Slot::Term(s),
                Slot::Term(p),
                Slot::Term(o),
                g.map(Slot::Term),
            ),
            Self::Reifier { r, t, g } => (
                Slot::Term(r),
                Slot::Sentinel(SENTINEL_REIFIES),
                Slot::Term(t),
                // The reifier's graph reuses the (previously always-empty) graph slot,
                // so a default-graph reifier (`g == None`) is byte-identical to before.
                g.map(Slot::Term),
            ),
            Self::Annotation { r, p, o, g } => (
                Slot::Term(r),
                Slot::Term(p),
                Slot::Term(o),
                Some(Slot::AnnotationGraph(g)),
            ),
        }
    }

    /// Invoke `f` for every blank [`TermId`] appearing anywhere in this component
    /// (recursing into triple terms).
    fn for_each_blank(self, ds: &RdfDataset, f: &mut impl FnMut(TermId)) {
        let (s, p, o, g) = self.slots();
        for slot in [Some(s), Some(p), Some(o), g].into_iter().flatten() {
            match slot {
                Slot::Term(id) | Slot::AnnotationGraph(Some(id)) => blanks_in_term(ds, id, f),
                Slot::Sentinel(_) | Slot::AnnotationGraph(None) => {}
            }
        }
    }
}

/// Invoke `f` for every blank [`TermId`] reachable at `id` (recursing triple terms).
fn blanks_in_term(ds: &RdfDataset, id: TermId, f: &mut impl FnMut(TermId)) {
    match ds.resolve(id) {
        TermRef::Blank { .. } => f(id),
        TermRef::Triple { s, p, o } => {
            blanks_in_term(ds, s, f);
            blanks_in_term(ds, p, f);
            blanks_in_term(ds, o, f);
        }
        _ => {}
    }
}

/// Drive `f` over every [`Component`] of the dataset (quads, reifiers, annotations).
fn collect_components(ds: &RdfDataset, f: &mut impl FnMut(Component)) {
    for q in ds.quads() {
        f(Component::Quad {
            s: q.s,
            p: q.p,
            o: q.o,
            g: q.g,
        });
    }
    for (r, t, g) in ds.reifiers_with_graph() {
        f(Component::Reifier { r, t, g });
    }
    for (r, p, o, g) in ds.annotations_with_graph() {
        f(Component::Annotation { r, p, o, g });
    }
}

/// How a blank renders during serialization.
#[derive(Clone, Copy)]
enum BlankRender<'a> {
    /// Hash First Degree Quads (§4.6): the focus blank → `_:a`, every other → `_:z`.
    FirstDegree { focus: TermId },
    /// Final output (§4.4 step 7): each blank → its issued `_:c14nN` label.
    Canonical { issuer: &'a IdIssuer },
}

impl BlankRender<'_> {
    /// The `_:`-less label a blank renders to under this strategy.
    fn label(&self, id: TermId) -> String {
        match self {
            BlankRender::FirstDegree { focus } => {
                if id == *focus {
                    "a".to_owned()
                } else {
                    "z".to_owned()
                }
            }
            BlankRender::Canonical { issuer } => issuer
                .issued_for(id)
                .expect("every blank has a canonical id at output time")
                .to_owned(),
        }
    }
}

/// The RDFC-1.0 "identifier issuer": mints prefixed ids (`c14n0`, `b0`, …) in a
/// stable order, remembering each blank's id and the issuance order.
#[derive(Clone)]
struct IdIssuer {
    prefix: &'static str,
    issued: BTreeMap<TermId, Box<str>>,
    order: Vec<TermId>,
}

impl IdIssuer {
    fn new(prefix: &'static str) -> Self {
        Self {
            prefix,
            issued: BTreeMap::new(),
            order: Vec::new(),
        }
    }

    /// Issue (or return the already-issued) id for `b`.
    fn issue(&mut self, b: TermId) -> &str {
        if !self.issued.contains_key(&b) {
            let id = format!("{}{}", self.prefix, self.order.len()).into_boxed_str();
            self.issued.insert(b, id);
            self.order.push(b);
        }
        self.issued.get(&b).expect("just inserted")
    }

    fn issued_for(&self, b: TermId) -> Option<&str> {
        self.issued.get(&b).map(Box::as_ref)
    }

    fn has(&self, b: TermId) -> bool {
        self.issued.contains_key(&b)
    }

    /// The blanks in issuance order.
    fn order(&self) -> &[TermId] {
        &self.order
    }
}

/// Per-dataset canonicalization state.
struct CanonState<'a> {
    ds: &'a RdfDataset,
    /// Every blank, in ascending [`TermId`] order (the deterministic reference set).
    blanks: Vec<TermId>,
    /// The components each blank participates in (its "quads", RDFC-1.0 §4.4).
    incident: BTreeMap<TermId, Vec<Component>>,
    /// First-degree hash (§4.6) of each blank, computed once.
    first_degree: BTreeMap<TermId, HashHex>,
    /// The durable canonical issuer.
    canonical: IdIssuer,
    /// The hash algorithm for this run (RDFC-1.0 §3).
    hash: CanonHash,
    /// Remaining recursion/permutation budget (poison guard).
    budget: u64,
}

/// Internal early-unwind carrier for the poison-budget guard.
struct BudgetExceeded;

impl<'a> CanonState<'a> {
    fn new(ds: &'a RdfDataset, hash: CanonHash) -> Self {
        let mut blank_set: BTreeSet<TermId> = BTreeSet::new();
        let mut incident: BTreeMap<TermId, Vec<Component>> = BTreeMap::new();
        collect_components(ds, &mut |comp| {
            // Record incidence for each distinct blank in the component (a blank that
            // appears in two positions of one quad still lists that quad once).
            let mut seen: BTreeSet<TermId> = BTreeSet::new();
            comp.for_each_blank(ds, &mut |b| {
                blank_set.insert(b);
                if seen.insert(b) {
                    incident.entry(b).or_default().push(comp);
                }
            });
        });
        let blanks: Vec<TermId> = blank_set.into_iter().collect();
        Self {
            ds,
            blanks,
            incident,
            first_degree: BTreeMap::new(),
            canonical: IdIssuer::new(CANON_PREFIX),
            hash,
            budget: RDFC_CALL_LIMIT,
        }
    }

    /// Run the full algorithm, panicking on poison-budget exhaustion.
    fn run(mut self) -> Canonicalized {
        match self.run_inner() {
            Ok(()) => {}
            Err(BudgetExceeded) => panic!(
                "RDFC-1.0 canonicalization exceeded its call budget ({RDFC_CALL_LIMIT}) on a \
                 pathologically symmetric blank graph ({} blanks); the input is adversarial and \
                 cannot be canonicalized deterministically within bounds",
                self.blanks.len()
            ),
        }
        let nquads = self.serialize_canonical();
        let labels = self
            .canonical
            .issued
            .iter()
            .map(|(&id, label)| (id, label.clone()))
            .collect();
        Canonicalized { nquads, labels }
    }

    fn run_inner(&mut self) -> Result<(), BudgetExceeded> {
        // §4.4 step 3: first-degree hash of every blank, grouped by hash.
        let mut by_hash: BTreeMap<HashHex, Vec<TermId>> = BTreeMap::new();
        for &b in &self.blanks {
            let h = self.hash_first_degree(b);
            self.first_degree.insert(b, h);
            by_hash.entry(h).or_default().push(b);
        }

        // §4.4 step 4: issue canonical ids to uniquely-hashed blanks, ascending hash.
        // Defer hash-colliding groups to the n-degree pass.
        let mut ambiguous: Vec<HashHex> = Vec::new();
        for (h, group) in &by_hash {
            if group.len() == 1 {
                self.canonical.issue(group[0]);
            } else {
                ambiguous.push(*h);
            }
        }

        // §4.4 step 5: resolve each ambiguous group via the n-degree search.
        for h in ambiguous {
            let group = by_hash.get(&h).expect("ambiguous hash present").clone();
            // 5.2–5.3: for each not-yet-canonical blank, run hashNDegreeQuads against a
            // fresh temporary issuer seeded with that blank.
            let mut hash_paths: Vec<(HashHex, IdIssuer)> = Vec::new();
            for b in group {
                if self.canonical.has(b) {
                    continue;
                }
                let mut temp = IdIssuer::new(TEMP_PREFIX);
                temp.issue(b);
                let (result_hash, result_issuer) = self.hash_n_degree(b, temp)?;
                hash_paths.push((result_hash, result_issuer));
            }
            // 5.5: promote the temp issuers' bindings into the canonical issuer, the
            // groups taken in ascending result-hash order, each issuer in its own
            // issuance order.
            hash_paths.sort_by_key(|(h, _)| *h);
            for (_h, issuer) in hash_paths {
                for &b in issuer.order() {
                    self.canonical.issue(b);
                }
            }
        }
        Ok(())
    }

    /// Hash First Degree Quads (RDFC-1.0 §4.6) for blank `b`.
    fn hash_first_degree(&self, b: TermId) -> HashHex {
        let render = BlankRender::FirstDegree { focus: b };
        let mut lines: Vec<String> = self
            .incident
            .get(&b)
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .map(|comp| {
                let mut s = String::new();
                self.write_component(*comp, render, &mut s);
                s
            })
            .collect();
        lines.sort_unstable();
        hash_lines(self.hash, &lines)
    }

    /// Hash N-Degree Quads (RDFC-1.0 §4.8): the gossip-path permutation search.
    fn hash_n_degree(
        &mut self,
        identifier: TermId,
        mut issuer: IdIssuer,
    ) -> Result<(HashHex, IdIssuer), BudgetExceeded> {
        self.budget = self.budget.checked_sub(1).ok_or(BudgetExceeded)?;

        // §4.8 step 3: map related-blank hash → the related blanks bearing it.
        let mut hn: BTreeMap<HashHex, Vec<TermId>> = BTreeMap::new();
        let components = self.incident.get(&identifier).cloned().unwrap_or_default();
        for comp in &components {
            self.related_blanks(*comp, identifier, &issuer, &mut |related, related_hash| {
                hn.entry(related_hash).or_default().push(related);
            });
        }

        let mut data_to_hash = String::new();
        // §4.8 step 5: for each related hash, ascending.
        for (related_hash, related_list) in &hn {
            data_to_hash.push_str(related_hash.as_str());
            let mut chosen_path: Option<String> = None;
            let mut chosen_issuer: Option<IdIssuer> = None;

            // §4.8 step 5.4: every permutation of the related list, identity first.
            for perm in permutations(related_list) {
                // Charge the poison budget PER PERMUTATION: a related group of size k
                // contributes k! permutations, so this — not the recursive-call count —
                // is the dominant cost on a pathologically symmetric graph (e.g. a
                // 10-blank clique). Counting it here bounds the actual work.
                self.budget = self.budget.checked_sub(1).ok_or(BudgetExceeded)?;
                let mut issuer_copy = issuer.clone();
                let mut path = String::new();
                let mut recursion: Vec<TermId> = Vec::new();
                let mut pruned = false;

                // 5.4.4
                for related in &perm {
                    if let Some(id) = self.canonical.issued_for(*related) {
                        path.push_str("_:");
                        path.push_str(id);
                    } else {
                        if !issuer_copy.has(*related) {
                            recursion.push(*related);
                        }
                        path.push_str("_:");
                        path.push_str(issuer_copy.issue(*related));
                    }
                    // 5.4.4.3: prune if this partial path can no longer win.
                    if let Some(best) = &chosen_path {
                        if path.len() >= best.len() && path.as_str() > best.as_str() {
                            pruned = true;
                            break;
                        }
                    }
                }
                if pruned {
                    continue;
                }

                // 5.4.5: recurse into newly-seen related blanks in path order.
                for related in &recursion {
                    let (rec_hash, rec_issuer) =
                        self.hash_n_degree(*related, issuer_copy.clone())?;
                    path.push_str("_:");
                    path.push_str(issuer_copy.issue(*related));
                    path.push('<');
                    path.push_str(rec_hash.as_str());
                    path.push('>');
                    issuer_copy = rec_issuer;
                    if let Some(best) = &chosen_path {
                        if path.len() >= best.len() && path.as_str() > best.as_str() {
                            pruned = true;
                            break;
                        }
                    }
                }
                if pruned {
                    continue;
                }

                // 5.4.6: keep the lexicographically least path (first wins ties).
                if chosen_path
                    .as_ref()
                    .is_none_or(|best| path.as_str() < best.as_str())
                {
                    chosen_path = Some(path);
                    chosen_issuer = Some(issuer_copy);
                }
            }

            // 5.5–5.6: fold the winning path and adopt its issuer.
            data_to_hash.push_str(chosen_path.as_deref().unwrap_or(""));
            if let Some(winner) = chosen_issuer {
                issuer = winner;
            }
        }

        Ok((digest_hex(self.hash, data_to_hash.as_bytes()), issuer))
    }

    /// §4.8 step 3 + §4.7: for each related blank of `comp` (other than `focus`),
    /// invoke `f(related, hash_related_blank_node(related, …))`.
    fn related_blanks(
        &self,
        comp: Component,
        focus: TermId,
        issuer: &IdIssuer,
        f: &mut impl FnMut(TermId, HashHex),
    ) {
        let (s, p, o, g) = comp.slots();
        // Standard quad positions whose blanks are "related": subject, object, graph.
        // (Predicates are always IRIs / sentinels — never blank.) Blanks nested
        // inside a triple-term slot recurse with a position-path tag (RDF-1.2 ext).
        self.related_in_slot(s, "s", &p, focus, issuer, f);
        self.related_in_slot(o, "o", &p, focus, issuer, f);
        if let Some(g) = g {
            self.related_in_slot(g, "g", &p, focus, issuer, f);
        }
    }

    /// Walk a slot for related blanks, recursing triple terms with a position path.
    fn related_in_slot(
        &self,
        slot: Slot,
        position: &str,
        predicate: &Slot,
        focus: TermId,
        issuer: &IdIssuer,
        f: &mut impl FnMut(TermId, HashHex),
    ) {
        // The annotation-overlay graph marker carries a real graph term whose blanks
        // are "related" exactly like any graph-slot term.
        let id = match slot {
            Slot::Term(id) | Slot::AnnotationGraph(Some(id)) => id,
            Slot::Sentinel(_) | Slot::AnnotationGraph(None) => return,
        };
        match self.ds.resolve(id) {
            TermRef::Blank { .. } => {
                if id != focus {
                    let h = self.hash_related_blank_node(id, position, predicate, issuer);
                    f(id, h);
                }
            }
            TermRef::Triple { s, p, o } => {
                // Nested-triple blanks get a position path so role inside the quoted
                // triple is distinguished (RDF-1.2 extension; never hit by the W3C suite).
                self.related_in_slot(
                    Slot::Term(s),
                    &format!("{position}.s"),
                    predicate,
                    focus,
                    issuer,
                    f,
                );
                self.related_in_slot(
                    Slot::Term(p),
                    &format!("{position}.p"),
                    predicate,
                    focus,
                    issuer,
                    f,
                );
                self.related_in_slot(
                    Slot::Term(o),
                    &format!("{position}.o"),
                    predicate,
                    focus,
                    issuer,
                    f,
                );
            }
            _ => {}
        }
    }

    /// Hash Related Blank Node (RDFC-1.0 §4.7).
    fn hash_related_blank_node(
        &self,
        related: TermId,
        position: &str,
        predicate: &Slot,
        issuer: &IdIssuer,
    ) -> HashHex {
        let mut input = String::new();
        input.push_str(position);
        if position != "g" && !position.starts_with("g.") {
            input.push('<');
            input.push_str(&self.predicate_iri(predicate));
            input.push('>');
        }
        if let Some(id) = self.canonical.issued_for(related) {
            input.push_str("_:");
            input.push_str(id);
        } else if let Some(id) = issuer.issued_for(related) {
            input.push_str("_:");
            input.push_str(id);
        } else {
            input.push_str(self.first_degree[&related].as_str());
        }
        digest_hex(self.hash, input.as_bytes())
    }

    /// The IRI value of a predicate slot (a real IRI term or a sentinel).
    fn predicate_iri(&self, predicate: &Slot) -> String {
        match predicate {
            Slot::Sentinel(iri) => (*iri).to_owned(),
            Slot::Term(id) => match self.ds.resolve(*id) {
                TermRef::Iri(iri) => iri.to_owned(),
                other => unreachable!("predicate must be an IRI, got {other:?}"),
            },
            Slot::AnnotationGraph(_) => {
                unreachable!("the annotation-graph marker is never a predicate slot")
            }
        }
    }

    /// §4.4 step 7: serialize every component with canonical labels, sorted + deduped.
    fn serialize_canonical(&self) -> String {
        let render = BlankRender::Canonical {
            issuer: &self.canonical,
        };
        let mut lines: BTreeSet<String> = BTreeSet::new();
        collect_components(self.ds, &mut |comp| {
            let mut s = String::new();
            self.write_component(comp, render, &mut s);
            lines.insert(s);
        });
        let mut out = String::new();
        for line in &lines {
            out.push_str(line);
        }
        out
    }

    /// Write one component as a canonical N-Quads line (`s p o [g] .\n`).
    fn write_component(&self, comp: Component, render: BlankRender<'_>, out: &mut String) {
        let (s, p, o, g) = comp.slots();
        self.write_slot(s, render, out);
        out.push(' ');
        self.write_slot(p, render, out);
        out.push(' ');
        self.write_slot(o, render, out);
        if let Some(g) = g {
            out.push(' ');
            self.write_slot(g, render, out);
        }
        out.push_str(" .\n");
    }

    fn write_slot(&self, slot: Slot, render: BlankRender<'_>, out: &mut String) {
        match slot {
            Slot::Sentinel(iri) => {
                out.push('<');
                write_iri_escaped(iri, out);
                out.push('>');
            }
            Slot::Term(id) => self.write_term(id, render, out),
            Slot::AnnotationGraph(g) => {
                // `None`: bare annotation sentinel — byte-identical to the pre-graph
                // form. `Some(g)`: sentinel then the graph term, so a named-graph
                // annotation stays lossless and never collides with a genuine quad
                // (which never carries two graph tokens). Not re-parsed — this string
                // is only hashed / byte-compared as the canonical oracle.
                out.push('<');
                write_iri_escaped(SENTINEL_ANNOTATION_GRAPH, out);
                out.push('>');
                if let Some(g) = g {
                    out.push(' ');
                    self.write_term(g, render, out);
                }
            }
        }
    }

    /// Write a term in canonical N-Quads form. Literal lexical forms / datatypes /
    /// language / direction are emitted **verbatim** (never normalized).
    fn write_term(&self, id: TermId, render: BlankRender<'_>, out: &mut String) {
        match self.ds.resolve(id) {
            TermRef::Iri(iri) => {
                out.push('<');
                write_iri_escaped(iri, out);
                out.push('>');
            }
            TermRef::Blank { .. } => {
                out.push_str("_:");
                out.push_str(&render.label(id));
            }
            TermRef::Literal {
                lexical,
                datatype,
                language,
                direction,
            } => {
                out.push('"');
                write_literal_escaped(lexical, out);
                out.push('"');
                if let Some(lang) = language {
                    out.push('@');
                    out.push_str(lang);
                    if let Some(dir) = direction {
                        out.push_str("--");
                        out.push_str(dir.as_str());
                    }
                } else {
                    let dt = match self.ds.resolve(datatype) {
                        TermRef::Iri(iri) => iri,
                        other => unreachable!("literal datatype must be an IRI, got {other:?}"),
                    };
                    if dt != XSD_STRING {
                        out.push_str("^^<");
                        write_iri_escaped(dt, out);
                        out.push('>');
                    }
                }
            }
            TermRef::Triple { s, p, o } => {
                // RDF-1.2 triple term: `<<( <s> <p> <o> )>>` (the form oxigraph/Jena parse).
                out.push_str("<<( ");
                self.write_term(s, render, out);
                out.push(' ');
                self.write_term(p, render, out);
                out.push(' ');
                self.write_term(o, render, out);
                out.push_str(" )>>");
            }
        }
    }
}

/// A **lazy** generator of every permutation of a slice (identity first, then
/// lexicographic position order). Lazy generation matters for the poison case: a
/// 9-element related group has 9! = 362 880 permutations, so collecting them all
/// upfront would allocate a factorial-sized `Vec<Vec<_>>` per n-degree call. Yielding
/// one small `Vec` at a time keeps the call-budget guard the only bound on cost.
struct Permutations<T> {
    items: Vec<T>,
    idx: Vec<usize>,
    first: bool,
    done: bool,
}

impl<T: Copy> Iterator for Permutations<T> {
    type Item = Vec<T>;

    fn next(&mut self) -> Option<Vec<T>> {
        if self.done {
            return None;
        }
        if self.first {
            self.first = false;
        } else if !next_permutation(&mut self.idx) {
            self.done = true;
            return None;
        }
        Some(self.idx.iter().map(|&i| self.items[i]).collect())
    }
}

/// Lazily generate every permutation of `items` (identity first; see [`Permutations`]).
fn permutations<T: Copy>(items: &[T]) -> Permutations<T> {
    Permutations {
        items: items.to_vec(),
        idx: (0..items.len()).collect(),
        first: true,
        // An empty slice still yields exactly one (empty) permutation.
        done: false,
    }
}

/// In-place next lexicographic permutation of `a`; `false` if `a` was the last.
fn next_permutation(a: &mut [usize]) -> bool {
    let n = a.len();
    if n < 2 {
        return false;
    }
    let mut i = n - 1;
    while i > 0 && a[i - 1] >= a[i] {
        i -= 1;
    }
    if i == 0 {
        return false;
    }
    let mut j = n - 1;
    while a[j] <= a[i - 1] {
        j -= 1;
    }
    a.swap(i - 1, j);
    a[i..].reverse();
    true
}

/// Escape an IRI for `<…>` N-Quads form: control chars (C0, the space character, DEL,
/// and the C1 block `0x80-0x9F`) and the reserved delimiter set become `\uXXXX`
/// (canonical N-Triples IRIREF rules). Clean ASCII IRIs pass through unchanged.
fn write_iri_escaped(iri: &str, out: &mut String) {
    for ch in iri.chars() {
        match ch {
            c if c.is_control() || c == ' ' => write_u_escape(c, out),
            '<' | '>' | '"' | '{' | '}' | '|' | '^' | '`' | '\\' => {
                write_u_escape(ch, out);
            }
            _ => out.push(ch),
        }
    }
}

/// Escape a literal lexical form for a `"…"` N-Quads string, matching the canonical
/// N-Triples ECHAR set; other C0 control characters become `\uXXXX`.
fn write_literal_escaped(value: &str, out: &mut String) {
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            // Canonical N-Quads escapes C0 controls and U+007F (DEL) as \uXXXX; every
            // other character (incl. all non-ASCII, including the C1 block) is emitted
            // verbatim as UTF-8 — the W3C RDFC-1.0 test suite fixtures (e.g. test060)
            // pin the C1 block passing through raw in literals, unlike IRIs where the
            // IRIREF grammar forbids the full control range.
            c if (c as u32) < 0x20 || c as u32 == 0x7f => write_u_escape(c, out),
            c => out.push(c),
        }
    }
}

/// Write `\uXXXX` (or `\UXXXXXXXX` beyond the BMP) for `ch`.
fn write_u_escape(ch: char, out: &mut String) {
    let cp = ch as u32;
    if cp <= 0xFFFF {
        let _ = write!(out, "\\u{cp:04X}");
    } else {
        let _ = write!(out, "\\U{cp:08X}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::RdfDatasetBuilder;
    use crate::{RdfLiteral, RdfTextDirection};
    use std::sync::Arc;

    fn iri(b: &mut RdfDatasetBuilder, n: &str) -> TermId {
        b.intern_iri(&format!("http://example.org/{n}"))
    }

    fn canon(ds: &RdfDataset) -> String {
        canonicalize(ds).nquads
    }

    #[test]
    fn all_ground_fast_path_sorts_quads() {
        let mut b = RdfDatasetBuilder::new();
        let (s, p, o1, o2) = (
            iri(&mut b, "s"),
            iri(&mut b, "p"),
            iri(&mut b, "o1"),
            iri(&mut b, "o2"),
        );
        b.push_quad(s, p, o2, None);
        b.push_quad(s, p, o1, None);
        let ds = b.freeze().expect("valid");
        assert_eq!(
            canon(&ds),
            "<http://example.org/s> <http://example.org/p> <http://example.org/o1> .\n\
             <http://example.org/s> <http://example.org/p> <http://example.org/o2> .\n"
        );
        assert!(canonicalize(&ds).labels.is_empty(), "no blanks → no labels");
    }

    #[test]
    fn empty_dataset_canonicalizes_to_empty() {
        let ds = RdfDatasetBuilder::new().freeze().expect("valid");
        assert_eq!(canon(&ds), "");
    }

    #[test]
    fn literal_forms_are_verbatim() {
        let mut b = RdfDatasetBuilder::new();
        let (s, p) = (iri(&mut b, "s"), iri(&mut b, "p"));
        // A typed literal whose lexical form MUST NOT be normalized (0.70 != 0.7).
        let lit = b.intern_literal(RdfLiteral::typed(
            "0.70",
            "http://www.w3.org/2001/XMLSchema#decimal",
        ));
        b.push_quad(s, p, lit, None);
        let ds = b.freeze().expect("valid");
        assert!(
            canon(&ds).contains("\"0.70\"^^<http://www.w3.org/2001/XMLSchema#decimal>"),
            "lexical form preserved: {}",
            canon(&ds)
        );
    }

    #[test]
    fn xsd_string_is_bare_and_directional_literal_renders() {
        let mut b = RdfDatasetBuilder::new();
        let (s, p, q) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "q"));
        let plain = b.intern_literal(RdfLiteral::simple("hi"));
        let rtl = b.intern_literal(RdfLiteral {
            lexical_form: "مرحبا".to_owned(),
            datatype: None,
            language: Some("ar".to_owned()),
            direction: Some(RdfTextDirection::Rtl),
        });
        b.push_quad(s, p, plain, None);
        b.push_quad(s, q, rtl, None);
        let out = canon(&ds_of(b));
        assert!(out.contains("\"hi\" ."), "xsd:string bare: {out}");
        assert!(
            out.contains("\"مرحبا\"@ar--rtl ."),
            "directional literal: {out}"
        );
    }

    fn ds_of(b: RdfDatasetBuilder) -> Arc<RdfDataset> {
        b.freeze().expect("valid")
    }

    #[test]
    fn isomorphic_blank_relabeling_is_byte_equal() {
        use super::super::term::BlankScope;
        let build = |l: &str, scope: u32| -> Arc<RdfDataset> {
            let mut b = RdfDatasetBuilder::new();
            let p = iri(&mut b, "p");
            let o = iri(&mut b, "o");
            let blank = b.intern_blank(l, BlankScope(scope));
            b.push_quad(blank, p, o, None);
            b.freeze().expect("valid")
        };
        let a = build("x", 0);
        let c = build("totally-different", 9);
        assert_eq!(
            canon(&a),
            canon(&c),
            "blank label/scope must not affect canon"
        );
    }

    /// The symmetric two-blank ring the OLD FNV comparator false-negatived: now it
    /// canonicalizes deterministically and two relabelings are byte-equal.
    #[test]
    fn symmetric_ring_resolves_deterministically() {
        use super::super::term::BlankScope;
        let build = |l1: &str, l2: &str| -> Arc<RdfDataset> {
            let mut b = RdfDatasetBuilder::new();
            let (p, q) = (iri(&mut b, "p"), iri(&mut b, "q"));
            let x = b.intern_blank(l1, BlankScope(0));
            let y = b.intern_blank(l2, BlankScope(0));
            b.push_quad(x, p, y, None);
            b.push_quad(y, q, x, None);
            b.freeze().expect("valid")
        };
        let a = build("x", "y");
        let c = build("m", "n");
        let ca = canon(&a);
        assert_eq!(
            ca,
            canon(&c),
            "relabeled ring must canonicalize identically"
        );
        assert!(
            ca.contains("_:c14n0") && ca.contains("_:c14n1"),
            "stable labels: {ca}"
        );
    }

    #[test]
    fn self_loop_canonicalizes() {
        use super::super::term::BlankScope;
        let mut b = RdfDatasetBuilder::new();
        let p = iri(&mut b, "p");
        let x = b.intern_blank("x", BlankScope::DEFAULT);
        b.push_quad(x, p, x, None);
        let ds = b.freeze().expect("valid");
        assert_eq!(canon(&ds), "_:c14n0 <http://example.org/p> _:c14n0 .\n");
    }

    /// Differently-wired blank graphs must NOT be byte-equal.
    #[test]
    fn different_wiring_differs() {
        use super::super::term::BlankScope;
        let build = |neighbour: &str| -> Arc<RdfDataset> {
            let mut b = RdfDatasetBuilder::new();
            let (p, link, s) = (iri(&mut b, "p"), iri(&mut b, "link"), iri(&mut b, "s"));
            let blank = b.intern_blank("b", BlankScope::DEFAULT);
            let nb = iri(&mut b, neighbour);
            b.push_quad(s, p, blank, None);
            b.push_quad(blank, link, nb, None);
            b.freeze().expect("valid")
        };
        assert_ne!(canon(&build("o1")), canon(&build("o2")));
    }

    /// Reifier COUNT is observable in the canonical form (the headline gate).
    #[test]
    fn reifier_count_shows_in_canon() {
        let build = |reifiers: &[&str]| -> Arc<RdfDataset> {
            let mut b = RdfDatasetBuilder::new();
            let (s, p, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
            let triple = b.intern_triple(s, p, o);
            b.push_quad(s, p, o, None);
            for r in reifiers {
                let rid = iri(&mut b, r);
                b.push_reifier(rid, triple);
            }
            b.freeze().expect("valid")
        };
        let one = canon(&build(&["r1"]));
        let two = canon(&build(&["r1", "r2"]));
        assert_ne!(one, two, "two reifiers must differ from one");
        assert!(
            one.contains("<urn:purrdf:rdfc:reifies> <<( "),
            "reifier sentinel: {one}"
        );
        assert!(
            two.contains(
                "<<( <http://example.org/s> <http://example.org/p> <http://example.org/o> )>>"
            ),
            "triple term rendered: {two}"
        );
    }

    /// Annotation presence is observable.
    #[test]
    fn annotation_shows_in_canon() {
        let build = |annotated: bool| -> Arc<RdfDataset> {
            let mut b = RdfDatasetBuilder::new();
            let (s, p, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
            let triple = b.intern_triple(s, p, o);
            let r = iri(&mut b, "r");
            b.push_quad(s, p, o, None);
            b.push_reifier(r, triple);
            if annotated {
                let (ap, ao) = (iri(&mut b, "ap"), iri(&mut b, "ao"));
                b.push_annotation(r, ap, ao);
            }
            b.freeze().expect("valid")
        };
        let with = canon(&build(true));
        let without = canon(&build(false));
        assert_ne!(with, without);
        assert!(
            with.contains("<urn:purrdf:rdfc:annotation> ."),
            "annotation graph sentinel: {with}"
        );
    }

    #[test]
    fn blank_count_counts_distinct_including_nested() {
        use super::super::term::BlankScope;
        let mut b = RdfDatasetBuilder::new();
        let p = iri(&mut b, "p");
        let x = b.intern_blank("x", BlankScope::DEFAULT);
        let y = b.intern_blank("y", BlankScope::DEFAULT);
        b.push_quad(x, p, y, None);
        b.push_quad(y, p, x, None);
        let ds = b.freeze().expect("valid");
        assert_eq!(blank_count(&ds), 2);
    }

    #[test]
    fn permutations_are_lexicographic_identity_first() {
        let perms: Vec<Vec<u32>> = permutations(&[10u32, 20, 30]).collect();
        assert_eq!(perms.len(), 6);
        assert_eq!(perms[0], vec![10, 20, 30], "identity first");
        assert_eq!(perms[5], vec![30, 20, 10], "reverse last");
        // A single-element slice yields exactly one permutation.
        assert_eq!(permutations(&[7u32]).count(), 1);
    }

    /// purrdf-EXT n-degree path: a symmetric blank pair reachable ONLY through
    /// quoted triple-term slots — the `.s`/`.o` position paths in
    /// [`CanonState::related_in_slot`] that the W3C suite never exercises. The
    /// automorphism must resolve deterministically (two relabelings byte-equal),
    /// and an asymmetric sibling must canonicalize differently.
    #[test]
    fn nested_triple_term_symmetry_resolves_deterministically() {
        use super::super::term::BlankScope;
        // <base> <ref> <<( x <link> y )>> .
        // <base> <ref> <<( y <link> x )>> .   — symmetric under x<->y, the symmetry
        // mediated entirely by blanks nested inside triple terms (no top-level blank
        // edge), so resolving it forces the triple-term-recursing n-degree search.
        let build = |l1: &str, l2: &str| -> Arc<RdfDataset> {
            let mut b = RdfDatasetBuilder::new();
            let (base, refp, link) = (iri(&mut b, "base"), iri(&mut b, "ref"), iri(&mut b, "link"));
            let x = b.intern_blank(l1, BlankScope(0));
            let y = b.intern_blank(l2, BlankScope(0));
            let t1 = b.intern_triple(x, link, y);
            let t2 = b.intern_triple(y, link, x);
            b.push_quad(base, refp, t1, None);
            b.push_quad(base, refp, t2, None);
            b.freeze().expect("valid")
        };
        let ca = canon(&build("x", "y"));
        assert_eq!(
            ca,
            canon(&build("m", "n")),
            "nested-triple-term automorphism must canonicalize identically regardless of input labels"
        );
        assert!(
            ca.contains("_:c14n0") && ca.contains("_:c14n1"),
            "two stable nested blank labels: {ca}"
        );
        assert!(ca.contains("<<("), "triple terms rendered: {ca}");

        // Break the symmetry: give x one extra ground edge nested in a triple term.
        // x and y are no longer automorphic, so the canon output must differ.
        let asym = {
            let mut b = RdfDatasetBuilder::new();
            let (base, refp, link, tag) = (
                iri(&mut b, "base"),
                iri(&mut b, "ref"),
                iri(&mut b, "link"),
                iri(&mut b, "tag"),
            );
            let x = b.intern_blank("x", BlankScope(0));
            let y = b.intern_blank("y", BlankScope(0));
            let t1 = b.intern_triple(x, link, y);
            let t2 = b.intern_triple(y, link, x);
            let t3 = b.intern_triple(x, link, tag);
            b.push_quad(base, refp, t1, None);
            b.push_quad(base, refp, t2, None);
            b.push_quad(base, refp, t3, None);
            b.freeze().expect("valid")
        };
        assert_ne!(
            ca,
            canon(&asym),
            "an asymmetric nested-triple graph must not canonicalize to the symmetric one"
        );
    }
}

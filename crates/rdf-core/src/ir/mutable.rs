// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The copy-on-write, suppression-delta **mutable dataset**.
//!
//! A [`MutableDataset`] branches cheaply off a shared, frozen
//! [`Arc<RdfDataset>`](RdfDataset) base and records mutations as an *append delta*
//! plus a *suppression set* — GTS's append+suppression model held in memory. The
//! effective contents are
//!
//! ```text
//! effective = (base ∪ added) − suppressed
//! ```
//!
//! A named graph is part of the effective dataset while it holds an effective row,
//! or while it is a graph the base declared empty that no mutation has emptied
//! since (removing its last row or [withdrawing its
//! declaration](MutableDataset::withdraw_graph_declaration)).
//! [`GraphExistenceMode::RememberEmpty`] instead retains a named graph's slot when
//! its last row is removed; explicit declaration withdrawal still removes it.
//!
//! [`MutableDataset::freeze`] is the **compaction** pass that re-interns the
//! effective set (terms, reifiers, annotations, graph names, locations) into a fresh
//! frozen [`RdfDataset`] through the existing [`RdfDatasetBuilder`].
//!
//! # Term identity — TAGGED handles, never a numeric threshold
//!
//! A plain two-tier numeric `TermId` would break the load-bearing invariant that a
//! [`TermId`] belongs to exactly ONE frozen dataset (C0.8). So the mutable layer
//! never widens `TermId`; instead it works in `MutTermId`, a tagged enum of
//! `Base(TermId)` (an id into the frozen base) and `Delta(DeltaTermId)` (an index
//! into the delta's own small interner). When a quad mentions a term, the layer asks
//! the base `term_id_by_value(&TermValue)`: a hit binds `Base`, a miss mints a
//! `Delta`. `MutTermId`/`DeltaTermId` are strictly INTERNAL — the outside world only
//! ever sees frozen base `TermId`s (pre-mutation) or post-`freeze()` dense `TermId`s.
//!
//! # The four mutation rules (explicit + unit-tested)
//!
//! 1. insert of a currently-SUPPRESSED *base* quad → **un-suppresses** it (removes
//!    it from `suppressed`), and does NOT also add it to `added`.
//! 2. remove of a *delta-added* quad → **drops it from `added`**, and does NOT
//!    create a suppression.
//! 3. remove of a *base* quad (not in `added`) → **creates a suppression**.
//! 4. reinsert-after-removal is consistent with both orders (insert→remove→insert
//!    and remove→insert→… both return to "present").

use crate::TermBox;
use std::convert::Infallible;
use std::ops::ControlFlow;
use std::sync::Arc;

use purrdf_iri::IriError;

use crate::backend::TermFactory as _;
use crate::dataset_view::{DatasetMut, GraphMatch, GraphMatchValue};
use crate::hash::{FastMap, FastSet};
use crate::ir::{RdfDataset, RdfDatasetBuilder, TermValue};

use super::dataset::{QuadHandle, TermRef};
use super::term::TermId;
use super::term_walk::{Nested, try_fold_nested};

mod delta_view;
pub use delta_view::{DeltaDatasetView, DeltaViewId};

/// The lifetime of named-graph declarations in a mutable branch.
///
/// Frozen datasets carry graph presence, never this caller-selected policy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum GraphExistenceMode {
    /// Preserve the current row-based graph lifetime: removing a graph's last row
    /// withdraws its declaration. An explicitly declared empty graph survives
    /// until a mutation or explicit withdrawal empties it.
    #[default]
    Implicit,
    /// Remember named-graph slots independently of rows, until explicit withdrawal.
    RememberEmpty,
}

/// The `rdf:reifies` predicate IRI — mirrors [`super::dataset`]'s private copy (kept
/// local rather than exported: both classify the SAME fold, independently, from a
/// value/id they already hold). A delta-added row shaped `_ rdf:reifies <<( … )>>`
/// is the RDF 1.2 reifier declaration `freeze` folds out of the flat quad delta (see
/// [`MutableDataset::freeze`]).
use purrdf_iri::vocab::rdf::REIFIES as RDF_REIFIES;

/// A dense index into a [`MutableDataset`]'s OWN delta term interner. Newtype (not a
/// bare `u32`) so it can never be confused with a base [`TermId`]; only ever wrapped
/// inside [`MutTermId::Delta`] and never observed outside the mutable layer.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub(crate) struct DeltaTermId(u32);

impl DeltaTermId {
    #[inline]
    fn index(self) -> usize {
        self.0 as usize
    }
}

/// A term identity in the mutable layer: either an id into the frozen base, or an id
/// minted in the delta interner. The TAGGED form (not a numeric threshold) preserves
/// the C0.8 invariant that a `TermId` belongs to ONE frozen dataset — a `Base` id is
/// always a valid index into `base`, a `Delta` id into the delta interner.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub(crate) enum MutTermId {
    /// A term already present in the frozen base, by its base id.
    Base(TermId),
    /// A brand-new term minted in the delta interner.
    Delta(DeltaTermId),
}

/// The canonical, hashable key of one effective quad in [`MutTermId`] space. Used
/// both as the membership key of the `suppressed` set and as the dedup key of the
/// `added` set, so the two layers speak the same identity language. `g == None`
/// names the default graph.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct QuadKey {
    pub s: MutTermId,
    pub p: MutTermId,
    pub o: MutTermId,
    pub g: Option<MutTermId>,
}

/// The delta's own small term interner. Terms not found in the base are minted here,
/// each yielding a [`DeltaTermId`]. Stores fully-resolved [`TermValue`]s by value (a
/// delta triple term is a `TermValue::Triple` holding its components by value, so no
/// `MutTermId` recursion is needed to read one back). Kept INTERNAL to the mutable
/// layer.
#[derive(Default, Debug)]
struct DeltaBuilder {
    /// The sole owner of each delta term value, in mint order.
    values: Vec<TermValue>,
    /// Reverse hash→id index: keyed by a hash of the term VALUE with
    /// `Vec<DeltaTermId>` collision buckets, so interning and lookup are O(1) expected
    /// instead of a linear scan. The hash only chooses a bucket and is never
    /// persisted; equal values always meet in one bucket and are then compared by
    /// `==`, so the hasher's choice affects speed, never results.
    index: FastMap<u64, Vec<DeltaTermId>>,
}

impl DeltaBuilder {
    /// Intern a delta term BY VALUE, returning its [`DeltaTermId`]. Idempotent: equal
    /// values dedup to one id (probe the hash bucket, compare candidates by `==`).
    ///
    /// # The absoluteness invariant is checked here, on the MISS path only
    ///
    /// The bucket probe below is the HIT path and returns before any validation: a
    /// value this delta already holds was checked when it was first minted. Only a
    /// genuinely new value pays the parse, exactly as the frozen IR's builder does
    /// (see [`super::absolute`]).
    ///
    /// # Errors
    ///
    /// [`IriError`] when the value carries a non-absolute IRI — its own, a literal's
    /// datatype, or one nested in a triple term.
    fn intern(&mut self, value: TermValue) -> Result<DeltaTermId, IriError> {
        let h = crate::hash::hash_of(&value);
        // Probe read-only (as `find` does): a hash miss must not mint an empty
        // bucket — least of all for a value that then fails the check below.
        if let Some(bucket) = self.index.get(&h) {
            for &did in bucket {
                if self.values[did.index()] == value {
                    return Ok(did);
                }
            }
        }
        // Miss: this value is entering the delta for the first time, so it is owed the
        // check.
        check_value_absolute(&value)?;
        let bucket = self.index.entry(h).or_default();
        let i = u32::try_from(self.values.len()).expect("delta term table exceeds u32::MAX");
        let did = DeltaTermId(i);
        bucket.push(did);
        self.values.push(value);
        Ok(did)
    }

    /// Find an already-interned [`TermValue`] WITHOUT minting; `None` if absent.
    fn find(&self, value: &TermValue) -> Option<DeltaTermId> {
        let bucket = self.index.get(&crate::hash::hash_of(value))?;
        bucket
            .iter()
            .copied()
            .find(|&did| self.values[did.index()] == *value)
    }

    #[inline]
    fn value(&self, id: DeltaTermId) -> &TermValue {
        &self.values[id.index()]
    }
}

/// A copy-on-write mutable RDF dataset. Branches cheaply off a
/// shared frozen base; records mutations as an append delta + a suppression set; and
/// compacts back to a frozen [`RdfDataset`] via [`freeze`](Self::freeze).
///
/// Many `MutableDataset`s may branch off ONE shared `base: Arc<RdfDataset>` (a clone
/// of the `Arc`), mutate independently, and never disturb each other or the base —
/// branching invalidates no externally-visible handle.
#[derive(Debug)]
pub struct MutableDataset {
    /// The shared, immutable COW base. Cloning the `Arc` is the cheap branch.
    base: Arc<RdfDataset>,
    graph_existence: GraphExistenceMode,
    /// The delta's own term interner (mints `Delta` ids for brand-new terms).
    delta: DeltaBuilder,
    /// Quads added on top of the base, in [`MutTermId`] space, deduplicated by value.
    /// Fixed-key hashed (never a source of process-random iteration order); the
    /// insertion SEQUENCE a caller actually cares about is tracked separately in
    /// `added_ord`, since — per the workspace's hash-determinism policy — hash
    /// iteration order must never be observed, even a fixed-key one built from a
    /// different insertion sequence than some other equal-content set.
    added: FastSet<QuadKey>,
    /// The insertion ordinal of each key currently in `added`, so `freeze()` and
    /// `effective_keys()` can replay delta-added quads in the order the caller
    /// actually added them — never in `added`'s hash-iteration order (see
    /// [`Self::added_in_order`]). A key present in `added` is always present here
    /// with the SAME ordinal it was (re)inserted at; removed keys are dropped from
    /// both maps together.
    added_ord: FastMap<QuadKey, u64>,
    /// The next ordinal `insert_key` will hand out to a brand-new `added` entry.
    /// Monotonically increasing for the lifetime of this `MutableDataset` — never
    /// reused, even across a remove/reinsert of the same key.
    next_added_ord: u64,
    /// Base quads suppressed (logically removed). A base quad is effective iff it is
    /// NOT in this set. Fixed-key hashed; only ever probed by membership, never
    /// iterated for order.
    suppressed: FastSet<QuadKey>,
    suppressed_rows: usize,
    /// Named graphs declared on top of the base, in declaration order, deduplicated:
    /// each survives [`freeze`](Self::freeze) and every snapshot as a declared graph
    /// whether or not it owns a quad.
    declared_graphs: Vec<TermValue>,
    /// The live RDF row count — quads, reifier bindings and annotations, base and
    /// added — of every named graph a mutation has touched, base-named or
    /// delta-named. A base graph is seeded from the base on first touch
    /// ([`RdfDataset::named_graph_row_count`]); a delta graph holds no base row, and
    /// its first touch is the insert of its first row, so it is seeded with zero.
    /// Kept exact in O(1) by every insert, removal, suppression and un-suppression
    /// after it, so deciding whether a removal emptied a graph never scans.
    graph_rows: FastMap<MutTermId, usize>,
    /// Base graphs a mutation emptied — removed their last row, or withdrew their
    /// declaration while they held none — and that hold no row now. Publication
    /// shares it with each snapshot, which withholds these graphs from named-graph
    /// enumeration (see [`DeltaDatasetView`]). Kept current by the row counts, so
    /// publication never scans for it; copied only when it changes after a
    /// snapshot took it. Probed only; never iterated for an observable order.
    withdrawn_graphs: Arc<FastSet<TermId>>,
    work: super::view_accounting::WorkCounter,
}

impl MutableDataset {
    /// Branch a fresh mutable dataset off a shared frozen `base`. O(1): only the
    /// `Arc` refcount is touched; no quad/term is copied.
    #[must_use]
    pub fn new(base: Arc<RdfDataset>) -> Self {
        Self {
            base,
            graph_existence: GraphExistenceMode::Implicit,
            delta: DeltaBuilder::default(),
            added: FastSet::default(),
            added_ord: FastMap::default(),
            next_added_ord: 0,
            suppressed: FastSet::default(),
            suppressed_rows: 0,
            declared_graphs: Vec::new(),
            graph_rows: FastMap::default(),
            withdrawn_graphs: Arc::default(),
            work: super::view_accounting::WorkCounter::default(),
        }
    }

    /// Branch from `base` with an explicitly selected graph-existence policy.
    /// No base row is copied; the policy applies only to this mutable branch.
    #[must_use]
    pub fn new_with_graph_existence(base: Arc<RdfDataset>, mode: GraphExistenceMode) -> Self {
        Self {
            graph_existence: mode,
            ..Self::new(base)
        }
    }

    /// The graph-existence policy selected when this branch was created.
    #[must_use]
    pub const fn graph_existence(&self) -> GraphExistenceMode {
        self.graph_existence
    }

    /// Create an empty named graph under this branch's selected policy.
    /// In implicit mode this succeeds without registering a declaration. In
    /// remembered mode a new slot is registered, and an existing slot is refused.
    ///
    /// # Errors
    /// Invalid graph names receive the same ingress diagnostic as
    /// [`Self::declare_named_graph`]; `rdf-ir-graph-already-exists` names a
    /// remembered slot that already exists, including a populated graph.
    pub fn create_named_graph(&mut self, graph: TermValue) -> Result<bool, crate::RdfDiagnostic> {
        Self::check_graph_name(&graph)?;
        if self.graph_existence == GraphExistenceMode::Implicit {
            return Ok(false);
        }
        if self.has_named_graph(&graph) {
            return Err(crate::RdfDiagnostic::error(
                "rdf-ir-graph-already-exists",
                "the named graph already exists",
            ));
        }
        Ok(self.declare_graph(graph))
    }

    /// Whether a named graph has a present slot or any effective RDF row.
    /// Probes the existing declaration registry and live row counts without
    /// freezing, scanning rows or minting a term.
    #[must_use]
    pub fn has_named_graph(&self, graph: &TermValue) -> bool {
        if self.declared_graphs.contains(graph)
            || self
                .base_graph(graph)
                .is_some_and(|id| !self.withdrawn_graphs.contains(&id))
        {
            return true;
        }
        self.base
            .term_id_by_value(graph)
            .map(MutTermId::Base)
            .or_else(|| self.delta.find(graph).map(MutTermId::Delta))
            .is_some_and(|id| self.graph_rows.get(&id).is_some_and(|&rows| rows > 0))
    }

    /// Declare that the named graph `graph` exists, even if it never owns a quad —
    /// the mutable twin of
    /// [`RdfDatasetBuilder::declare_named_graph`]. The declaration survives
    /// [`freeze`](Self::freeze) and [`snapshot_view`](Self::snapshot_view), where the
    /// graph is listed among [`crate::DatasetView::named_graphs`]. Returns `false` when the
    /// graph was already declared here or by the base.
    ///
    /// # Errors
    ///
    /// `rdf-ir-graph-name-invalid` when `graph` is neither an IRI nor a blank node,
    /// and the shared IRI diagnostic code when it is a relative IRI.
    pub fn declare_named_graph(&mut self, graph: TermValue) -> Result<bool, crate::RdfDiagnostic> {
        Self::check_graph_name(&graph)?;
        Ok(self.declare_graph(graph))
    }

    fn check_graph_name(graph: &TermValue) -> Result<(), crate::RdfDiagnostic> {
        if !matches!(graph, TermValue::Iri(_) | TermValue::Blank { .. }) {
            return Err(crate::RdfDiagnostic::error(
                "rdf-ir-graph-name-invalid",
                "a declared named graph must be an IRI or blank node",
            ));
        }
        check_value_absolute(graph).map_err(|error| {
            crate::RdfDiagnostic::error(error.diagnostic_code(), error.to_string())
        })
    }

    /// Register presence in the one declaration home. Record ingress and explicit
    /// declaration ingress retain their existing validation boundaries.
    fn declare_graph(&mut self, graph: TermValue) -> bool {
        // A base graph keeps its one declaration: declaring it again after a mutation
        // withdrew it restores the base's, so it is never listed twice.
        if let Some(id) = self.base_graph(&graph) {
            if !self.withdrawn_graphs.contains(&id) {
                return false;
            }
            Arc::make_mut(&mut self.withdrawn_graphs).remove(&id);
            return true;
        }
        if self.declared_graphs.contains(&graph) {
            return false;
        }
        self.declared_graphs.push(graph);
        true
    }

    /// Every named graph this dataset carries as a declaration — the base's named
    /// graphs a mutation has not withdrawn, then each graph declared since — whether
    /// or not it owns a quad now.
    pub fn declared_named_graphs(&self) -> impl Iterator<Item = TermValue> + '_ {
        self.base
            .named_graphs()
            .filter(|id| !self.withdrawn_graphs.contains(id))
            .map(|id| self.base_value(id))
            .chain(self.declared_graphs.iter().cloned())
    }

    /// The base's id for `graph` when the base names it as a graph.
    fn base_graph(&self, graph: &TermValue) -> Option<TermId> {
        self.base
            .term_id_by_value(graph)
            .filter(|&id| self.base.has_named_graph(id))
    }

    /// The shared frozen base this dataset branched from.
    #[must_use]
    pub fn base(&self) -> &Arc<RdfDataset> {
        &self.base
    }

    /// Visit every retained blank identity without freezing or copying the dataset.
    /// Includes suppressed base terms, blanks nested in delta triple terms or
    /// composite literals, and blank names of graphs declared through
    /// [`Self::declare_named_graph`] — which own no row and so live in no term table,
    /// yet survive every freeze and snapshot — so fresh publication can avoid all
    /// identities this destination owns. An identity may be visited more than once.
    /// The first `Break` ends the visit.
    pub fn visit_blank_identities<B>(
        &self,
        mut visit: impl FnMut(&str, crate::BlankScope) -> ControlFlow<B>,
    ) -> ControlFlow<B> {
        for index in 0..self.base.term_count() {
            let id = TermId::from_index(u32::try_from(index).expect("native index fits u32"));
            if let TermRef::Blank { label, scope } = self.base.resolve(id) {
                visit(label, scope)?;
            }
        }
        for value in &self.delta.values {
            value.visit_blank_identities(&mut visit)?;
        }
        for graph in &self.declared_graphs {
            graph.visit_blank_identities(&mut visit)?;
        }
        ControlFlow::Continue(())
    }

    // -- value ↔ MutTermId resolution -------------------------------------------------

    /// Resolve a base [`TermId`] to its dataset-independent [`TermValue`], through
    /// datatype ids and triple components. The inverse of interning a value.
    fn base_value(&self, id: TermId) -> TermValue {
        Self::base_value_of(&self.base, id)
    }

    /// `base_value` without `&self`, so it can be reused by `freeze`'s remap closures.
    ///
    /// A triple term is assembled bottom-up over [`try_fold_nested`]'s work list: its
    /// subject, predicate and object are resolved in that order, each fully before
    /// the next.
    fn base_value_of(base: &RdfDataset, id: TermId) -> TermValue {
        let value = try_fold_nested(
            id,
            &mut (),
            |(), id| {
                Ok::<_, Infallible>(Nested::Leaf(match base.resolve(id) {
                    TermRef::Iri(iri) => TermValue::Iri(iri.to_string()),
                    TermRef::Blank { label, scope } => TermValue::Blank {
                        label: label.to_string(),
                        scope,
                    },
                    TermRef::Literal {
                        lexical,
                        datatype,
                        language,
                        direction,
                    } => {
                        // The datatype id is a base IRI term; resolve it to its IRI string.
                        // A frozen dataset's literal datatype is an IRI by construction:
                        // `RdfDatasetBuilder::intern_literal` mints it through `intern_iri`,
                        // and the pack decoder refuses a datatype entry that is not an IRI
                        // before a pack ever becomes a dataset. Any other shape is a broken
                        // invariant, stated exactly as `RdfDataset::term_value` states it —
                        // never rendered into a datatype string, because a `Debug` rendering
                        // used as an IRI does not fail HERE; it fails later as an `IriError`
                        // about text nobody wrote.
                        let datatype = match base.resolve(datatype) {
                            TermRef::Iri(dt) => dt.to_string(),
                            other => unreachable!(
                                "literal datatype must resolve to an IRI, got {other:?}"
                            ),
                        };
                        TermValue::Literal {
                            lexical_form: lexical.to_string(),
                            datatype,
                            language: language.map(str::to_string),
                            direction,
                        }
                    }
                    TermRef::Triple { s, p, o } => return Ok(Nested::Triple(s, p, o)),
                }))
            },
            |(), _, s, p, o| {
                Ok(TermValue::Triple {
                    s: TermBox::new(s),
                    p: TermBox::new(p),
                    o: TermBox::new(o),
                })
            },
        );
        match value {
            Ok(value) => value,
        }
    }

    /// Resolve a [`MutTermId`] to its dataset-independent [`TermValue`]. A `Delta`
    /// component is stored fully-resolved by value, so this is a single clone — no
    /// recursion through the delta interner.
    fn mut_value(&self, id: MutTermId) -> TermValue {
        match id {
            MutTermId::Base(b) => self.base_value(b),
            MutTermId::Delta(d) => self.delta.value(d).clone(),
        }
    }

    /// Resolve a [`TermValue`] to a [`MutTermId`]: a base hit binds `Base`, a miss
    /// mints (or finds) a `Delta` id in the delta interner. The delta stores the term
    /// fully-resolved by value, so a brand-new triple term is interned whole as one
    /// `TermValue::Triple` (its components carried by value).
    /// A base hit needs no absoluteness check: the base is a FROZEN [`RdfDataset`],
    /// which could not have been constructed carrying a relative IRI. Only the delta
    /// mint below can introduce a new one, and it is checked there.
    fn resolve_value(&mut self, value: &TermValue) -> Result<MutTermId, IriError> {
        if let Some(id) = self.base.term_id_by_value(value) {
            return Ok(MutTermId::Base(id));
        }
        Ok(MutTermId::Delta(self.delta.intern(value.clone())?))
    }

    /// Build a [`QuadKey`] from a value-quad, resolving each component to a
    /// [`MutTermId`] (minting delta ids for new terms as a side effect).
    ///
    /// # Errors
    ///
    /// [`IriError`] if any component carries a non-absolute IRI. The delta may have
    /// minted ids for earlier components before the failing one; that is harmless —
    /// an unreferenced delta term is invisible to every read path and is dropped at
    /// freeze — and no [`QuadKey`] is produced, so the quad is not inserted.
    fn key_of(&mut self, quad: &QuadValues) -> Result<QuadKey, IriError> {
        Ok(QuadKey {
            s: self.resolve_value(&quad.s)?,
            p: self.resolve_value(&quad.p)?,
            o: self.resolve_value(&quad.o)?,
            g: match quad.g.as_ref() {
                Some(g) => Some(self.resolve_value(g)?),
                None => None,
            },
        })
    }

    /// Build a [`QuadKey`] from a value-quad WITHOUT minting: every component must
    /// already resolve to a base id, else the quad cannot be present and we return
    /// `None`. Used by the read paths (`contains`/`remove`) so a probe for an absent
    /// quad never grows the delta interner.
    fn key_of_existing(&self, quad: &QuadValues) -> Option<QuadKey> {
        let resolve = |v: &TermValue| self.base.term_id_by_value(v).map(MutTermId::Base);
        // A component that is not in the base can still match a delta-added quad, so a
        // base miss is NOT a definitive absence — fall back to scanning the delta.
        let base_key = (|| {
            Some(QuadKey {
                s: resolve(&quad.s)?,
                p: resolve(&quad.p)?,
                o: resolve(&quad.o)?,
                g: match &quad.g {
                    None => None,
                    Some(g) => Some(resolve(g)?),
                },
            })
        })();
        if let Some(k) = base_key {
            return Some(k);
        }
        // Some component is delta-only: reconstruct the key against the delta interner
        // by value (no mint). If any component is absent from BOTH base and delta, the
        // quad cannot exist, so return None.
        Some(QuadKey {
            s: self.find_value(&quad.s)?,
            p: self.find_value(&quad.p)?,
            o: self.find_value(&quad.o)?,
            g: match &quad.g {
                None => None,
                Some(g) => Some(self.find_value(g)?),
            },
        })
    }

    /// Find a [`TermValue`] as an existing [`MutTermId`] (base OR delta) WITHOUT
    /// minting. `None` if the value is interned nowhere.
    fn find_value(&self, value: &TermValue) -> Option<MutTermId> {
        if let Some(id) = self.base.term_id_by_value(value) {
            return Some(MutTermId::Base(id));
        }
        // Hash-indexed delta lookup (O(1) expected) — no linear scan, no per-term
        // value rebuild: the delta stores `TermValue`s by value and indexes them by
        // their hash, as the base's store-once table indexes its terms.
        self.delta.find(value).map(MutTermId::Delta)
    }

    /// Whether a base [`QuadKey`] (all components `Base`) names a quad in the base.
    fn base_occurrences(&self, key: &QuadKey) -> usize {
        let (MutTermId::Base(s), MutTermId::Base(p), MutTermId::Base(o)) = (key.s, key.p, key.o)
        else {
            return 0;
        };
        let g = match key.g {
            None => GraphMatch::Default,
            Some(MutTermId::Base(g)) => GraphMatch::Named(g),
            // A delta graph id can never name a base quad.
            Some(MutTermId::Delta(_)) => return 0,
        };
        usize::from(
            RdfDataset::quads_for_pattern_indexed(&self.base, Some(s), Some(p), Some(o), g)
                .next()
                .is_some(),
        ) + usize::from(
            self.base
                .reifier_quads_of(s)
                .any(|q| q.p == p && q.o == o && g.matches(q.g)),
        ) + usize::from(
            self.base
                .annotations_of_with_graph(s)
                .any(|(pred, obj, graph)| pred == p && obj == o && g.matches(graph)),
        )
    }

    fn base_contains(&self, key: &QuadKey) -> bool {
        self.base_occurrences(key) > 0
    }

    // -- mutation core ----------------------------------------------------------------

    /// Insert an effective quad (the four rules, insert side). Returns `true` if the
    /// effective set changed.
    fn insert_key(&mut self, key: QuadKey) -> bool {
        let rows = self.insert_rows(key);
        if rows > 0
            && let Some(graph) = key.g
        {
            let first_rows = {
                let live = self.graph_rows_of(graph);
                *live += rows;
                *live == rows
            };
            if first_rows {
                if let MutTermId::Base(id) = graph
                    && self.withdrawn_graphs.contains(&id)
                {
                    Arc::make_mut(&mut self.withdrawn_graphs).remove(&id);
                }
                if self.graph_existence == GraphExistenceMode::RememberEmpty
                    && !matches!(graph, MutTermId::Base(id) if self.base.has_named_graph(id))
                {
                    self.declare_graph(self.mut_value(graph));
                }
            }
        }
        rows > 0
    }

    /// The insert side of the four rules; the number of RDF rows it made effective.
    fn insert_rows(&mut self, key: QuadKey) -> usize {
        // Rule 1: inserting a currently-suppressed base quad un-suppresses it (and
        // does NOT also push to `added`).
        if self.suppressed.remove(&key) {
            let occurrences = self.base_occurrences(&key);
            self.suppressed_rows -= occurrences;
            return occurrences;
        }
        // Already effective (present in base-and-not-suppressed, or already added)?
        if self.contains_key(&key) {
            return 0;
        }
        let inserted = self.added.insert(key);
        if inserted {
            // A brand-new `added` entry: stamp it with the next insertion ordinal so
            // `added_in_order` can replay delta-added quads in call order rather than
            // `added`'s hash-iteration order.
            self.added_ord.insert(key, self.next_added_ord);
            self.next_added_ord += 1;
        }
        usize::from(inserted)
    }

    /// Remove an effective quad (the four rules, remove side). Returns `true` if the
    /// effective set changed.
    fn remove_key(&mut self, key: QuadKey) -> bool {
        let rows = self.remove_rows(key);
        if rows > 0
            && let Some(graph) = key.g
        {
            let live = self.graph_rows_of(graph);
            *live -= rows;
            if *live == 0 && self.graph_existence == GraphExistenceMode::Implicit {
                if let MutTermId::Base(graph) = graph {
                    Arc::make_mut(&mut self.withdrawn_graphs).insert(graph);
                }
                self.withdraw_emptied_declaration(graph);
            }
        }
        rows > 0
    }

    /// A graph declared through [`Self::declare_named_graph`] follows the base's rule:
    /// the mutation that removes its last row withdraws the declaration. Called only
    /// when the graph's live row count reaches zero, and probes only when some
    /// declaration exists, so neither a dataset without one nor a removal that leaves
    /// rows behind pays anything.
    fn withdraw_emptied_declaration(&mut self, graph: MutTermId) {
        if self.declared_graphs.is_empty() {
            return;
        }
        let index = match graph {
            MutTermId::Base(_) => {
                let value = self.mut_value(graph);
                self.declared_graphs.iter().position(|g| *g == value)
            }
            MutTermId::Delta(id) => {
                let value = self.delta.value(id);
                self.declared_graphs.iter().position(|g| g == value)
            }
        };
        if let Some(index) = index {
            self.declared_graphs.remove(index);
        }
    }

    /// The remove side of the four rules; the number of RDF rows it took away.
    fn remove_rows(&mut self, key: QuadKey) -> usize {
        // Rule 2: removing a delta-added quad drops it from `added` (no suppression).
        if self.added.remove(&key) {
            // Drop the matching ordinal too — a later reinsert of the SAME key mints a
            // fresh (later) ordinal, so it replays at its new position, not its stale one.
            self.added_ord.remove(&key);
            return 1;
        }
        // Rule 3: removing a base quad (not in `added`) creates a suppression — but
        // only if it is actually an effective base quad and not already suppressed.
        let occurrences = self.base_occurrences(&key);
        if occurrences > 0 && self.suppressed.insert(key) {
            self.suppressed_rows += occurrences;
            return occurrences;
        }
        0
    }

    /// The live row count of the named graph `graph`, seeded on first touch. Every
    /// mutation in a graph passes through here, so a graph not yet in the map has
    /// never been mutated: a base graph's live count is then its base count, and a
    /// delta graph — which no base row can name — holds none.
    fn graph_rows_of(&mut self, graph: MutTermId) -> &mut usize {
        let base = &self.base;
        self.graph_rows.entry(graph).or_insert_with(|| match graph {
            MutTermId::Base(id) => base.named_graph_row_count(id),
            MutTermId::Delta(_) => 0,
        })
    }

    /// Whether a [`QuadKey`] is in the effective set: `(base ∪ added) − suppressed`.
    fn contains_key(&self, key: &QuadKey) -> bool {
        if self.suppressed.contains(key) {
            return false;
        }
        self.added.contains(key) || self.base_contains(key)
    }

    /// The delta-added keys, replayed in the order they were actually added — NEVER
    /// `added`'s own hash-iteration order.
    ///
    /// `added`/`suppressed` are fixed-key hashed sets, which makes hash-bucket layout
    /// a pure (reproducible) function of content — but reproducible-given-content is
    /// not enough: two processes that inserted the same quads in a different order
    /// (or that hashed to different bucket counts along the way) can still iterate a
    /// fixed-key `HashSet` in different orders. Per the workspace's hash-determinism
    /// policy (`crate::hash`), the fix is never "iterate the hash set" — it's an
    /// explicit sort. Here that sort key is `added_ord`, the insertion ordinal minted
    /// in [`Self::insert_key`], so `freeze()`/`quads_for_pattern` reproduce the exact
    /// call-order sequence a caller built, independent of hash layout or process.
    fn added_in_order(&self) -> Vec<QuadKey> {
        let mut ordered: Vec<(u64, QuadKey)> = self
            .added_ord
            .iter()
            .map(|(&key, &ord)| (ord, key))
            .collect();
        debug_assert_eq!(
            ordered.len(),
            self.added.len(),
            "added/added_ord must stay in lockstep"
        );
        ordered.sort_unstable_by_key(|&(ord, _)| ord);
        ordered.into_iter().map(|(_, key)| key).collect()
    }

    /// A signal that the delta has grown enough that compacting (re-`freeze()` to a
    /// fresh base) is worthwhile — when the added + suppressed churn exceeds a
    /// fraction (here ½) of the base quad count. Advisory only; correctness never
    /// depends on it. An empty base always signals once churn appears, so the first
    /// build off a trivial base still compacts.
    #[must_use]
    pub fn should_compact(&self) -> bool {
        let churn = self.added.len() + self.suppressed.len();
        let base = self.base.rdf_row_count();
        churn * 2 > base
    }

    /// Withdraw the base's declaration of the named graph `graph`, so that it is
    /// enumerated by a snapshot or a freeze only while it holds a row.
    ///
    /// In implicit mode a graph exists while it holds a row. The exception is a graph the base
    /// declared empty (a TriG `GRAPH <g> {}`), which is enumerated until a mutation
    /// empties it: removing a graph's last row does that implicitly, and this call
    /// does it for a graph that has no row to remove — `DROP GRAPH` / `CLEAR GRAPH`
    /// of a declared empty graph. A graph that still holds rows is unaffected (it
    /// stays enumerated while it holds them), rows added afterwards bring the graph
    /// back, and a graph the base never named is a no-op.
    /// In remembered mode last-row removal retains the slot; this explicit call
    /// withdraws it once no rows remain, for example after a DROP removes its rows.
    pub fn withdraw_graph_declaration(&mut self, graph: &TermValue) {
        if let Some(id) = self.base.term_id_by_value(graph) {
            self.withdraw_base_graph(id);
        }
        // A declaration made through `declare_named_graph` is withdrawn the same way:
        // a graph that still holds rows stays enumerated through them.
        self.declared_graphs.retain(|g| g != graph);
    }

    /// [`Self::withdraw_graph_declaration`] for every named graph of the base —
    /// `DROP NAMED` / `DROP ALL` of a dataset with declared empty graphs.
    pub fn withdraw_named_graph_declarations(&mut self) {
        self.try_withdraw_named_graph_declarations(|| Ok::<_, Infallible>(()))
            .unwrap_or_else(|never| match never {});
    }

    /// Withdraw every named-graph declaration, checking before each base entry
    /// and each declaration added to this branch.
    ///
    /// Entries are visited once; populated graphs remain present through their rows.
    /// A caller that needs atomic publication must discard its private branch on error.
    ///
    /// # Errors
    /// Returns the callback's first error before withdrawing that entry. Earlier
    /// withdrawals remain applied to this mutable branch; retained snapshots are unchanged.
    pub fn try_withdraw_named_graph_declarations<E>(
        &mut self,
        mut checkpoint: impl FnMut() -> Result<(), E>,
    ) -> Result<(), E> {
        let base = Arc::clone(&self.base);
        for graph in base.named_graphs() {
            checkpoint()?;
            self.withdraw_base_graph(graph);
        }
        while !self.declared_graphs.is_empty() {
            checkpoint()?;
            let _ = self.declared_graphs.pop();
        }
        Ok(())
    }

    fn withdraw_base_graph(&mut self, graph: TermId) {
        if *self.graph_rows_of(MutTermId::Base(graph)) == 0
            && !self.withdrawn_graphs.contains(&graph)
        {
            Arc::make_mut(&mut self.withdrawn_graphs).insert(graph);
        }
    }

    /// The number of quads added on top of the base (delta size).
    #[must_use]
    pub fn added_len(&self) -> usize {
        self.added.len()
    }

    /// The number of base quads currently suppressed.
    #[must_use]
    pub fn suppressed_len(&self) -> usize {
        self.suppressed.len()
    }

    /// Iterate the effective quads as value-quads — the independent test/property-test
    /// oracle for the effective set. `freeze` builds the effective set directly (so it
    /// can carry per-base-quad source locations), so this is a test-only helper; the
    /// public surface is [`DatasetMut`].
    #[cfg(test)]
    fn effective_value_quads(&self) -> Vec<QuadValues> {
        let mut out: Vec<QuadValues> = Vec::new();
        // Base quads that are not suppressed.
        for q in self
            .base
            .quads()
            .chain(self.base.reifier_quads())
            .chain(self.base.annotation_quads())
        {
            let key = QuadKey {
                s: MutTermId::Base(q.s),
                p: MutTermId::Base(q.p),
                o: MutTermId::Base(q.o),
                g: q.g.map(MutTermId::Base),
            };
            if !self.suppressed.contains(&key) {
                out.push(self.quad_values_of(&key));
            }
        }
        // Delta-added quads, in call order (never `added`'s hash-iteration order).
        for key in self.added_in_order() {
            out.push(self.quad_values_of(&key));
        }
        out
    }

    /// Resolve a [`QuadKey`] to a value-quad (each component to its [`TermValue`]).
    fn quad_values_of(&self, key: &QuadKey) -> QuadValues {
        QuadValues {
            s: self.mut_value(key.s),
            p: self.mut_value(key.p),
            o: self.mut_value(key.o),
            g: key.g.map(|g| self.mut_value(g)),
        }
    }

    // -- freeze (compaction) ----------------------------------------------------------

    /// Compact the effective set into a fresh frozen [`RdfDataset`], **remapping
    /// EVERYTHING** — terms, reifiers, annotations, graph names, and source locations
    /// — into dense [`TermId`]s. `MutTermId`/`DeltaTermId` never leak past this point.
    ///
    /// The shared typed importer re-interns surviving RDF rows, retaining the
    /// statement tables and surviving named-graph declarations. In
    /// [`GraphExistenceMode::Implicit`], last-row removal withdraws the graph;
    /// [`GraphExistenceMode::RememberEmpty`] retains its slot until explicit
    /// [`Self::withdraw_graph_declaration`]. Suppressed statement rows stay absent.
    /// Removing a reifier declaration demotes its surviving annotations to ordinary
    /// quads when no other declaration remains in that graph. Non-RDF sidecars
    /// remain owned by the original base.
    ///
    /// Source LOCATIONS of the surviving base quads are carried too: a base quad is
    /// pushed in base order (so a base ordinal maps to a running new ordinal), and its
    /// location — if the base recorded one — is re-attached to that new handle. The
    /// builder's own freeze sort then remaps the handle to the dense frozen position
    /// (the `attach_location` contract). Delta-added quads were minted in memory, not
    /// parsed from a source, so they carry no location.
    pub fn freeze(&self) -> Result<Arc<RdfDataset>, crate::RdfDiagnostic> {
        let view = self.snapshot_view()?;
        let mut builder = self.base.rebuild_builder();
        super::import::DatasetImporter::new(&mut builder, &view).append();
        let mut ordinal = 0;
        for (old, quad) in self.base.quads().enumerate() {
            if !view.base_quad_is_ordinary(quad) {
                continue;
            }
            if let Some(location) = self.base.location_of(QuadHandle::from_index(old as u32)) {
                builder.attach_location(QuadHandle::from_index(ordinal), location.clone());
            }
            ordinal += 1;
        }
        let dataset = builder.freeze()?;
        self.work.add(super::view_accounting::ViewWork {
            copied_terms: dataset.term_count(),
            copied_rows: dataset.rdf_row_count(),
            freezes: 1,
            materializations: 1,
            copied_text_bytes: dataset.rdf_text_bytes(),
            ..Default::default()
        });
        Ok(dataset)
    }

    /// Successful work across snapshots and compactions created by this owner.
    #[must_use]
    pub fn work_stats(&self) -> super::view_accounting::ViewWork {
        self.work.get()
    }

    /// Publish an immutable read view by freezing only the added delta. The base
    /// dataset and its indexes remain shared; later mutations cannot affect the
    /// snapshot. This is a mutation of one RDF identity space, not a union of
    /// independently parsed documents (blank scopes are preserved).
    ///
    /// # Errors
    /// The delta fails the same RDF admission checks as [`Self::freeze`].
    pub fn snapshot_view(&self) -> Result<DeltaDatasetView, crate::RdfDiagnostic> {
        self.snapshot_view_with_limits(super::view_accounting::ViewLimits::default())
    }

    /// Snapshot publication under caller-selected finite retention ceilings.
    ///
    /// # Errors
    /// Refuses retention overflow or invalid delta records before publication.
    pub fn snapshot_view_with_limits(
        &self,
        limits: super::view_accounting::ViewLimits,
    ) -> Result<DeltaDatasetView, crate::RdfDiagnostic> {
        let mut builder = self.base.rebuild_builder();
        let mut admission = DeltaAdmission::new(&self.base, self.suppressed.len(), limits);
        self.append_delta(&mut builder, &mut admission)?;
        let extent = admission.check(&builder)?;
        let delta = builder.freeze()?;
        debug_assert_eq!(
            (
                delta.term_count(),
                delta.rdf_row_count(),
                delta.rdf_payload_bytes()
            ),
            extent,
            "the admitted extent is exactly what the frozen delta retains"
        );
        // `DeltaDatasetView::new` below checks the same limits once more, over stats
        // it computes from the frozen base and delta: sources, the base's share, and
        // the delta's terms, rows and payload are the values admitted above (the
        // assertion pins the delta's), and its auxiliary charge is the same formula
        // over the same counts. Every quantity is equal, so a snapshot admitted above
        // is admitted there and its refusal is unreachable; the `?` keeps the view's
        // own constructor total rather than trusting this caller.
        self.work.add(super::view_accounting::ViewWork {
            copied_terms: delta.term_count(),
            copied_rows: delta.rdf_row_count(),
            copied_text_bytes: delta.rdf_text_bytes(),
            freezes: 1,
            ..Default::default()
        });
        let base_id = |id| match id {
            MutTermId::Base(id) => id,
            MutTermId::Delta(_) => unreachable!("only base rows can be suppressed"),
        };
        let suppressed = self
            .suppressed
            .iter()
            .map(|q| super::QuadIds {
                s: base_id(q.s),
                p: base_id(q.p),
                o: base_id(q.o),
                g: q.g.map(base_id),
            })
            .collect();
        let view = DeltaDatasetView::new(
            Arc::clone(&self.base),
            delta,
            suppressed,
            Arc::clone(&self.withdrawn_graphs),
            limits,
        )?;
        self.work.add(super::view_accounting::ViewWork {
            copied_index_bytes: view.stats().work.copied_index_bytes,
            ..Default::default()
        });
        Ok(view)
    }

    /// One RDF 1.2 delta classifier shared by compaction and snapshot publication.
    /// Only added subjects probe the base reifier index; a small delta never builds
    /// a base-sized set of owned reifier values.
    ///
    /// `admission` sees every row and declaration as it is interned, and refuses the
    /// delta as soon as it exceeds a retention limit, before the freeze.
    fn append_delta(
        &self,
        builder: &mut RdfDatasetBuilder,
        admission: &mut DeltaAdmission<'_>,
    ) -> Result<(), crate::RdfDiagnostic> {
        let added_values: Vec<QuadValues> = self
            .added_in_order()
            .into_iter()
            .map(|k| self.quad_values_of(&k))
            .collect();
        let mut reifier_subjects = FastSet::default();
        let mut reifier_decl: Vec<bool> = Vec::with_capacity(added_values.len());
        for q in &added_values {
            let is_decl = matches!(&q.p, TermValue::Iri(iri) if iri == RDF_REIFIES)
                && matches!(q.o, TermValue::Triple { .. });
            if is_decl {
                reifier_subjects.insert((q.g.clone(), q.s.clone()));
            }
            reifier_decl.push(is_decl);
        }
        // Pass 2: push reifier declarations first (so the side table is populated
        // before any freeze consumer inspects it), then classify the rest.
        for (q, &is_decl) in added_values.iter().zip(&reifier_decl) {
            if !is_decl {
                continue;
            }
            let TermValue::Triple { s, p, o } = &q.o else {
                unreachable!("is_decl implies a triple-term object");
            };
            let reifier = builder.intern_value(&q.s);
            let s = builder.intern_value(s);
            let p = builder.intern_value(p);
            let o = builder.intern_value(o);
            let triple = builder.intern_triple(s, p, o);
            let g = q.g.as_ref().map(|g| builder.intern_value(g));
            builder.push_reifier_in_graph(reifier, triple, g);
            admission.row(builder, g)?;
        }
        for (q, &is_decl) in added_values.iter().zip(&reifier_decl) {
            if is_decl {
                continue;
            }
            let s = builder.intern_value(&q.s);
            let p = builder.intern_value(&q.p);
            let o = builder.intern_value(&q.o);
            let g = q.g.as_ref().map(|g| builder.intern_value(g));
            // A quad whose subject is a reifier is that reifier's annotation, in its
            // own graph — mirroring `fold_statement_layer`'s pass 2 so an UPDATE freeze
            // and a parse of the same statement agree.
            if reifier_subjects.contains(&(q.g.clone(), q.s.clone()))
                || self.base.term_id_by_value(&q.s).is_some_and(|id| {
                    self.base.reifier_quads_of(id).any(|row| {
                        row.g.map(|graph| self.base_value(graph)) == q.g
                            && !self.suppressed.contains(&QuadKey {
                                s: MutTermId::Base(row.s),
                                p: MutTermId::Base(row.p),
                                o: MutTermId::Base(row.o),
                                g: row.g.map(MutTermId::Base),
                            })
                    })
                })
            {
                builder.push_annotation_in_graph(s, p, o, g);
            } else {
                builder.push_quad(s, p, o, g);
            }
            admission.row(builder, g)?;
        }
        // Declarations last, so a delta that declares nothing interns exactly as it
        // always did.
        for graph in &self.declared_graphs {
            let id = builder.intern_value(graph);
            builder.declare_named_graph(id);
            admission.row(builder, Some(id))?;
        }
        Ok(())
    }
}

/// The retention check of one snapshot, made while [`MutableDataset::append_delta`]
/// interns the delta rather than over a second walk of it.
///
/// The builder's tables hold one entry per distinct term and row as it goes
/// ([`RdfDatasetBuilder::pending_extent`]), and every quantity the limits bound
/// only grows as interning proceeds. A check that fails part way therefore fails
/// at the end too, and the check after the last row is the exact verdict on the
/// frozen delta: a delta over a limit is refused before the freeze, with no work
/// counted, and one within every limit is admitted.
struct DeltaAdmission<'a> {
    base: &'a RdfDataset,
    suppressed: usize,
    limits: super::view_accounting::ViewLimits,
    /// The distinct named graphs the delta names, by row or by declaration — the
    /// one table the freeze deduplicates.
    graphs: FastSet<TermId>,
    /// The graph of the previous row, so a run of rows in one graph probes the set
    /// once.
    last_graph: Option<TermId>,
    rows_since_check: usize,
}

impl<'a> DeltaAdmission<'a> {
    /// Rows interned between two checks part way through the delta. Only how soon an
    /// oversized delta stops depends on it; the verdict is the final check's.
    const CHECK_EVERY: usize = 1024;

    fn new(
        base: &'a RdfDataset,
        suppressed: usize,
        limits: super::view_accounting::ViewLimits,
    ) -> Self {
        Self {
            base,
            suppressed,
            limits,
            graphs: FastSet::default(),
            last_graph: None,
            rows_since_check: 0,
        }
    }

    /// The snapshot's retention as of what `builder` holds now, and the delta's own
    /// `(terms, rows, payload)`.
    fn stats(
        &self,
        builder: &RdfDatasetBuilder,
    ) -> (super::view_accounting::ViewStats, (usize, usize, usize)) {
        let extent = builder.pending_extent(self.graphs.len());
        let (terms, rows, payload) = extent;
        let mut stats = super::view_accounting::ViewStats::default();
        stats.retain(self.base);
        stats.retained_sources += 1;
        stats.retained_terms = stats.retained_terms.saturating_add(terms);
        stats.retained_rows = stats.retained_rows.saturating_add(rows);
        stats.retained_payload_bytes = stats.retained_payload_bytes.saturating_add(payload);
        // The construction charge `DeltaDatasetView::new` makes for the same delta.
        stats.auxiliary_bytes = terms
            .saturating_mul(DeltaDatasetView::AUXILIARY_BYTES_PER_DELTA_TERM)
            .saturating_add(
                self.suppressed
                    .saturating_add(rows)
                    .saturating_mul(4 * size_of::<super::QuadIds>()),
            );
        (stats, extent)
    }

    /// Check the limits against what `builder` holds now.
    fn check(
        &self,
        builder: &RdfDatasetBuilder,
    ) -> Result<(usize, usize, usize), crate::RdfDiagnostic> {
        let (stats, extent) = self.stats(builder);
        self.limits.check(&stats)?;
        Ok(extent)
    }

    /// Account one interned row (or declaration) naming `graph`, checking the limits
    /// every [`Self::CHECK_EVERY`] rows.
    fn row(
        &mut self,
        builder: &RdfDatasetBuilder,
        graph: Option<TermId>,
    ) -> Result<(), crate::RdfDiagnostic> {
        if let Some(graph) = graph
            && self.last_graph != Some(graph)
        {
            self.graphs.insert(graph);
            self.last_graph = Some(graph);
        }
        self.rows_since_check += 1;
        if self.rows_since_check == Self::CHECK_EVERY {
            self.rows_since_check = 0;
            self.check(builder)?;
        }
        Ok(())
    }
}

/// Enforce the IR-boundary absoluteness invariant over every IRI a [`TermValue`]
/// carries: the value itself, a literal's datatype, and — at any depth — the
/// components of a triple term, which the delta interns WHOLE rather than
/// component-by-component.
///
/// # This is fail-fast, and it does not replace the freeze-time check
///
/// [`MutableDataset::freeze`] re-interns every value through
/// [`RdfDatasetBuilder`], which enforces the same invariant; that is what makes a
/// relative IRI unrepresentable in the frozen IR, and it must stay. This check
/// exists so a caller mutating a dataset learns at the `insert` that named the
/// offending quad, instead of at some later freeze whose error cannot point back to
/// the call. **Deleting either one because "the other covers it" is a regression:**
/// the freeze check is the invariant, this one is the diagnosis.
///
/// Blank-node labels and literal lexical forms are arbitrary strings and are
/// deliberately untouched — only IRIs are IRIs.
///
/// The IRIs are checked in [`TermValue::visit_terms`]'s pre-order — a triple term's
/// subject fully before its predicate, before its object — and the first one refused
/// is the error.
fn check_value_absolute(value: &TermValue) -> Result<(), IriError> {
    let checked = value.visit_terms(|term| {
        let checked = match term {
            TermValue::Iri(iri) => super::absolute::check_absolute(iri),
            TermValue::Literal { datatype, .. } => super::absolute::check_absolute(datatype),
            TermValue::Blank { .. } | TermValue::Triple { .. } => Ok(()),
        };
        match checked {
            Ok(()) => ControlFlow::Continue(()),
            Err(error) => ControlFlow::Break(error),
        }
    });
    match checked {
        ControlFlow::Continue(()) => Ok(()),
        ControlFlow::Break(error) => Err(error),
    }
}

/// An owned, dataset-independent quad value — the argument type of
/// [`DatasetMut::insert`]/[`remove`](DatasetMut::remove)/[`contains`](DatasetMut::contains).
///
/// Insert/remove take terms BY VALUE (each component a [`TermValue`]) rather than by
/// id because a `MutableDataset`'s caller does not hold dataset-local ids for
/// brand-new terms (those don't exist until minted), and a value is the only
/// identity that is well-defined across the base/delta boundary (C0.8). The mutable
/// layer resolves each value to a `MutTermId` (base hit, or a freshly-minted delta
/// id) internally.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct QuadValues {
    /// The subject term value.
    pub s: TermValue,
    /// The predicate term value.
    pub p: TermValue,
    /// The object term value.
    pub o: TermValue,
    /// The graph-name term value (`None` = default graph).
    pub g: Option<TermValue>,
}

impl QuadValues {
    /// Every row of `dataset`'s RDF surface — the plain quads and BOTH statement
    /// tables (reifier and annotation rows) — as owned value-quads, in table order:
    /// what a copy of the dataset through [`DatasetMut`] is seeded with.
    #[must_use]
    pub fn surface_of(dataset: &RdfDataset) -> Vec<Self> {
        dataset
            .quads()
            .chain(dataset.reifier_quads())
            .chain(dataset.annotation_quads())
            .map(|quad| Self {
                s: dataset.term_value(quad.s),
                p: dataset.term_value(quad.p),
                o: dataset.term_value(quad.o),
                g: quad.g.map(|graph| dataset.term_value(graph)),
            })
            .collect()
    }

    /// A convenience constructor for a default-graph quad.
    #[must_use]
    pub fn triple(s: TermValue, p: TermValue, o: TermValue) -> Self {
        Self { s, p, o, g: None }
    }

    /// A convenience constructor for a named-graph quad.
    #[must_use]
    pub fn quad(s: TermValue, p: TermValue, o: TermValue, g: TermValue) -> Self {
        Self {
            s,
            p,
            o,
            g: Some(g),
        }
    }

    /// Enforce the IR-boundary absoluteness invariant over this quad WITHOUT interning
    /// it: every IRI in all four positions, each literal's datatype, and — at any depth —
    /// the components of any triple term.
    ///
    /// # Why this is public
    ///
    /// [`DatasetMut::insert`] already applies it, so a caller building a dataset needs
    /// nothing here. This exists for the container that holds quads but is NOT a store —
    /// the Python `Dataset`, which is a plain quad list with set semantics and no term
    /// table to intern into. Without a seam it would have had to re-spell "parse, then
    /// test for a scheme", and a second spelling of the rule is exactly how the codecs
    /// and the IR boundary would drift apart. It delegates to the same
    /// crate-private `super::absolute::check_absolute` every other ingress reaches, so there is
    /// exactly one owner of the rule.
    ///
    /// Blank-node labels and literal lexical forms are arbitrary strings and are
    /// deliberately untouched — only IRIs are IRIs.
    ///
    /// # Errors
    ///
    /// [`IriError`] naming the first non-absolute IRI found, carrying the workspace's
    /// shared [`IriError::diagnostic_code`] spelling (`iri-relative-no-base` for a
    /// scheme-less reference).
    pub fn check_absolute_iris(&self) -> Result<(), IriError> {
        check_value_absolute(&self.s)?;
        check_value_absolute(&self.p)?;
        check_value_absolute(&self.o)?;
        match &self.g {
            Some(g) => check_value_absolute(g),
            None => Ok(()),
        }
    }
}

impl DatasetMut for MutableDataset {
    type Quad = QuadValues;

    fn insert(&mut self, quad: Self::Quad) -> Result<bool, IriError> {
        let key = self.key_of(&quad)?;
        Ok(self.insert_key(key))
    }

    fn remove(&mut self, quad: &Self::Quad) -> bool {
        match self.key_of_existing(quad) {
            Some(key) => self.remove_key(key),
            // A quad mentioning a term interned nowhere cannot be present.
            None => false,
        }
    }

    fn contains(&self, quad: &Self::Quad) -> bool {
        match self.key_of_existing(quad) {
            Some(key) => self.contains_key(&key),
            None => false,
        }
    }

    fn quads_for_pattern(
        &self,
        s: Option<&TermValue>,
        p: Option<&TermValue>,
        o: Option<&TermValue>,
        g: GraphMatchValue<'_>,
    ) -> Vec<QuadValues> {
        // Resolve each bound position to a MutTermId without minting; a bound value
        // interned nowhere can match nothing, so the whole pattern yields empty.
        let resolve_bound = |v: Option<&TermValue>| -> Result<Option<MutTermId>, ()> {
            match v {
                None => Ok(None),
                Some(v) => self.find_value(v).map(Some).ok_or(()),
            }
        };
        let (Ok(sb), Ok(pb), Ok(ob)) = (resolve_bound(s), resolve_bound(p), resolve_bound(o))
        else {
            return Vec::new();
        };
        // The graph filter, in MutTermId space. The named graph is matched BY VALUE
        // (resolved without minting), so both a base-named and a delta-only-named
        // graph are expressible. A `Named` value interned nowhere — in neither base
        // nor delta — names no graph at all, so the whole pattern yields empty
        // (mirroring a bound `s`/`p`/`o` miss above).
        let gb: GraphMatch<MutTermId> = match g {
            GraphMatchValue::Any => GraphMatch::Any,
            GraphMatchValue::Default => GraphMatch::Default,
            GraphMatchValue::Named(value) => match self.find_value(value) {
                Some(id) => GraphMatch::Named(id),
                None => return Vec::new(),
            },
        };

        self.effective_keys()
            .filter(|k| {
                sb.is_none_or(|id| k.s == id)
                    && pb.is_none_or(|id| k.p == id)
                    && ob.is_none_or(|id| k.o == id)
                    && gb.matches(k.g)
            })
            .map(|k| self.quad_values_of(&k))
            .collect()
    }
}

impl MutableDataset {
    /// All effective quad KEYS (base-not-suppressed ∪ added), in MutTermId space,
    /// yielded lazily: base quads in frozen order, then the delta. The sole caller
    /// filters and maps them once, so no base-sized `Vec` is materialized.
    fn effective_keys(&self) -> impl Iterator<Item = QuadKey> + '_ {
        let base = self
            .base
            .quads()
            .chain(self.base.reifier_quads())
            .chain(self.base.annotation_quads())
            .filter_map(|q| {
                let key = QuadKey {
                    s: MutTermId::Base(q.s),
                    p: MutTermId::Base(q.p),
                    o: MutTermId::Base(q.o),
                    g: q.g.map(MutTermId::Base),
                };
                (!self.suppressed.contains(&key)).then_some(key)
            });
        // Delta-added quads, in call order (never `added`'s hash-iteration order) —
        // `quads_for_pattern` (the sole caller) filters this sequence, so its own
        // output order inherits the same call-order guarantee.
        base.chain(self.added_in_order())
    }

    /// The effective quads as `Copy` base-or-frozen `QuadIds` is NOT exposed: ids
    /// straddling base/delta have no single dataset to be local to (C0.8). Consumers
    /// read values via [`DatasetMut::quads_for_pattern`] or `freeze()` to a frozen
    /// dataset and read `QuadIds` there.
    #[doc(hidden)]
    pub fn effective_count(&self) -> usize {
        // O(1) from the mutation invariants (no base scan):
        //   • `suppressed_rows` counts all native table occurrences hidden by
        //     suppression keys, including equal rows in different tables;
        //   • every key in `added` is a non-base, non-suppressed quad (insert adds
        //     only when `!contains_key`), so `added.len()` quads are net-new;
        //   • `added` and `suppressed` are disjoint.
        // Hence effective = base ∪ added − suppressed has exactly this cardinality.
        self.base.rdf_row_count() + self.added.len() - self.suppressed_rows
    }
}

// A `MutableDataset` holds an `Arc<RdfDataset>` (Send+Sync) plus owned `HashSet`/`Vec`
// state, so it is itself `Send + Sync`. The guard fails the build if that regresses
// (e.g. if a future field introduces a non-Sync interior). `QuadValues` is the owned
// value-quad public arg type and must also cross threads.
const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<MutableDataset>();
    assert_send_sync::<QuadValues>();
    assert_send_sync::<MutTermId>();
    assert_send_sync::<QuadKey>();
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::RdfDatasetBuilder;
    use crate::model::RdfLiteral;
    use purrdf_testkit::prop::prelude::*;
    use std::collections::HashSet;

    // -- helpers ----------------------------------------------------------------------

    fn iri_val(n: &str) -> TermValue {
        TermValue::Iri(format!("http://example.org/{n}"))
    }

    fn q(s: &str, p: &str, o: &str) -> QuadValues {
        QuadValues::triple(iri_val(s), iri_val(p), iri_val(o))
    }

    /// Insert a fixture quad, asserting it satisfies the absoluteness invariant.
    /// Every fixture below builds its IRIs through `iri_val`, so it does; the refusal
    /// path has its own cases at the end of this module.
    fn ins(m: &mut MutableDataset, quad: QuadValues) -> bool {
        m.insert(quad).expect("fixture IRIs are absolute")
    }

    /// A graph declared on the mutable layer survives a freeze and a snapshot as a
    /// declared graph, IRI- and blank-named alike; a redeclaration and a non-graph
    /// term are answered, and a dataset that declares nothing freezes as before.
    #[test]
    fn declared_named_graphs_survive_freeze_and_snapshot() {
        let empty_base = RdfDatasetBuilder::new().freeze().expect("empty base");
        let mut m = MutableDataset::new(Arc::clone(&empty_base));
        ins(&mut m, q("s", "p", "o"));
        let plain = m.freeze().expect("freezes");
        assert_eq!(plain.named_graphs().count(), 0);

        let blank = TermValue::Blank {
            label: "bg".to_owned(),
            scope: crate::BlankScope::DEFAULT,
        };
        assert_eq!(m.declare_named_graph(iri_val("g")), Ok(true));
        assert_eq!(m.declare_named_graph(blank.clone()), Ok(true));
        assert_eq!(m.declare_named_graph(iri_val("g")), Ok(false));
        let literal = TermValue::Literal {
            lexical_form: "x".to_owned(),
            datatype: purrdf_xsd::datatype::XSD_STRING.to_owned(),
            language: None,
            direction: None,
        };
        assert_eq!(
            m.declare_named_graph(literal).map_err(|e| e.code),
            Err("rdf-ir-graph-name-invalid".into())
        );
        assert_eq!(m.declared_named_graphs().count(), 2);

        let frozen = m.freeze().expect("freezes");
        let names: std::collections::BTreeSet<TermValue> = frozen
            .named_graphs()
            .map(|g| frozen.term_value(g))
            .collect();
        assert_eq!(
            names,
            std::collections::BTreeSet::from([iri_val("g"), blank])
        );
        let view = m.snapshot_view().expect("snapshots");
        assert_eq!(crate::DatasetView::named_graphs(&view).count(), 2);

        // A branch off the frozen result keeps the declarations as base declarations.
        let branch = MutableDataset::new(frozen);
        assert_eq!(branch.declared_named_graphs().count(), 2);
    }

    /// Declarations made through `declare_named_graph` take part in the withdrawal
    /// rules: a declaration DROP/CLEAR names is withdrawn, an untouched one survives,
    /// `DROP NAMED`/`ALL` withdraws every one, and removing a declared graph's last
    /// row withdraws it as it would a base declaration. Rows keep a graph enumerated.
    #[test]
    fn declared_named_graphs_follow_the_withdrawal_rules() {
        let names = |m: &MutableDataset| -> std::collections::BTreeSet<TermValue> {
            let frozen = m.freeze().expect("freezes");
            frozen
                .named_graphs()
                .map(|g| frozen.term_value(g))
                .collect()
        };
        let empty_base = || RdfDatasetBuilder::new().freeze().expect("empty base");

        // DROP GRAPH of one declaration; the other survives.
        let mut m = MutableDataset::new(empty_base());
        m.declare_named_graph(iri_val("g")).expect("declares");
        m.declare_named_graph(iri_val("h")).expect("declares");
        m.withdraw_graph_declaration(&iri_val("h"));
        assert_eq!(names(&m), std::collections::BTreeSet::from([iri_val("g")]));
        assert_eq!(m.declared_named_graphs().count(), 1);

        // DROP ALL withdraws every declaration.
        m.withdraw_named_graph_declarations();
        assert!(names(&m).is_empty());

        // A declared graph with a row stays while the row does; removing the last
        // row withdraws it.
        let mut m = MutableDataset::new(empty_base());
        m.declare_named_graph(iri_val("g")).expect("declares");
        let row = QuadValues::quad(iri_val("s"), iri_val("p"), iri_val("o"), iri_val("g"));
        ins(&mut m, row.clone());
        m.withdraw_graph_declaration(&iri_val("g"));
        assert_eq!(names(&m), std::collections::BTreeSet::from([iri_val("g")]));
        let mut m = MutableDataset::new(empty_base());
        m.declare_named_graph(iri_val("g")).expect("declares");
        ins(&mut m, row.clone());
        assert!(m.remove(&row));
        assert!(names(&m).is_empty());

        // A withdrawn base declaration is not reported, and may be declared again.
        let mut b = RdfDatasetBuilder::new();
        let e = b.intern_iri("http://example.org/e");
        b.declare_named_graph(e);
        let mut m = MutableDataset::new(b.freeze().expect("freezes"));
        m.withdraw_graph_declaration(&iri_val("e"));
        assert_eq!(m.declared_named_graphs().count(), 0);
        assert!(names(&m).is_empty());
        assert_eq!(m.declare_named_graph(iri_val("e")), Ok(true));
        assert_eq!(names(&m), std::collections::BTreeSet::from([iri_val("e")]));
    }

    /// Declaring a withdrawn base graph again restores the base's declaration
    /// rather than adding a second one: the graph is listed once whether or not a
    /// row then lands in it, and the removal of that row withdraws it again.
    #[test]
    fn redeclaring_a_withdrawn_base_graph_lists_it_once() {
        let listed = |m: &MutableDataset| m.declared_named_graphs().collect::<Vec<_>>();
        let enumerated = |m: &MutableDataset| {
            let view = m.snapshot_view().expect("publishes");
            crate::DatasetView::named_graphs(&view).count()
        };
        let row = QuadValues::quad(iri_val("s"), iri_val("p"), iri_val("o"), iri_val("bg"));
        // A base graph declared empty, and one that held a row the mutation removed.
        let mut empty = RdfDatasetBuilder::new();
        let bg = empty.intern_iri("http://example.org/bg");
        empty.declare_named_graph(bg);
        let mut populated = RdfDatasetBuilder::new();
        let (s, p, o) = (
            populated.intern_iri("http://example.org/s0"),
            populated.intern_iri("http://example.org/p"),
            populated.intern_iri("http://example.org/o0"),
        );
        let bg = populated.intern_iri("http://example.org/bg");
        populated.push_quad(s, p, o, Some(bg));
        for (base, emptied) in [(empty.freeze(), false), (populated.freeze(), true)] {
            let mut m = MutableDataset::new(base.expect("base freezes"));
            if emptied {
                assert!(m.remove(&QuadValues::quad(
                    iri_val("s0"),
                    iri_val("p"),
                    iri_val("o0"),
                    iri_val("bg"),
                )));
            } else {
                m.withdraw_graph_declaration(&iri_val("bg"));
            }
            assert_eq!(listed(&m), []);
            assert_eq!(m.declare_named_graph(iri_val("bg")), Ok(true));
            assert_eq!(listed(&m), [iri_val("bg")]);
            assert_eq!(enumerated(&m), 1);
            assert_eq!(m.declare_named_graph(iri_val("bg")), Ok(false));
            ins(&mut m, row.clone());
            assert_eq!(listed(&m), [iri_val("bg")], "listed once with a row");
            assert_eq!(enumerated(&m), 1);
            assert!(m.remove(&row));
            assert_eq!(listed(&m), [], "removing its last row withdraws it");
            assert_eq!(enumerated(&m), 0);
        }
        // A base graph that was never withdrawn is already declared.
        let mut b = RdfDatasetBuilder::new();
        let e = b.intern_iri("http://example.org/bg");
        b.declare_named_graph(e);
        let mut m = MutableDataset::new(b.freeze().expect("freezes"));
        assert_eq!(m.declare_named_graph(iri_val("bg")), Ok(false));
        assert_eq!(listed(&m), [iri_val("bg")]);
    }

    /// A base with three quads: (a,p,b), (a,p,c), (b,p,c) — and one reifier+annotation.
    fn base3() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let a = b.intern_iri("http://example.org/a");
        let p = b.intern_iri("http://example.org/p");
        let bb = b.intern_iri("http://example.org/b");
        let c = b.intern_iri("http://example.org/c");
        b.push_quad(a, p, bb, None);
        b.push_quad(a, p, c, None);
        b.push_quad(bb, p, c, None);
        // A reifier + annotation over the (a,p,b) triple term.
        let triple = b.intern_triple(a, p, bb);
        let r = b.intern_iri("http://example.org/r");
        let conf = b.intern_iri("http://example.org/confidence");
        let score = b.intern_literal(RdfLiteral::typed(
            "0.9",
            "http://www.w3.org/2001/XMLSchema#decimal",
        ));
        b.push_reifier(r, triple);
        b.push_annotation(r, conf, score);
        b.freeze().expect("base freezes")
    }

    /// A base frozen under caller-supplied content addressing: two content-id IRIs
    /// and a derivation annotation between them.
    fn content_addressed_base() -> Arc<RdfDataset> {
        let scheme = crate::ContentIdScheme::new("blake3:").expect("valid scheme");
        let mut b = RdfDatasetBuilder::with_content_addressing(
            scheme,
            Some("http://example.org/derivedFrom".into()),
        );
        let a = b.intern_iri(CONTENT_A);
        let bb = b.intern_iri(CONTENT_B);
        let p = b.intern_iri("http://example.org/p");
        let derived_from = b.intern_iri("http://example.org/derivedFrom");
        b.push_quad(a, p, bb, None);
        b.push_annotation(a, derived_from, bb);
        b.freeze().expect("base freezes")
    }

    const CONTENT_A: &str =
        "blake3:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const CONTENT_B: &str =
        "blake3:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const CONTENT_C: &str =
        "blake3:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

    /// The base's content addressing survives `freeze` — after a mutation and for a
    /// no-op freeze alike — and governs the delta a snapshot freezes: a content-id IRI
    /// the mutation added is recognized, and the derivation index still resolves.
    #[test]
    fn freeze_and_snapshot_keep_the_base_content_addressing() {
        let base = content_addressed_base();
        let scheme = base.content_id_scheme().cloned();
        assert!(
            scheme.is_some(),
            "fixture is degenerate — the base is configured"
        );

        let untouched = MutableDataset::new(Arc::clone(&base));
        let frozen = untouched.freeze().expect("no-op freeze");
        assert_eq!(frozen.content_id_scheme(), scheme.as_ref(), "no-op freeze");
        assert_eq!(frozen.content_ids().count(), 2, "no-op freeze");
        let a = frozen.term_id_by_iri(CONTENT_A).expect("A survives");
        let b = frozen.term_id_by_iri(CONTENT_B).expect("B survives");
        assert_eq!(frozen.predecessors(a), &[b], "no-op freeze");

        let mut mutated = MutableDataset::new(base);
        ins(
            &mut mutated,
            QuadValues {
                s: TermValue::Iri(CONTENT_C.into()),
                p: iri_val("p"),
                o: TermValue::Iri(CONTENT_A.into()),
                g: None,
            },
        );
        let frozen = mutated.freeze().expect("mutated freeze");
        assert_eq!(
            frozen.content_id_scheme(),
            scheme.as_ref(),
            "mutated freeze"
        );
        assert_eq!(frozen.content_ids().count(), 3, "mutated freeze");
        let a = frozen.term_id_by_iri(CONTENT_A).expect("A survives");
        let b = frozen.term_id_by_iri(CONTENT_B).expect("B survives");
        assert_eq!(frozen.predecessors(a), &[b], "mutated freeze");

        let view = mutated.snapshot_view().expect("snapshot");
        assert_eq!(
            view.delta().content_id_scheme(),
            scheme.as_ref(),
            "snapshot"
        );
        let c = view
            .delta()
            .term_id_by_iri(CONTENT_C)
            .expect("C is a delta term");
        assert!(
            view.delta().content_id(c).is_some(),
            "snapshot recognizes C"
        );
    }

    /// A derivation predicate configured on the base but never interned there still
    /// governs what a mutation adds: `C rdf:reifies <<( x y z )>>` and
    /// `C derivedFrom A`, inserted through the mutable dataset, freeze into a
    /// derivation of `C` from `A`. The neighbouring case interns the predicate in the
    /// base and gives the same answer.
    #[test]
    fn a_configured_but_unused_derivation_predicate_survives_freeze() {
        const DERIVED_FROM: &str = "http://example.org/derivedFrom";
        for interned_in_base in [false, true] {
            let scheme = crate::ContentIdScheme::new("blake3:").expect("valid scheme");
            let mut b =
                RdfDatasetBuilder::with_content_addressing(scheme, Some(DERIVED_FROM.into()));
            let a = b.intern_iri(CONTENT_A);
            let p = b.intern_iri("http://example.org/p");
            let object = if interned_in_base {
                b.intern_iri(DERIVED_FROM)
            } else {
                b.intern_iri("http://example.org/o")
            };
            b.push_quad(a, p, object, None);
            let base = b.freeze().expect("base freezes");
            assert_eq!(
                base.derivation_predicate().is_some(),
                interned_in_base,
                "fixture: the predicate is interned in the base exactly when asked"
            );

            let mut m = MutableDataset::new(base);
            let c = TermValue::Iri(CONTENT_C.into());
            ins(
                &mut m,
                QuadValues {
                    s: c.clone(),
                    p: TermValue::Iri(RDF_REIFIES.into()),
                    o: TermValue::Triple {
                        s: TermBox::new(iri_val("x")),
                        p: TermBox::new(iri_val("y")),
                        o: TermBox::new(iri_val("z")),
                    },
                    g: None,
                },
            );
            ins(
                &mut m,
                QuadValues {
                    s: c,
                    p: TermValue::Iri(DERIVED_FROM.into()),
                    o: TermValue::Iri(CONTENT_A.into()),
                    g: None,
                },
            );
            let frozen = m.freeze().expect("freeze");
            let c = frozen.term_id_by_iri(CONTENT_C).expect("C survives");
            let a = frozen.term_id_by_iri(CONTENT_A).expect("A survives");
            assert_eq!(
                frozen.predecessors(c),
                &[a],
                "interned_in_base={interned_in_base}"
            );
        }
    }

    /// The neighbouring case: a base with no content addressing freezes and
    /// snapshots with none, rather than gaining a fabricated scheme.
    #[test]
    fn freeze_and_snapshot_of_an_unconfigured_base_stay_unconfigured() {
        let mut m = MutableDataset::new(base3());
        ins(
            &mut m,
            QuadValues {
                s: TermValue::Iri(CONTENT_C.into()),
                p: iri_val("p"),
                o: iri_val("a"),
                g: None,
            },
        );
        let frozen = m.freeze().expect("freeze");
        assert!(frozen.content_id_scheme().is_none());
        assert_eq!(frozen.content_ids().count(), 0);
        assert!(
            m.snapshot_view()
                .expect("snapshot")
                .delta()
                .content_id_scheme()
                .is_none()
        );
    }

    /// The effective value-quad set as a comparable `BTreeSet` of stringy tuples.
    fn eff_set(m: &MutableDataset) -> std::collections::BTreeSet<String> {
        m.effective_value_quads()
            .iter()
            .map(|q| format!("{:?}|{:?}|{:?}|{:?}", q.s, q.p, q.o, q.g))
            .collect()
    }

    #[test]
    fn blank_identity_visit_retains_suppressed_and_removed_nested_values() {
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_blank("base", crate::BlankScope::DEFAULT);
        let predicate = builder.intern_iri("http://example.org/p");
        let object = builder.intern_iri("http://example.org/o");
        builder.push_quad(subject, predicate, object, None);
        builder.intern_blank("unused", crate::BlankScope(5));
        let mut mutable = MutableDataset::new(builder.freeze().expect("base freezes"));
        assert!(mutable.remove(&QuadValues::triple(
            TermValue::blank("base"),
            iri_val("p"),
            iri_val("o"),
        )));
        let delta = QuadValues::triple(
            iri_val("holder"),
            iri_val("p"),
            TermValue::Triple {
                s: TermBox::new(TermValue::blank("delta")),
                p: TermBox::new(iri_val("p")),
                o: TermBox::new(TermValue::typed_literal(
                    "[_:embedded]",
                    purrdf_cdt::CDT_LIST,
                )),
            },
        );
        assert!(ins(&mut mutable, delta.clone()));
        assert!(mutable.remove(&delta));
        assert_eq!(mutable.effective_value_quads(), []);

        let mut identities = std::collections::BTreeSet::new();
        let complete = mutable.visit_blank_identities(|label, scope| {
            identities.insert((label.to_owned(), scope));
            ControlFlow::<Infallible>::Continue(())
        });
        assert_eq!(complete, ControlFlow::Continue(()));
        assert_eq!(
            identities,
            [
                ("base".to_owned(), crate::BlankScope::DEFAULT),
                ("unused".to_owned(), crate::BlankScope(5)),
                ("delta".to_owned(), crate::BlankScope::DEFAULT),
                ("embedded".to_owned(), crate::BlankScope::DEFAULT),
            ]
            .into_iter()
            .collect()
        );
        let mut calls = 0;
        assert_eq!(
            mutable.visit_blank_identities(|label, scope| {
                calls += 1;
                ControlFlow::Break((label.to_owned(), scope))
            }),
            ControlFlow::Break(("base".to_owned(), crate::BlankScope::DEFAULT))
        );
        assert_eq!(calls, 1);
    }

    /// `base_value_of` and [`RdfDataset::term_value`] are the two resolvers of a
    /// frozen literal's datatype, and they must agree on every literal shape: the
    /// datatype handed back is the IRI that was interned, never a rendering of some
    /// other term. A non-IRI datatype id is unreachable from the public API (the
    /// builder interns every datatype through `intern_iri`; the pack decoder refuses
    /// a non-IRI datatype entry), so the pinned behaviour is the agreement itself —
    /// both sites state the invariant with `unreachable!`, and neither fabricates a
    /// value that would only fail later as an `IriError` about text nobody wrote.
    #[test]
    fn base_value_of_agrees_with_term_value_on_every_literal_shape() {
        use crate::model::RdfTextDirection;

        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("http://example.org/s");
        let p = b.intern_iri("http://example.org/p");
        let plain = b.intern_literal(RdfLiteral::simple("plain"));
        let typed = b.intern_literal(RdfLiteral::typed(
            "1",
            "http://www.w3.org/2001/XMLSchema#integer",
        ));
        let lang = b.intern_literal(RdfLiteral::language_tagged("hi", "en"));
        let directional = b.intern_literal(RdfLiteral {
            direction: Some(RdfTextDirection::Rtl),
            ..RdfLiteral::language_tagged("مرحبا", "ar")
        });
        // A literal nested inside a triple term resolves through the triple-term arm.
        let nested = b.intern_triple(s, p, typed);
        for o in [plain, typed, lang, directional, nested] {
            b.push_quad(s, p, o, None);
        }
        let base = b.freeze().expect("base freezes");

        let mut literal_datatypes = Vec::new();
        for index in 0..base.term_count() {
            let id = TermId::from_index(u32::try_from(index).expect("small fixture"));
            let via_mutable = MutableDataset::base_value_of(&base, id);
            assert_eq!(via_mutable, base.term_value(id), "term {index}");
            if let TermValue::Literal { datatype, .. } = via_mutable {
                literal_datatypes.push(datatype);
            }
        }
        literal_datatypes.sort();
        assert_eq!(
            literal_datatypes,
            [
                "http://www.w3.org/1999/02/22-rdf-syntax-ns#dirLangString",
                "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString",
                "http://www.w3.org/2001/XMLSchema#integer",
                "http://www.w3.org/2001/XMLSchema#string",
            ]
            .map(str::to_owned),
            "every literal datatype is the interned IRI, never a Debug rendering"
        );
    }

    // -- the four mutation rules ------------------------------------------------------

    #[test]
    fn rule1_insert_of_suppressed_base_unsuppresses() {
        let mut m = MutableDataset::new(base3());
        let quad = q("a", "p", "b"); // a base quad
        assert!(m.contains(&quad));
        // Remove → suppression.
        assert!(m.remove(&quad));
        assert_eq!(m.suppressed_len(), 1);
        assert!(!m.contains(&quad));
        // Insert of the suppressed base quad un-suppresses, does NOT add.
        assert!(ins(&mut m, quad.clone()));
        assert_eq!(m.suppressed_len(), 0, "un-suppressed");
        assert_eq!(m.added_len(), 0, "not pushed to added");
        assert!(m.contains(&quad));
    }

    #[test]
    fn rule2_remove_of_delta_added_drops_from_added() {
        let mut m = MutableDataset::new(base3());
        let quad = q("x", "p", "y"); // brand-new (delta) quad
        assert!(ins(&mut m, quad.clone()));
        assert_eq!(m.added_len(), 1);
        assert!(m.contains(&quad));
        // Remove of a delta-added quad drops it from `added`, NO suppression.
        assert!(m.remove(&quad));
        assert_eq!(m.added_len(), 0, "dropped from added");
        assert_eq!(m.suppressed_len(), 0, "no suppression created");
        assert!(!m.contains(&quad));
    }

    #[test]
    fn rule3_remove_of_base_quad_creates_suppression() {
        let mut m = MutableDataset::new(base3());
        let quad = q("a", "p", "c"); // a base quad
        assert!(m.contains(&quad));
        assert!(m.remove(&quad));
        assert_eq!(m.suppressed_len(), 1, "suppression created");
        assert_eq!(m.added_len(), 0);
        assert!(!m.contains(&quad));
        // Removing it again is a no-op (already suppressed).
        assert!(!m.remove(&quad));
        assert_eq!(m.suppressed_len(), 1);
    }

    #[test]
    fn rule4_reinsert_after_removal_both_orders() {
        // insert → remove → insert returns to present (delta quad).
        let mut m = MutableDataset::new(base3());
        let nq = q("n", "p", "m");
        assert!(ins(&mut m, nq.clone()));
        assert!(m.remove(&nq));
        assert!(ins(&mut m, nq.clone()));
        assert!(m.contains(&nq));
        assert_eq!(m.added_len(), 1);
        assert_eq!(m.suppressed_len(), 0);

        // remove → insert returns to present (base quad).
        let mut m = MutableDataset::new(base3());
        let bq = q("b", "p", "c");
        assert!(m.remove(&bq));
        assert!(!m.contains(&bq));
        assert!(ins(&mut m, bq.clone()));
        assert!(m.contains(&bq));
        assert_eq!(m.suppressed_len(), 0);
        assert_eq!(m.added_len(), 0, "base quad re-presented by un-suppress");
    }

    #[test]
    fn insert_existing_base_quad_is_noop() {
        let mut m = MutableDataset::new(base3());
        let quad = q("a", "p", "b");
        assert!(!ins(&mut m, quad), "already effective → no change");
        assert_eq!(m.added_len(), 0);
    }

    // -- contains / quads_for_pattern reflect the effective set -----------------------

    #[test]
    fn contains_and_pattern_reflect_effective_set() {
        let mut m = MutableDataset::new(base3());
        // Add a quad, remove a base quad.
        ins(&mut m, q("z", "p", "w"));
        m.remove(&q("a", "p", "b"));

        assert!(m.contains(&q("z", "p", "w")));
        assert!(!m.contains(&q("a", "p", "b")));
        assert!(m.contains(&q("a", "p", "c")));

        // Pattern: all quads with predicate p.
        let all_p = m.quads_for_pattern(None, Some(&iri_val("p")), None, GraphMatchValue::Any);
        // Effective: (a,p,c), (b,p,c), (z,p,w) = 3.
        assert_eq!(all_p.len(), 3);

        // Pattern bound on a delta subject.
        let zq = m.quads_for_pattern(Some(&iri_val("z")), None, None, GraphMatchValue::Any);
        assert_eq!(zq.len(), 1);
        assert_eq!(zq[0], q("z", "p", "w"));

        // Pattern bound on the now-suppressed quad yields nothing.
        let gone = m.quads_for_pattern(
            Some(&iri_val("a")),
            Some(&iri_val("p")),
            Some(&iri_val("b")),
            GraphMatchValue::Any,
        );
        assert_eq!(gone, [] as [_; 0]);

        // A bound value interned nowhere matches nothing.
        let nothing = m.quads_for_pattern(
            Some(&iri_val("never-seen")),
            None,
            None,
            GraphMatchValue::Any,
        );
        assert_eq!(nothing, [] as [_; 0]);
    }

    #[test]
    fn named_graph_quads_round_trip() {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("http://example.org/s");
        let p = b.intern_iri("http://example.org/p");
        let o = b.intern_iri("http://example.org/o");
        let g = b.intern_iri("http://example.org/g");
        b.push_quad(s, p, o, Some(g));
        let base = b.freeze().unwrap();

        let mut m = MutableDataset::new(base);
        // Add a named-graph quad in a NEW delta graph.
        let nq = QuadValues::quad(iri_val("s2"), iri_val("p"), iri_val("o2"), iri_val("g2"));
        assert!(ins(&mut m, nq.clone()));
        assert!(m.contains(&nq));

        // Default-graph match excludes both named quads.
        let dflt = m.quads_for_pattern(None, None, None, GraphMatchValue::Default);
        assert_eq!(dflt, [] as [_; 0]);
        // Any matches both.
        let any = m.quads_for_pattern(None, None, None, GraphMatchValue::Any);
        assert_eq!(any.len(), 2);
    }

    #[test]
    fn quads_for_pattern_matches_delta_only_named_graph() {
        // A base whose graph term `g2` does NOT exist — branch off it, then insert a
        // quad into a brand-new named graph `g2`. The graph term is delta-only (no
        // base TermId), so a TermId-keyed filter could never name it; the value-based
        // GraphMatchValue::Named resolves it via the delta interner.
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("http://example.org/s");
        let p = b.intern_iri("http://example.org/p");
        let o = b.intern_iri("http://example.org/o");
        b.push_quad(s, p, o, None); // default-graph base quad; no g2 anywhere
        let base = b.freeze().unwrap();

        let mut m = MutableDataset::new(base);
        let g2 = iri_val("g2"); // a graph term interned NOWHERE in the base
        let nq = QuadValues::quad(iri_val("s2"), iri_val("p"), iri_val("o2"), g2.clone());
        assert!(ins(&mut m, nq.clone()));

        // Query the delta-only named graph by VALUE — it must return that quad.
        let hits = m.quads_for_pattern(None, None, None, GraphMatchValue::Named(&g2));
        assert_eq!(hits.len(), 1, "delta-only named graph is now queryable");
        assert_eq!(hits[0], nq);

        // A graph value interned nowhere still matches nothing.
        let none = m.quads_for_pattern(None, None, None, GraphMatchValue::Named(&iri_val("nope")));
        assert_eq!(none, [] as [_; 0]);
    }

    // -- freeze round-trip ------------------------------------------------------------

    #[test]
    fn freeze_round_trip_quads_reifiers_annotations() {
        let mut m = MutableDataset::new(base3());
        // Insert a brand-new quad with a brand-new term, remove a base quad.
        ins(&mut m, q("new", "p", "thing"));
        m.remove(&q("a", "p", "b"));

        let want = eff_set(&m);
        let frozen = m.freeze().expect("freeze compacts");

        // The frozen quad set equals the effective set (compared by value).
        let frozen_set: std::collections::BTreeSet<String> = frozen
            .quads()
            .chain(frozen.reifier_quads())
            .chain(frozen.annotation_quads())
            .map(|qd| {
                let s = MutableDataset::base_value_of(&frozen, qd.s);
                let p = MutableDataset::base_value_of(&frozen, qd.p);
                let o = MutableDataset::base_value_of(&frozen, qd.o);
                let g = qd.g.map(|g| MutableDataset::base_value_of(&frozen, g));
                format!("{s:?}|{p:?}|{o:?}|{g:?}")
            })
            .collect();
        assert_eq!(frozen_set, want);

        // Reifiers + annotations survived (remapped, not lost).
        assert_eq!(
            frozen.reifiers().count(),
            1,
            "reifier carried through freeze"
        );
        assert_eq!(
            frozen.annotations().count(),
            1,
            "annotation carried through freeze"
        );

        // All term ids are valid/dense (every quad component < term_count).
        let tc = frozen.term_count();
        for qd in frozen.quads() {
            assert!(qd.s.index() < tc);
            assert!(qd.p.index() < tc);
            assert!(qd.o.index() < tc);
            if let Some(g) = qd.g {
                assert!(g.index() < tc);
            }
        }

        // The annotation predicate survived as a real, remapped IRI (full remap), and
        // the annotation references it.
        let conf = frozen
            .term_id_by_value(&iri_val("confidence"))
            .expect("confidence iri remapped");
        assert_eq!(
            frozen.annotations().filter(|(_, p, _)| *p == conf).count(),
            1,
            "the decimal annotation survived remap, keyed on the remapped predicate"
        );
    }

    /// Regression pin for the process-nondeterministic scan order defect: `freeze()`
    /// must replay delta-added quads in CALL order, never `added`'s hash-iteration
    /// order — because a bare `std::collections::HashSet<QuadKey>` (the pre-fix
    /// `added`/`suppressed` type) draws a fresh, process-random hasher key
    /// EVERY time `HashSet::new()` runs (a per-call counter seeded once per thread
    /// from OS randomness), so two `MutableDataset`s built from the identical
    /// insertion sequence — even in the SAME process — could iterate `added` in
    /// different orders. That reordered which brand-new term got which dense
    /// `TermId` at `freeze()` time, and the frozen dataset sorts quads BY `TermId`,
    /// so the reordering was directly observable as scan-order drift downstream
    /// (`GROUP_CONCAT`, `FIRST`/`LAST`, `TOPK`, and plain projection order all read
    /// that scan).
    ///
    /// This test defeats exactly that per-construction seeding: it builds MANY fresh
    /// `MutableDataset`s (each its own `HashSet::new()` call, had the bug still been
    /// present) off the SAME base, replays the SAME scrambled insertion sequence of
    /// brand-new subjects into each, freezes each, and asserts every one yields the
    /// bitwise-identical subject order. Against the pre-fix implementation this loop
    /// is expected to observe at least two different orderings; against the fix it
    /// always sees exactly one.
    #[test]
    fn freeze_replays_delta_insertions_in_call_order_across_fresh_datasets() {
        // A scrambled (neither insertion-adjacent-sorted nor alphabetical) subject
        // sequence, so an order bug can't hide behind a coincidental match.
        let order = ["s7", "s2", "s9", "s0", "s5", "s3", "s8", "s1", "s6", "s4"];

        let mut orderings: std::collections::BTreeSet<Vec<String>> =
            std::collections::BTreeSet::new();
        for _ in 0..25 {
            let mut m = MutableDataset::new(base3());
            for s in order {
                assert!(ins(&mut m, q(s, "brand-new-pred", "o")));
            }
            let frozen = m.freeze().expect("freeze compacts");

            // Read back the delta-added quads' subjects in the frozen dataset's own
            // scan order (`quads()`, id-ascending — the same order a full BGP scan
            // over this dataset walks).
            let pred = frozen
                .term_id_by_value(&iri_val("brand-new-pred"))
                .expect("the shared new predicate is interned");
            let mut subjects: Vec<String> = Vec::new();
            for qd in frozen.quads() {
                if qd.p != pred {
                    continue;
                }
                let TermValue::Iri(iri) = MutableDataset::base_value_of(&frozen, qd.s) else {
                    panic!("subject must be an IRI");
                };
                subjects.push(iri);
            }
            orderings.insert(subjects);
        }

        assert_eq!(
            orderings.len(),
            1,
            "every fresh MutableDataset replayed the SAME insertion sequence into \
             the SAME base, so freeze()'s scan order must be identical every time — \
             observed {} distinct orderings: {orderings:#?}",
            orderings.len()
        );
        let only = orderings.into_iter().next().unwrap();
        let want: Vec<String> = order
            .iter()
            .map(|s| format!("http://example.org/{s}"))
            .collect();
        assert_eq!(
            only, want,
            "scan order must equal insertion (call) order, not hash-bucket order"
        );
    }

    #[test]
    fn freeze_empty_mutation_equals_base_quads() {
        let m = MutableDataset::new(base3());
        let frozen = m.freeze().unwrap();
        assert_eq!(frozen.quad_count(), 3, "no mutation → same quads");
        assert_eq!(frozen.reifiers().count(), 1);
        assert_eq!(frozen.annotations().count(), 1);
    }

    #[test]
    fn freeze_carries_base_quad_locations() {
        use crate::RdfLocation;

        // A base with two located quads. We attach a location to ONE of them, then
        // after a delta insert + a DIFFERENT-quad suppression, freeze must preserve
        // the surviving located quad's location (across the base-ord → new-handle →
        // frozen-sort remap) while dropping the suppressed one.
        let mut b = RdfDatasetBuilder::new();
        let a = b.intern_iri("http://example.org/a");
        let p = b.intern_iri("http://example.org/p");
        let bb = b.intern_iri("http://example.org/b");
        let c = b.intern_iri("http://example.org/c");

        // (a,p,b) gets a location; (a,p,c) does not.
        let h_ab = b.next_quad_handle();
        b.push_quad(a, p, bb, None);
        b.push_quad(a, p, c, None);
        b.attach_location(h_ab, RdfLocation::logical("loc-a-p-b"));
        let base = b.freeze().expect("base freezes");

        let mut m = MutableDataset::new(base);
        // Insert a brand-new delta quad and suppress a DIFFERENT base quad (a,p,c).
        ins(&mut m, q("x", "p", "y"));
        assert!(m.remove(&q("a", "p", "c")));

        let frozen = m.freeze().expect("freeze compacts");

        // The surviving located base quad (a,p,b) STILL carries its location, found at
        // its frozen position.
        let a2 = frozen.term_id_by_value(&iri_val("a")).expect("a remapped");
        let p2 = frozen.term_id_by_value(&iri_val("p")).expect("p remapped");
        let b2 = frozen.term_id_by_value(&iri_val("b")).expect("b remapped");
        let frozen_ab = frozen
            .quads()
            .position(|qd| qd.s == a2 && qd.p == p2 && qd.o == b2 && qd.g.is_none())
            .expect("(a,p,b) survives");
        assert_eq!(
            frozen
                .location_of(QuadHandle::from_index(frozen_ab as u32))
                .and_then(|l| l.logical.as_deref()),
            Some("loc-a-p-b"),
            "the surviving base quad keeps its location through freeze"
        );

        // The suppressed quad (a,p,c) is gone.
        assert!(!m.contains(&q("a", "p", "c")));
        assert!(
            frozen.term_id_by_value(&iri_val("c")).is_none(),
            "the suppressed quad's unique object term is no longer interned"
        );
    }

    // -- branch / handle stability ----------------------------------------------------

    #[test]
    fn two_branches_mutate_independently() {
        let base = base3();
        let mut m1 = MutableDataset::new(Arc::clone(&base));
        let mut m2 = MutableDataset::new(Arc::clone(&base));

        ins(&mut m1, q("only", "in", "one"));
        m2.remove(&q("a", "p", "b"));

        // m1 sees its add, not m2's removal.
        assert!(m1.contains(&q("only", "in", "one")));
        assert!(m1.contains(&q("a", "p", "b")));
        // m2 sees its removal, not m1's add.
        assert!(!m2.contains(&q("only", "in", "one")));
        assert!(!m2.contains(&q("a", "p", "b")));
        // The shared base is untouched: it still has 3 quads, addressable directly.
        assert_eq!(base.quad_count(), 3);
        assert_eq!(
            Arc::strong_count(&base),
            3,
            "base shared by 2 branches + local"
        );
    }

    #[test]
    fn should_compact_signals_on_churn() {
        let mut m = MutableDataset::new(base3()); // 3 ordinary + 2 statement rows
        assert!(!m.should_compact());
        ins(&mut m, q("e1", "p", "o"));
        ins(&mut m, q("e2", "p", "o"));
        assert!(!m.should_compact()); // 2 * 2 <= 5 retained RDF rows
        ins(&mut m, q("e3", "p", "o"));
        assert!(m.should_compact()); // 3 * 2 > 5
    }

    // -- named-graph existence --------------------------------------------------------

    /// A base holding a default-graph quad, a declared EMPTY graph `empty`, a graph
    /// `one` with a single quad, a graph `two` with two quads, and a graph `stmt`
    /// whose only row is a reifier binding.
    fn graph_base() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let a = b.intern_iri("http://example.org/a");
        let p = b.intern_iri("http://example.org/p");
        let c = b.intern_iri("http://example.org/c");
        let d = b.intern_iri("http://example.org/d");
        let empty = b.intern_iri("http://example.org/empty");
        let one = b.intern_iri("http://example.org/one");
        let two = b.intern_iri("http://example.org/two");
        let stmt = b.intern_iri("http://example.org/stmt");
        let r = b.intern_iri("http://example.org/r");
        b.push_quad(a, p, c, None);
        b.declare_named_graph(empty);
        b.push_quad(a, p, c, Some(one));
        b.push_quad(a, p, c, Some(two));
        b.push_quad(a, p, d, Some(two));
        let triple = b.intern_triple(a, p, c);
        b.push_reifier_in_graph(r, triple, Some(stmt));
        b.freeze().expect("graph base freezes")
    }

    fn in_graph(s: &str, o: &str, g: &str) -> QuadValues {
        QuadValues::quad(iri_val(s), iri_val("p"), iri_val(o), iri_val(g))
    }

    fn reifier_in(g: &str) -> QuadValues {
        QuadValues::quad(
            iri_val("r"),
            TermValue::Iri(RDF_REIFIES.to_owned()),
            TermValue::Triple {
                s: TermBox::new(iri_val("a")),
                p: TermBox::new(iri_val("p")),
                o: TermBox::new(iri_val("c")),
            },
            iri_val(g),
        )
    }

    /// The local names of every named graph a snapshot and a freeze enumerate,
    /// asserted equal to each other.
    fn graph_names(m: &MutableDataset) -> Vec<String> {
        use crate::DatasetView as _;
        let local = |v: TermValue| match v {
            TermValue::Iri(iri) => iri.trim_start_matches("http://example.org/").to_owned(),
            other => panic!("graph names here are IRIs, not {other:?}"),
        };
        let view = m.snapshot_view().expect("snapshot publishes");
        // Constant `GRAPH <g>` addressing answers membership through
        // `has_named_graph`; it must agree with the enumeration for every term the
        // snapshot holds — a withdrawn graph is no graph, a repopulated one is.
        let enumerated: std::collections::BTreeSet<_> = view.named_graphs().collect();
        for id in view.term_ids() {
            assert_eq!(
                view.has_named_graph(id),
                enumerated.contains(&id),
                "has_named_graph({:?}) disagrees with named_graphs",
                view.term_value(id)
            );
        }
        let mut from_view: Vec<String> = view
            .named_graphs()
            .map(|g| local(view.term_value(g)))
            .collect();
        from_view.sort();
        let frozen = m.freeze().expect("freeze");
        let mut from_freeze: Vec<String> = frozen
            .named_graphs()
            .map(|g| local(RdfDataset::term_value(&frozen, g)))
            .collect();
        from_freeze.sort();
        assert_eq!(
            from_view, from_freeze,
            "snapshot and freeze enumerate alike"
        );
        from_view
    }

    #[test]
    fn an_untouched_dataset_enumerates_every_base_graph_including_the_declared_empty_one() {
        let m = MutableDataset::new(graph_base());
        assert_eq!(graph_names(&m), ["empty", "one", "stmt", "two"]);
    }

    #[test]
    fn removing_the_last_quad_of_a_graph_withdraws_the_graph() {
        let mut m = MutableDataset::new(graph_base());
        assert!(m.remove(&in_graph("a", "c", "one")));
        assert_eq!(graph_names(&m), ["empty", "stmt", "two"]);
    }

    #[test]
    fn removing_some_but_not_all_quads_of_a_graph_keeps_it() {
        let mut m = MutableDataset::new(graph_base());
        assert!(m.remove(&in_graph("a", "c", "two")));
        assert_eq!(graph_names(&m), ["empty", "one", "stmt", "two"]);
        assert!(m.remove(&in_graph("a", "d", "two")));
        assert_eq!(graph_names(&m), ["empty", "one", "stmt"]);
    }

    #[test]
    fn removing_the_only_reifier_of_a_graph_withdraws_the_graph() {
        let mut m = MutableDataset::new(graph_base());
        assert!(m.remove(&reifier_in("stmt")));
        assert_eq!(graph_names(&m), ["empty", "one", "two"]);
    }

    #[test]
    fn a_graph_repopulated_after_losing_its_last_quad_exists() {
        // Re-inserting the same base quad (un-suppression) and inserting a different
        // quad (a delta row in a base graph) both bring the graph back.
        let mut m = MutableDataset::new(graph_base());
        assert!(m.remove(&in_graph("a", "c", "one")));
        assert!(ins(&mut m, in_graph("a", "c", "one")));
        assert_eq!(graph_names(&m), ["empty", "one", "stmt", "two"]);
        assert!(m.remove(&in_graph("a", "c", "one")));
        assert!(ins(&mut m, in_graph("x", "y", "one")));
        assert_eq!(graph_names(&m), ["empty", "one", "stmt", "two"]);
    }

    #[test]
    fn an_unrelated_insert_keeps_the_declared_empty_graph() {
        let mut m = MutableDataset::new(graph_base());
        assert!(ins(&mut m, q("x", "p", "y")));
        assert!(ins(&mut m, in_graph("x", "y", "fresh")));
        assert_eq!(graph_names(&m), ["empty", "fresh", "one", "stmt", "two"]);
    }

    #[test]
    fn a_delta_graph_emptied_again_is_not_enumerated() {
        let mut m = MutableDataset::new(graph_base());
        assert!(ins(&mut m, in_graph("x", "y", "fresh")));
        assert!(m.remove(&in_graph("x", "y", "fresh")));
        assert_eq!(graph_names(&m), ["empty", "one", "stmt", "two"]);
    }

    #[test]
    fn emptying_a_declared_empty_graph_after_populating_it_withdraws_it() {
        // The declared empty graph gains a quad and then loses it: the removal of its
        // last quad is an operation that leaves it empty, so it is withdrawn.
        let mut m = MutableDataset::new(graph_base());
        assert!(ins(&mut m, in_graph("x", "y", "empty")));
        assert_eq!(graph_names(&m), ["empty", "one", "stmt", "two"]);
        assert!(m.remove(&in_graph("x", "y", "empty")));
        assert_eq!(graph_names(&m), ["one", "stmt", "two"]);
    }

    #[test]
    fn withdrawing_a_declaration_hides_only_an_empty_graph() {
        let mut m = MutableDataset::new(graph_base());
        // A graph that still holds rows is unaffected by a withdrawal.
        m.withdraw_graph_declaration(&iri_val("two"));
        // A graph the dataset never knew is a no-op, not an error.
        m.withdraw_graph_declaration(&iri_val("never"));
        assert_eq!(graph_names(&m), ["empty", "one", "stmt", "two"]);
        m.withdraw_graph_declaration(&iri_val("empty"));
        assert_eq!(graph_names(&m), ["one", "stmt", "two"]);
        // Populating it again brings it back.
        assert!(ins(&mut m, in_graph("x", "y", "empty")));
        assert_eq!(graph_names(&m), ["empty", "one", "stmt", "two"]);
    }

    #[test]
    fn a_graph_is_emptied_only_when_its_quads_reifiers_and_annotations_are_all_gone() {
        // `mix` holds a plain quad, a reifier binding and that reifier's annotation.
        let mut b = RdfDatasetBuilder::new();
        let a = b.intern_iri("http://example.org/a");
        let p = b.intern_iri("http://example.org/p");
        let c = b.intern_iri("http://example.org/c");
        let r = b.intern_iri("http://example.org/r");
        let mix = b.intern_iri("http://example.org/mix");
        b.push_quad(a, p, c, Some(mix));
        let triple = b.intern_triple(a, p, c);
        b.push_reifier_in_graph(r, triple, Some(mix));
        b.push_annotation_in_graph(r, p, c, Some(mix));
        let base = b.freeze().expect("mixed base freezes");
        let annotation = in_graph("r", "c", "mix");
        let mut m = MutableDataset::new(base);
        assert!(m.remove(&in_graph("a", "c", "mix")));
        assert_eq!(graph_names(&m), ["mix"], "a reifier and annotation remain");
        assert!(m.remove(&reifier_in("mix")));
        assert_eq!(graph_names(&m), ["mix"], "the (demoted) annotation remains");
        assert!(m.remove(&annotation));
        assert!(graph_names(&m).is_empty(), "every row is gone");
        // Order does not matter: the annotation first, then the reifier.
        let mut m = MutableDataset::new(m.base().clone());
        assert!(m.remove(&annotation));
        assert!(m.remove(&in_graph("a", "c", "mix")));
        assert_eq!(graph_names(&m), ["mix"], "the reifier remains");
        assert!(m.remove(&reifier_in("mix")));
        assert!(graph_names(&m).is_empty(), "every row is gone");
    }

    #[test]
    fn withdrawing_every_declaration_keeps_only_populated_graphs() {
        let mut m = MutableDataset::new(graph_base());
        m.withdraw_named_graph_declarations();
        assert_eq!(graph_names(&m), ["one", "stmt", "two"]);
        // The default graph is not a named graph and is untouched.
        assert_eq!(m.freeze().expect("freeze").quad_count(), 4);
    }

    // Named-graph existence against a full-scan oracle: random inserts, removals and
    // withdrawals over a base with quads, reifier bindings, annotations and a declared
    // empty graph. A graph is enumerated iff it holds an effective row, or it is a base
    // graph the model has not seen emptied since its last row (or withdrawn while
    // empty).
    prop_test! {
        #[test]
        fn property_graph_enumeration_matches_a_full_scan(
            ops in prop::collection::vec((0u8..3, 0u8..4, 0u8..4), 0..40)
        ) {
            const GRAPHS: [&str; 4] = ["empty", "one", "two", "stmt"];
            let base = graph_base();
            let base_graphs: std::collections::BTreeSet<String> = GRAPHS
                .iter()
                .map(|g| (*g).to_owned())
                .collect();
            let mut m = MutableDataset::new(base);
            let mut emptied = std::collections::BTreeSet::<String>::new();
            let local = |v: &TermValue| match v {
                TermValue::Iri(iri) => iri.trim_start_matches("http://example.org/").to_owned(),
                other => panic!("graph names here are IRIs, not {other:?}"),
            };
            for (kind, row, graph) in ops {
                let g = GRAPHS[usize::from(graph)];
                let quad = match row {
                    0 => in_graph("a", "c", g),
                    1 => in_graph("a", "d", g),
                    2 => reifier_in(g),
                    // In `stmt` this row is an annotation of the base reifier `r`.
                    _ => in_graph("r", "c", g),
                };
                let holds = |m: &MutableDataset| {
                    m.effective_value_quads()
                        .iter()
                        .any(|q| q.g.as_ref().map(local).as_deref() == Some(g))
                };
                match kind {
                    0 => {
                        if ins(&mut m, quad) {
                            emptied.remove(g);
                        }
                    }
                    1 => {
                        if m.remove(&quad) && !holds(&m) {
                            emptied.insert(g.to_owned());
                        }
                    }
                    _ => {
                        m.withdraw_graph_declaration(&iri_val(g));
                        if !holds(&m) {
                            emptied.insert(g.to_owned());
                        }
                    }
                }
                let mut expected: std::collections::BTreeSet<String> = m
                    .effective_value_quads()
                    .iter()
                    .filter_map(|q| q.g.as_ref().map(local))
                    .collect();
                expected.extend(base_graphs.difference(&emptied).cloned());
                let actual: std::collections::BTreeSet<String> =
                    graph_names(&m).into_iter().collect();
                prop_assert_eq!(actual, expected);
            }
        }
    }

    // -- differential property test --------------------------------------------------

    // Mirror of `property_indexed_pattern_matches_linear_scan`: apply a random
    // sequence of insert/remove ops to BOTH a `MutableDataset` and a reference
    // `HashSet` model of the effective quad-value set, then assert `contains` and the
    // effective set agree, and that `freeze()`'s quad-value set equals the model.
    prop_test! {
        #[test]
        fn property_mutations_match_hashset_model(
            ops in prop::collection::vec(
                // (is_insert, s, p, o) over a small pool; subjects/objects 0..6 so some
                // collide with the base's a/b/c terms (ids 0..2) and some are new.
                (any::<bool>(), 0u8..6, 0u8..3, 0u8..6),
                0..60,
            )
        ) {

            let names = ["a", "b", "c", "d", "e", "f"];
            let preds = ["p", "q", "r"];
            let base = base3();
            // Independent native-table oracle includes every RDF statement row.
            let mut model: HashSet<(String, String, String)> = base.quads()
                .chain(base.reifier_quads()).chain(base.annotation_quads())
                .map(|q| (val_str(&base.term_value(q.s)), val_str(&base.term_value(q.p)), val_str(&base.term_value(q.o))))
                .collect();
            let mut m = MutableDataset::new(base);

            for (is_insert, s, p, o) in ops {
                let (sn, pn, on) =
                    (names[s as usize], preds[p as usize], names[o as usize]);
                let quad = q(sn, pn, on);
                let tup = (sn.to_string(), pn.to_string(), on.to_string());
                if is_insert {
                    ins(&mut m, quad);
                    model.insert(tup);
                } else {
                    m.remove(&quad);
                    model.remove(&tup);
                }
            }

            // contains agrees across the WHOLE pool (present and absent).
            for &sn in &names {
                for &pn in &preds {
                    for &on in &names {
                        let present = model.contains(&(sn.into(), pn.into(), on.into()));
                        prop_assert_eq!(
                            m.contains(&q(sn, pn, on)),
                            present,
                            "contains disagrees for ({}, {}, {})", sn, pn, on
                        );
                    }
                }
            }

            // The effective value-quad set agrees with the model.
            let eff: HashSet<(String, String, String)> = m
                .effective_value_quads()
                .iter()
                .map(|qd| (
                    val_str(&qd.s), val_str(&qd.p), val_str(&qd.o),
                ))
                .collect();
            prop_assert_eq!(&eff, &model);

            // freeze()'s quad-value set equals the model too.
            let frozen = m.freeze().expect("freeze");
            let frozen_set: HashSet<(String, String, String)> = frozen
                .quads().chain(frozen.reifier_quads()).chain(frozen.annotation_quads())
                .map(|qd| (
                    iri_local(&MutableDataset::base_value_of(&frozen, qd.s)),
                    iri_local(&MutableDataset::base_value_of(&frozen, qd.p)),
                    iri_local(&MutableDataset::base_value_of(&frozen, qd.o)),
                ))
                .collect();
            prop_assert_eq!(frozen_set, model);
        }
    }

    /// The local suffix of an `http://example.org/<x>` IRI value, for model compare.
    fn iri_local(v: &TermValue) -> String {
        match v {
            TermValue::Iri(s) => s.rsplit('/').next().unwrap_or(s).to_string(),
            other => format!("{other:?}"),
        }
    }

    fn val_str(v: &TermValue) -> String {
        iri_local(v)
    }
}

#[cfg(test)]
mod term_walk_tests {
    //! The base-value resolution, the absoluteness check and the re-interning against
    //! their recursive references, and the latter two at a hundred thousand levels on a
    //! 128 KiB thread.

    use purrdf_iri::IriError;

    use super::{MutableDataset, check_value_absolute};
    use crate::backend::TermFactory as _;
    use crate::term_fixture::TermShape;
    use crate::{RdfDataset, RdfDatasetBuilder, TermBox, TermId, TermRef, TermValue};

    fn reference_base(base: &RdfDataset, id: TermId) -> TermValue {
        match base.resolve(id) {
            TermRef::Triple { s, p, o } => TermValue::Triple {
                s: TermBox::new(reference_base(base, s)),
                p: TermBox::new(reference_base(base, p)),
                o: TermBox::new(reference_base(base, o)),
            },
            _ => MutableDataset::base_value_of(base, id),
        }
    }

    fn reference_absolute(value: &TermValue) -> Result<(), IriError> {
        match value {
            TermValue::Triple { s, p, o } => {
                reference_absolute(s)?;
                reference_absolute(p)?;
                reference_absolute(o)
            }
            leaf => check_value_absolute(leaf),
        }
    }

    fn reference_intern(builder: &mut RdfDatasetBuilder, value: &TermValue) -> TermId {
        match value {
            TermValue::Triple { s, p, o } => {
                let s = reference_intern(builder, s);
                let p = reference_intern(builder, p);
                let o = reference_intern(builder, o);
                builder.intern_triple(s, p, o)
            }
            leaf => builder.intern_value(leaf),
        }
    }

    /// `value` with its IRI `http://example.org/i1` made relative, so the absoluteness
    /// check meets a refusal wherever that IRI sits.
    fn relativized(value: &TermValue) -> TermValue {
        value.fold(
            |leaf| match leaf {
                TermValue::Iri(iri) if iri == "http://example.org/i1" => TermValue::iri("i1"),
                other => other.clone(),
            },
            |s, p, o| TermValue::Triple {
                s: TermBox::new(s),
                p: TermBox::new(p),
                o: TermBox::new(o),
            },
        )
    }

    /// Every generated term is resolved from a base dataset, checked — as written and
    /// with a relative IRI in it — and re-interned exactly as the recursive references do.
    #[test]
    fn the_walks_agree_with_their_recursive_references_on_generated_terms() {
        let mut refused = 0;
        for seed in 0..300_u64 {
            let mut state = seed;
            let mut budget = 8;
            let value = crate::term_fixture::term_value(
                &mut state,
                purrdf_testkit::rng::splitmix64_next,
                &mut budget,
                TermShape::WellFormed,
            );
            let mut builder = RdfDatasetBuilder::new();
            let object = builder.intern_value(&value);
            let holder = builder.intern_iri("http://example.org/holder");
            builder.push_quad(holder, holder, object, None);
            let base = builder.freeze().expect("a generated term freezes");
            let object = base.quads().next().expect("one quad").o;
            assert_eq!(
                MutableDataset::base_value_of(&base, object),
                reference_base(&base, object),
                "seed {seed}"
            );
            for candidate in [value.clone(), relativized(&value)] {
                let found = check_value_absolute(&candidate);
                assert_eq!(
                    format!("{found:?}"),
                    format!("{:?}", reference_absolute(&candidate)),
                    "seed {seed}"
                );
                refused += usize::from(found.is_err());
            }
            let (mut found, mut expected) = (RdfDatasetBuilder::new(), RdfDatasetBuilder::new());
            assert_eq!(
                found.intern_value(&value),
                reference_intern(&mut expected, &value),
                "seed {seed}"
            );
            let sentinel = "http://example.org/sentinel";
            assert_eq!(
                found.intern_iri(sentinel),
                expected.intern_iri(sentinel),
                "seed {seed}"
            );
        }
        assert!(refused > 0, "some generated term holds a relative IRI");
    }

    /// A triple term a hundred thousand levels deep, its innermost object relative, is
    /// checked and re-interned on a thread whose whole stack is 128 KiB.
    #[test]
    fn a_hundred_thousand_level_term_is_checked_and_interned_on_a_128_kib_thread() {
        const LEVELS: usize = 100_000;
        purrdf_stack::on_stack(128 * 1024, || {
            let value = crate::term_fixture::triple_chain(LEVELS);
            assert!(check_value_absolute(&value).is_ok());
            let mut builder = RdfDatasetBuilder::new();
            assert_eq!(builder.intern_value(&value).index(), LEVELS + 2);
            let mut relative = TermValue::iri("o");
            for _ in 0..LEVELS {
                relative = TermValue::Triple {
                    s: TermBox::new(TermValue::iri("http://example.org/s")),
                    p: TermBox::new(TermValue::iri("http://example.org/p")),
                    o: TermBox::new(relative),
                };
            }
            assert!(check_value_absolute(&relative).is_err());
        })
        .expect("the thread starts");
    }
}

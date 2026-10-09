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
/// [`Self::RememberEmpty`] is the default; select [`Self::Implicit`] explicitly
/// to use row-driven graph lifetime.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum GraphExistenceMode {
    /// Preserve the current row-based graph lifetime: removing a graph's last row
    /// withdraws its declaration. An explicitly declared empty graph survives
    /// until a mutation or explicit withdrawal empties it.
    Implicit,
    /// Remember named-graph slots independently of rows, until explicit withdrawal.
    #[default]
    RememberEmpty,
}

/// Ordinary insertion recognizes this declaration predicate before publishing
/// a typed physical row; snapshot and freeze replay those normalized roles.
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
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub(crate) struct QuadKey {
    pub s: MutTermId,
    pub p: MutTermId,
    pub o: MutTermId,
    pub g: Option<MutTermId>,
}

/// The physical RDF table containing a record. Equal quad values in different
/// tables are distinct records.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[non_exhaustive]
pub enum RecordKind {
    /// An asserted quad, including a declaration-shaped quad explicitly kept ordinary.
    Ordinary,
    /// A resource binding to a triple term through `rdf:reifies`.
    Reifier,
    /// A statement annotation, including an annotation with no declaration.
    Annotation,
}

impl RecordKind {
    const ALL: [Self; 3] = [Self::Ordinary, Self::Reifier, Self::Annotation];
}

/// Owned terms and the exact physical table they belong to.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct RecordValues {
    /// Physical table, never an inference hint.
    pub kind: RecordKind,
    /// Dataset-independent term values.
    pub quad: QuadValues,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub(crate) struct RecordKey {
    pub kind: RecordKind,
    pub quad: QuadKey,
}

/// A reversible ordinary-API classification. Collapsed source records retain
/// their origins until explicitly removed, so removing the declaration can
/// restore their original, distinct physical roles.
#[derive(Clone, Copy, Debug)]
struct Reclassification {
    target: RecordKey,
    ordinal: Option<u64>,
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
    added: FastSet<RecordKey>,
    /// The insertion ordinal of each key currently in `added`, so `freeze()` and
    /// `effective_keys()` can replay delta-added quads in the order the caller
    /// actually added them — never in `added`'s hash-iteration order (see
    /// [`Self::added_in_order`]). A key present in `added` is always present here
    /// with the SAME ordinal it was (re)inserted at; removed keys are dropped from
    /// both maps together.
    added_ord: FastMap<RecordKey, u64>,
    /// The next ordinal `insert_key` will hand out to a brand-new `added` entry.
    /// Monotonically increasing for the lifetime of this `MutableDataset` — never
    /// reused, even across a remove/reinsert of the same key.
    next_added_ord: u64,
    /// Base quads suppressed (logically removed). A base quad is effective iff it is
    /// NOT in this set. Fixed-key hashed; only ever probed by membership, never
    /// iterated for order.
    suppressed: FastSet<RecordKey>,
    /// Maintained physical reference counts; public churn metrics count values.
    caller_added: FastMap<RecordKey, u64>,
    caller_suppressed: FastSet<RecordKey>,
    added_values: FastMap<QuadKey, usize>,
    suppressed_values: FastMap<QuadKey, usize>,
    /// Delta records admitted through the classifying, value-only API.
    automatic: FastMap<(MutTermId, Option<MutTermId>), FastSet<RecordKey>>,
    reclassified: FastMap<(MutTermId, Option<MutTermId>), FastMap<RecordKey, Reclassification>>,
    added_reifiers: FastMap<(MutTermId, Option<MutTermId>), usize>,
    classification_created: FastSet<RecordKey>,
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
            graph_existence: GraphExistenceMode::RememberEmpty,
            delta: DeltaBuilder::default(),
            added: FastSet::default(),
            added_ord: FastMap::default(),
            next_added_ord: 0,
            suppressed: FastSet::default(),
            caller_added: FastMap::default(),
            caller_suppressed: FastSet::default(),
            added_values: FastMap::default(),
            suppressed_values: FastMap::default(),
            automatic: FastMap::default(),
            reclassified: FastMap::default(),
            added_reifiers: FastMap::default(),
            classification_created: FastSet::default(),
            declared_graphs: Vec::new(),
            graph_rows: FastMap::default(),
            withdrawn_graphs: Arc::default(),
            work: super::view_accounting::WorkCounter::default(),
        }
    }

    /// Branch from `base` with an explicitly selected graph-existence policy.
    /// No base row is copied; the policy applies only to this mutable branch.
    /// [`Self::new`] remembers empty named graphs; select
    /// [`GraphExistenceMode::Implicit`] explicitly for row-driven lifetime.
    #[must_use]
    pub fn new_with_graph_existence(base: Arc<RdfDataset>, mode: GraphExistenceMode) -> Self {
        Self {
            graph_existence: mode,
            ..Self::new(base)
        }
    }

    /// The graph-existence policy selected when this mutable dataset was created.
    #[must_use]
    pub const fn graph_existence(&self) -> GraphExistenceMode {
        self.graph_existence
    }

    /// Create an empty named graph under this dataset's selected policy.
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
        self.holds_rows(graph)
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

    /// Insert into exactly the requested physical table, without inferred classification.
    ///
    /// # Errors
    /// Refuses invalid RDF terms, positions and noncanonical reifier records before
    /// publishing membership.
    pub fn insert_record(&mut self, record: &RecordValues) -> Result<bool, crate::RdfDiagnostic> {
        super::validate::validate_record(record)?;
        let quad = self.key_of(&record.quad).map_err(|error| {
            crate::RdfDiagnostic::error(error.diagnostic_code(), error.to_string())
        })?;
        let before = self.effective_count();
        let key = RecordKey {
            kind: record.kind,
            quad,
        };
        let changed = self.insert_record_rows(key, None);
        if changed {
            self.caller_insert(key);
        }
        self.apply_graph_change(quad.g, before);
        Ok(changed)
    }

    /// Remove only the requested physical role. Missing values mint no terms.
    pub fn remove_record(&mut self, record: &RecordValues) -> bool {
        let Some(quad) = self.key_of_existing(&record.quad) else {
            return false;
        };
        let before = self.effective_count();
        let key = RecordKey {
            kind: record.kind,
            quad,
        };
        let changed = self.remove_record_rows(key);
        if changed {
            self.caller_remove(key);
            self.forget_classification(key);
        }
        self.apply_graph_change(quad.g, before);
        changed
    }

    /// Take a stable, owned snapshot retaining each record's physical role.
    ///
    /// # Errors
    /// Refuses invalid unpublished delta records through snapshot admission.
    pub fn records_for_pattern(
        &self,
        s: Option<&TermValue>,
        p: Option<&TermValue>,
        o: Option<&TermValue>,
        g: GraphMatchValue<'_>,
    ) -> Result<Vec<RecordValues>, crate::RdfDiagnostic> {
        use crate::DatasetView as _;
        let view = self.snapshot_view()?;
        let lookup = |value: Option<&TermValue>| -> Option<Option<DeltaViewId>> {
            value.map_or(Some(None), |value| match view.term_id_by_value(value) {
                Ok(id) => id.map(Some),
                Err(error) => match error {},
            })
        };
        let (Some(s), Some(p), Some(o)) = (lookup(s), lookup(p), lookup(o)) else {
            return Ok(Vec::new());
        };
        let g = match g {
            GraphMatchValue::Any => GraphMatch::Any,
            GraphMatchValue::Default => GraphMatch::Default,
            GraphMatchValue::Named(value) => {
                let Some(Some(id)) = lookup(Some(value)) else {
                    return Ok(Vec::new());
                };
                GraphMatch::Named(id)
            }
        };
        let matches = |q: &crate::QuadIds<DeltaViewId>| {
            s.is_none_or(|s| q.s == s) && p.is_none_or(|p| q.p == p) && o.is_none_or(|o| q.o == o)
        };
        let rows = view
            .quads_for_pattern(s, p, o, g)
            .map(|q| (RecordKind::Ordinary, q))
            .chain(
                view.reifier_quads_in_graph(g)
                    .filter(matches)
                    .map(|q| (RecordKind::Reifier, q)),
            )
            .chain(
                view.annotation_quads_in_graph(g)
                    .filter(matches)
                    .map(|q| (RecordKind::Annotation, q)),
            );
        super::import::record_values_from(&view, rows)
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

    fn base_contains_record(&self, key: RecordKey) -> bool {
        let q = key.quad;
        let (MutTermId::Base(s), MutTermId::Base(p), MutTermId::Base(o)) = (q.s, q.p, q.o) else {
            return false;
        };
        let g = match q.g {
            None => GraphMatch::Default,
            Some(MutTermId::Base(g)) => GraphMatch::Named(g),
            Some(MutTermId::Delta(_)) => return false,
        };
        match key.kind {
            RecordKind::Ordinary => {
                RdfDataset::quads_for_pattern_indexed(&self.base, Some(s), Some(p), Some(o), g)
                    .next()
                    .is_some()
            }
            RecordKind::Reifier => self
                .base
                .reifier_quads_of(s)
                .any(|q| q.p == p && q.o == o && g.matches(q.g)),
            RecordKind::Annotation => self
                .base
                .annotations_of_with_graph(s)
                .any(|(pred, obj, graph)| pred == p && obj == o && g.matches(graph)),
        }
    }

    fn contains_record_key(&self, key: RecordKey) -> bool {
        self.added.contains(&key)
            || (!self.suppressed.contains(&key) && self.base_contains_record(key))
    }

    fn contains_key(&self, quad: &QuadKey) -> bool {
        RecordKind::ALL
            .into_iter()
            .any(|kind| self.contains_record_key(RecordKey { kind, quad: *quad }))
    }

    fn increment_value(index: &mut FastMap<QuadKey, usize>, key: QuadKey) {
        *index.entry(key).or_default() += 1;
    }

    fn decrement_value(index: &mut FastMap<QuadKey, usize>, key: QuadKey) {
        let count = index
            .get_mut(&key)
            .expect("physical membership has a value reference");
        *count -= 1;
        if *count == 0 {
            index.remove(&key);
        }
    }

    // Physical transitions deliberately do not change graph lifetime. Their
    // caller applies the net change after a complete conversion/value operation.
    fn insert_record_rows(&mut self, key: RecordKey, ordinal: Option<u64>) -> bool {
        if self.suppressed.remove(&key) {
            return true;
        }
        if self.contains_record_key(key) {
            return false;
        }
        self.added.insert(key);
        if key.kind == RecordKind::Reifier {
            *self
                .added_reifiers
                .entry((key.quad.s, key.quad.g))
                .or_default() += 1;
        }
        let ordinal = ordinal.unwrap_or_else(|| {
            let ordinal = self.next_added_ord;
            self.next_added_ord = ordinal.checked_add(1).expect("insertion ordinal exhausted");
            ordinal
        });
        self.added_ord.insert(key, ordinal);
        true
    }

    fn remove_record_rows(&mut self, key: RecordKey) -> bool {
        if self.added.remove(&key) {
            if key.kind == RecordKind::Reifier {
                let count = self
                    .added_reifiers
                    .get_mut(&(key.quad.s, key.quad.g))
                    .expect("added reifier has a subject index");
                *count -= 1;
                if *count == 0 {
                    self.added_reifiers.remove(&(key.quad.s, key.quad.g));
                }
            }
            self.added_ord.remove(&key);
            return true;
        }
        self.base_contains_record(key) && self.suppressed.insert(key)
    }

    // Public mutation metrics describe caller intent, not the physical masks and
    // derived targets needed to normalize classification. Keep their value reference
    // counts at the mutation boundary, so reads remain allocation-free.
    fn caller_insert(&mut self, key: RecordKey) {
        if self.base_contains_record(key) {
            if self.caller_suppressed.remove(&key) {
                Self::decrement_value(&mut self.suppressed_values, key.quad);
            }
        } else if !self.caller_added.contains_key(&key) {
            let ordinal = self.added_ord[&key];
            self.caller_added.insert(key, ordinal);
            Self::increment_value(&mut self.added_values, key.quad);
        }
    }

    fn caller_remove_origin(&mut self, key: RecordKey) {
        if self.caller_added.remove(&key).is_some() {
            Self::decrement_value(&mut self.added_values, key.quad);
        }
        if self.base_contains_record(key) && self.caller_suppressed.insert(key) {
            Self::increment_value(&mut self.suppressed_values, key.quad);
        }
    }

    fn caller_remove(&mut self, key: RecordKey) {
        let origins: Vec<_> = self
            .reclassified
            .get(&(key.quad.s, key.quad.g))
            .into_iter()
            .flat_map(|conversions| conversions.iter())
            .filter_map(|(&source, conversion)| (conversion.target == key).then_some(source))
            .collect();
        self.caller_remove_origin(key);
        for source in origins {
            self.caller_remove_origin(source);
        }
    }

    fn forget_classification(&mut self, key: RecordKey) {
        let scope = (key.quad.s, key.quad.g);
        if let Some(origins) = self.automatic.get_mut(&scope) {
            origins.remove(&key);
        }
        self.classification_created.remove(&key);
        if let Some(conversions) = self.reclassified.get_mut(&scope) {
            conversions.retain(|source, conversion| {
                if conversion.target == key {
                    if let Some(origins) = self.automatic.get_mut(&scope) {
                        origins.remove(source);
                    }
                    false
                } else {
                    true
                }
            });
        }
    }

    fn graph_rows_of(&mut self, graph: MutTermId) -> &mut usize {
        let base = &self.base;
        self.graph_rows.entry(graph).or_insert_with(|| match graph {
            MutTermId::Base(id) => base.named_graph_row_count(id),
            MutTermId::Delta(_) => 0,
        })
    }

    fn apply_graph_change(&mut self, graph: Option<MutTermId>, before: usize) {
        let after = self.effective_count();
        let Some(graph) = graph else {
            return;
        };
        let live = self.graph_rows_of(graph);
        let old = *live;
        *live = if after >= before {
            old + (after - before)
        } else {
            old - (before - after)
        };
        let now = *live;
        if now == 0 && old > 0 && self.graph_existence == GraphExistenceMode::Implicit {
            if let MutTermId::Base(id) = graph {
                Arc::make_mut(&mut self.withdrawn_graphs).insert(id);
            }
            self.withdraw_emptied_declaration(graph);
        } else if now > 0 && old == 0 {
            if let MutTermId::Base(id) = graph {
                Arc::make_mut(&mut self.withdrawn_graphs).remove(&id);
            }
            if self.graph_existence == GraphExistenceMode::RememberEmpty
                && !matches!(graph, MutTermId::Base(id) if self.base.has_named_graph(id))
            {
                self.declare_graph(self.mut_value(graph));
            }
        }
    }

    fn withdraw_emptied_declaration(&mut self, graph: MutTermId) {
        if self.declared_graphs.is_empty() {
            return;
        }
        let value = self.mut_value(graph);
        if let Some(index) = self.declared_graphs.iter().position(|g| *g == value) {
            self.declared_graphs.remove(index);
        }
    }

    fn declaration_key(&self, q: QuadKey) -> bool {
        let predicate = match q.p {
            MutTermId::Base(id) => {
                matches!(self.base.resolve(id), TermRef::Iri(iri) if iri == RDF_REIFIES)
            }
            MutTermId::Delta(id) => {
                matches!(self.delta.value(id), TermValue::Iri(iri) if iri == RDF_REIFIES)
            }
        };
        predicate
            && match q.o {
                MutTermId::Base(id) => matches!(self.base.resolve(id), TermRef::Triple { .. }),
                MutTermId::Delta(id) => matches!(self.delta.value(id), TermValue::Triple { .. }),
            }
    }

    fn has_effective_reifier(&self, s: MutTermId, g: Option<MutTermId>) -> bool {
        let base = match s {
            MutTermId::Base(id) => self.base.reifier_quads_of(id).any(|q| {
                let quad = Self::base_key(q);
                quad.g == g
                    && self.contains_record_key(RecordKey {
                        kind: RecordKind::Reifier,
                        quad,
                    })
            }),
            MutTermId::Delta(_) => false,
        };
        base || self
            .added_reifiers
            .get(&(s, g))
            .is_some_and(|count| *count > 0)
    }

    fn base_key(q: super::QuadIds) -> QuadKey {
        QuadKey {
            s: MutTermId::Base(q.s),
            p: MutTermId::Base(q.p),
            o: MutTermId::Base(q.o),
            g: q.g.map(MutTermId::Base),
        }
    }

    fn normalize_subject(&mut self, subject: MutTermId, graph: Option<MutTermId>) {
        let matches = |q: QuadKey| q.s == subject && q.g == graph;
        let old = self
            .reclassified
            .remove(&(subject, graph))
            .unwrap_or_default();
        // Remove only generated target rows, never an independently existing row
        // that a conversion happened to collide with.
        for conversion in old.values() {
            if self.classification_created.remove(&conversion.target) {
                self.remove_record_rows(conversion.target);
            }
        }
        for (source, conversion) in old {
            self.insert_record_rows(source, conversion.ordinal);
        }
        let effective_reifier = self.has_effective_reifier(subject, graph);
        let mut candidates = Vec::new();
        if let MutTermId::Base(id) = subject {
            let g = graph.map_or(GraphMatch::Default, |g| match g {
                MutTermId::Base(g) => GraphMatch::Named(g),
                MutTermId::Delta(_) => GraphMatch::Any,
            });
            candidates.extend(
                self.base
                    .quads_for_pattern_indexed(Some(id), None, None, g)
                    .map(|q| RecordKey {
                        kind: RecordKind::Ordinary,
                        quad: Self::base_key(q),
                    }),
            );
            candidates.extend(
                self.base
                    .annotations_of_with_graph(id)
                    .map(|(p, o, g)| RecordKey {
                        kind: RecordKind::Annotation,
                        quad: Self::base_key(super::QuadIds { s: id, p, o, g }),
                    })
                    .filter(|key| matches(key.quad)),
            );
        }
        if let Some(origins) = self.automatic.get(&(subject, graph)) {
            candidates.extend(origins.iter().copied());
        }
        // Base origins have a canonical typed-key order. Delta origins follow
        // their unique insertion ordinal; a total tie-break never exposes hash
        // iteration order when a base origin meets the first delta ordinal.
        candidates.sort_unstable_by_key(|key| {
            (
                self.added_ord
                    .get(key)
                    .copied()
                    .map_or((0, 0), |ordinal| (1, ordinal)),
                *key,
            )
        });
        candidates.dedup();
        for source in candidates {
            if !matches(source.quad) || !self.contains_record_key(source) {
                continue;
            }
            self.classify_record(source, effective_reifier);
        }
    }

    fn classify_record(&mut self, source: RecordKey, effective_reifier: bool) {
        let physical = match source.kind {
            RecordKind::Ordinary => StatementKind::Ordinary,
            RecordKind::Annotation => StatementKind::Annotation,
            RecordKind::Reifier => return,
        };
        let original_reifier = match source.quad.s {
            MutTermId::Base(id) => self
                .base
                .reifier_quads_of(id)
                .any(|q| Self::base_key(q).g == source.quad.g),
            MutTermId::Delta(_) => false,
        };
        let target_kind =
            match classify_statement(physical, true, original_reifier, effective_reifier)
                .expect("visible record has a classification")
            {
                StatementKind::Ordinary => RecordKind::Ordinary,
                StatementKind::Annotation => RecordKind::Annotation,
            };
        if target_kind == source.kind {
            return;
        }
        let ordinal = self.added_ord.get(&source).copied();
        self.remove_record_rows(source);
        let target = RecordKey {
            kind: target_kind,
            quad: source.quad,
        };
        if self.insert_record_rows(target, ordinal) {
            self.classification_created.insert(target);
        }
        self.reclassified
            .entry((source.quad.s, source.quad.g))
            .or_default()
            .insert(source, Reclassification { target, ordinal });
    }

    fn insert_key(&mut self, quad: QuadKey) -> bool {
        let before = self.effective_count();
        let mut changed = false;
        let mut restored = Vec::new();
        for kind in RecordKind::ALL {
            let key = RecordKey { kind, quad };
            if self.suppressed.contains(&key)
                && !self
                    .reclassified
                    .get(&(quad.s, quad.g))
                    .is_some_and(|conversions| conversions.contains_key(&key))
                && self.insert_record_rows(key, None)
            {
                self.caller_insert(key);
                restored.push(key);
                changed = true;
            }
        }
        if !changed && !self.contains_key(&quad) {
            let kind = if self.declaration_key(quad) {
                RecordKind::Reifier
            } else {
                RecordKind::Ordinary
            };
            let key = RecordKey { kind, quad };
            changed = self.insert_record_rows(key, None);
            if changed {
                self.caller_insert(key);
            }
            if kind == RecordKind::Ordinary {
                self.automatic
                    .entry((quad.s, quad.g))
                    .or_default()
                    .insert(key);
                let reifier = self.has_effective_reifier(quad.s, quad.g);
                self.classify_record(key, reifier);
            }
        }
        if changed && self.declaration_key(quad) {
            self.normalize_subject(quad.s, quad.g);
        } else {
            let reifier = self.has_effective_reifier(quad.s, quad.g);
            for key in restored {
                self.classify_record(key, reifier);
            }
        }
        self.apply_graph_change(quad.g, before);
        changed
    }

    fn remove_key(&mut self, quad: QuadKey) -> bool {
        let before = self.effective_count();
        let mut changed = false;
        for kind in RecordKind::ALL {
            let key = RecordKey { kind, quad };
            if self.contains_record_key(key) {
                changed |= self.remove_record_rows(key);
                self.caller_remove(key);
                self.forget_classification(key);
            }
        }
        if changed && self.declaration_key(quad) {
            self.normalize_subject(quad.s, quad.g);
        }
        self.apply_graph_change(quad.g, before);
        changed
    }

    fn added_in_order(&self) -> Vec<RecordKey> {
        let mut ordered: Vec<_> = self
            .added_ord
            .iter()
            .map(|(&key, &ord)| (ord, key))
            .collect();
        debug_assert_eq!(ordered.len(), self.added.len());
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
    /// In remembered mode last-row removal retains the slot, and this explicit call
    /// withdraws it only while the graph holds no row — for example after a DROP
    /// removed its rows. Withdrawing a populated graph leaves its slot in place, so
    /// removing its last row afterwards keeps it, whether the declaration came from
    /// the base or from this mutable dataset.
    pub fn withdraw_graph_declaration(&mut self, graph: &TermValue) {
        if let Some(id) = self.base.term_id_by_value(graph) {
            self.withdraw_base_graph(id);
        }
        // A declaration made through `declare_named_graph` is withdrawn the same way:
        // a graph that still holds rows stays enumerated through them.
        if self.graph_existence == GraphExistenceMode::RememberEmpty {
            if let Some(index) = self.declared_graphs.iter().position(|g| g == graph)
                && !self.holds_rows(graph)
            {
                self.declared_graphs.remove(index);
            }
        } else {
            self.declared_graphs.retain(|g| g != graph);
        }
    }

    /// Whether the named graph `graph` holds a live row in this mutable dataset.
    /// Answered from the exact per-graph row counts, which cover every graph a
    /// mutation touched; an untouched base graph is answered by its base declaration.
    fn holds_rows(&self, graph: &TermValue) -> bool {
        self.base
            .term_id_by_value(graph)
            .map(MutTermId::Base)
            .or_else(|| self.delta.find(graph).map(MutTermId::Delta))
            .is_some_and(|id| self.graph_rows.get(&id).is_some_and(|&rows| rows > 0))
    }

    /// [`Self::withdraw_graph_declaration`] for every named graph of the base —
    /// `DROP NAMED` / `DROP ALL` of a dataset with declared empty graphs.
    pub fn withdraw_named_graph_declarations(&mut self) {
        self.try_withdraw_named_graph_declarations(|| Ok::<_, Infallible>(()))
            .unwrap_or_else(|never| match never {});
    }

    /// Withdraw every named-graph declaration, checking before each base entry
    /// and each declaration added to this mutable dataset.
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
        if self.graph_existence == GraphExistenceMode::RememberEmpty {
            // A populated remembered slot keeps its declaration, exactly as a populated
            // base graph does above.
            let mut index = self.declared_graphs.len();
            while index > 0 {
                checkpoint()?;
                index -= 1;
                if !self.holds_rows(&self.declared_graphs[index]) {
                    self.declared_graphs.remove(index);
                }
            }
        } else {
            while !self.declared_graphs.is_empty() {
                checkpoint()?;
                let _ = self.declared_graphs.pop();
            }
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
        self.added_values.len()
    }

    /// The number of base quads currently suppressed.
    #[must_use]
    pub fn suppressed_len(&self) -> usize {
        self.suppressed_values.len()
    }

    /// Iterate the effective quads as value-quads — the independent test/property-test
    /// oracle for the effective set. `freeze` builds the effective set directly (so it
    /// can carry per-base-quad source locations), so this is a test-only helper; the
    /// public surface is [`DatasetMut`].
    #[cfg(test)]
    fn effective_value_quads(&self) -> Vec<QuadValues> {
        self.effective_record_keys()
            .map(|key| self.quad_values_of(&key.quad))
            .collect()
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
        let base_id = |id| match id {
            MutTermId::Base(id) => id,
            MutTermId::Delta(_) => unreachable!("base-origin conversion has only base terms"),
        };
        let converted: FastSet<_> = self
            .reclassified
            .values()
            .flat_map(|origins| origins.iter())
            .filter(|(source, conversion)| {
                matches!(source.quad.s, MutTermId::Base(_))
                    && conversion.ordinal.is_none()
                    && self.added.contains(&conversion.target)
                    && self.classification_created.contains(&conversion.target)
            })
            .map(|(source, conversion)| {
                (
                    conversion.target.kind,
                    super::QuadIds {
                        s: base_id(source.quad.s),
                        p: base_id(source.quad.p),
                        o: base_id(source.quad.o),
                        g: source.quad.g.map(base_id),
                    },
                )
            })
            .collect();
        let mut builder = self.base.rebuild_builder();
        let mut admission =
            DeltaAdmission::new(&self.base, self.suppressed.len(), converted.len(), limits);
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
            .map(|key| {
                (
                    key.kind,
                    super::QuadIds {
                        s: base_id(key.quad.s),
                        p: base_id(key.quad.p),
                        o: base_id(key.quad.o),
                        g: key.quad.g.map(base_id),
                    },
                )
            })
            .collect();
        let view = DeltaDatasetView::new(
            Arc::clone(&self.base),
            delta,
            suppressed,
            converted,
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
        // Preserve authored term admission order before replaying derived base
        // conversions. Native indexes order by term identity, so minting a derived
        // predicate first would reorder otherwise unchanged caller-added probes.
        let mut caller_terms: Vec<_> = self
            .caller_added
            .iter()
            .filter(|(key, _)| {
                self.added.contains(key)
                    || self
                        .reclassified
                        .get(&(key.quad.s, key.quad.g))
                        .and_then(|conversions| conversions.get(key))
                        .is_some_and(|conversion| self.added.contains(&conversion.target))
            })
            .collect();
        caller_terms.sort_unstable_by_key(|(_, ordinal)| **ordinal);
        for (key, _) in caller_terms {
            let quad = self.quad_values_of(&key.quad);
            for term in [&quad.s, &quad.p, &quad.o]
                .into_iter()
                .chain(quad.g.as_ref())
            {
                builder.intern_value(term);
                admission.check(builder)?;
            }
        }
        for key in self.added_in_order() {
            let record = RecordValues {
                kind: key.kind,
                quad: self.quad_values_of(&key.quad),
            };
            builder.push_record_unchecked(&record);
            let graph = record.quad.g.as_ref().map(|g| builder.intern_value(g));
            admission.row(builder, graph)?;
        }
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
    converted: usize,
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
        converted: usize,
        limits: super::view_accounting::ViewLimits,
    ) -> Self {
        Self {
            base,
            suppressed,
            converted,
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
                    .saturating_add(self.converted)
                    .saturating_add(rows)
                    .saturating_mul(4 * size_of::<(RecordKind, super::QuadIds)>()),
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
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
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

/// Shared graph-scoped RDF 1.2 statement classification. Source-specific indexed
/// probes supply visibility and reifier presence; an explicitly typed orphan
/// annotation stays typed unless its originally applicable declaration disappears.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StatementKind {
    Ordinary,
    Annotation,
}
pub(crate) const fn classify_statement(
    physical: StatementKind,
    visible: bool,
    original_reifier: bool,
    effective_reifier: bool,
) -> Option<StatementKind> {
    if !visible {
        return None;
    }
    match physical {
        StatementKind::Ordinary if effective_reifier => Some(StatementKind::Annotation),
        StatementKind::Annotation if original_reifier && !effective_reifier => {
            Some(StatementKind::Ordinary)
        }
        other => Some(other),
    }
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
    fn effective_record_keys(&self) -> impl Iterator<Item = RecordKey> + '_ {
        let base = super::import::record_ids(self.base.as_ref())
            .map(|(kind, q)| RecordKey {
                kind,
                quad: Self::base_key(q),
            })
            .filter(|key| !self.suppressed.contains(key));
        base.chain(self.added_in_order())
    }

    fn effective_keys(&self) -> impl Iterator<Item = QuadKey> + '_ {
        self.effective_record_keys().map(|key| key.quad)
    }

    /// The effective quads as `Copy` base-or-frozen `QuadIds` is NOT exposed: ids
    /// straddling base/delta have no single dataset to be local to (C0.8). Consumers
    /// read values via [`DatasetMut::quads_for_pattern`] or `freeze()` to a frozen
    /// dataset and read `QuadIds` there.
    #[doc(hidden)]
    pub fn effective_count(&self) -> usize {
        // O(1) from the mutation invariants (no base scan):
        //   • suppression keys name exact physical base roles;
        //   • every added record is absent from the visible base in that role;
        //   • `added` and `suppressed` are disjoint.
        // Hence effective = base ∪ added − suppressed has exactly this cardinality.
        self.base.rdf_row_count() + self.added.len() - self.suppressed.len()
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
    #[test]
    fn selected_records_match_physical_roles_before_owning_term_payloads() {
        use super::*;
        let mut builder = RdfDatasetBuilder::new();
        let subject = builder.intern_iri("https://example.org/s");
        let predicate = builder.intern_iri("https://example.org/p");
        let object = builder.intern_iri("https://example.org/o");
        let graph = builder.intern_iri("https://example.org/g");
        let other = builder.intern_iri("https://example.org/other");
        builder.push_quad(subject, predicate, object, Some(graph));
        builder.push_annotation_in_graph(subject, predicate, object, Some(graph));
        builder.push_quad(subject, predicate, object, Some(other));
        builder.push_quad(subject, predicate, object, None);
        let mutable = MutableDataset::new(builder.freeze().unwrap());
        let s = TermValue::iri("https://example.org/s");
        let p = TermValue::iri("https://example.org/p");
        let o = TermValue::iri("https://example.org/o");
        let g = TermValue::iri("https://example.org/g");
        let selected = mutable
            .records_for_pattern(Some(&s), Some(&p), Some(&o), GraphMatchValue::Named(&g))
            .unwrap();
        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].kind, RecordKind::Ordinary);
        assert_eq!(selected[1].kind, RecordKind::Annotation);
        assert_eq!(selected[0].quad, selected[1].quad);
        assert_eq!(
            mutable
                .records_for_pattern(None, None, None, GraphMatchValue::Default)
                .unwrap()
                .len(),
            1
        );
        let absent = TermValue::iri("https://example.org/absent");
        for (s, p, o, g) in [
            (Some(&absent), None, None, GraphMatchValue::Any),
            (None, Some(&absent), None, GraphMatchValue::Any),
            (None, None, Some(&absent), GraphMatchValue::Any),
            (None, None, None, GraphMatchValue::Named(&absent)),
        ] {
            assert_eq!(
                mutable.records_for_pattern(s, p, o, g).unwrap(),
                Vec::<RecordValues>::new()
            );
        }
    }
    #[test]
    fn typed_composites_refuse_malformed_values_and_preserve_bound_identity() {
        let mut mutable = MutableDataset::new(RdfDatasetBuilder::new().freeze().unwrap());
        for datatype in [purrdf_cdt::CDT_LIST, purrdf_cdt::CDT_MAP] {
            let record = RecordValues {
                kind: RecordKind::Ordinary,
                quad: QuadValues::triple(
                    iri_val("s"),
                    iri_val("p"),
                    TermValue::typed_literal("broken [_:missing", datatype),
                ),
            };
            assert_eq!(
                mutable.insert_record(&record).unwrap_err().code,
                "cdt-literal-malformed"
            );
            assert_eq!(mutable.delta.values.len(), 0);
            assert_eq!(mutable.effective_count(), 0);
            let mut builder = RdfDatasetBuilder::new();
            assert!(builder.push_record(&record).is_err());
            assert_eq!(builder.term_count(), 0);
        }
        let scope = crate::BlankScope(73);
        let label = crate::blank_label::encode_blank_label(
            "embedded",
            scope,
            crate::blank_label::LabelAlphabet::BlankNodeLabel,
        );
        let lexical = format!("[ _:{label}, _:{label} ]");
        let record = RecordValues {
            kind: RecordKind::Annotation,
            quad: QuadValues::triple(
                iri_val("s"),
                iri_val("p"),
                TermValue::typed_literal(&lexical, purrdf_cdt::CDT_LIST),
            ),
        };
        assert!(mutable.insert_record(&record).unwrap());
        assert_eq!(typed_image(&mutable), std::iter::once(record).collect());
        let frozen = mutable.freeze().unwrap();
        assert!(frozen.term_id_by_blank("embedded", scope).is_some());
    }
    #[test]
    fn normalization_has_stable_order_through_conversion_and_undo() {
        let mut builder = RdfDatasetBuilder::new();
        let s = builder.intern_iri("http://example.org/r");
        let p = builder.intern_iri("http://example.org/p");
        let b = builder.intern_iri("http://example.org/b");
        let a = builder.intern_iri("http://example.org/a");
        builder.push_quad(s, p, b, None);
        builder.push_quad(s, p, a, None);
        let base = builder.freeze().unwrap();
        let declaration = QuadValues::triple(
            iri_val("r"),
            TermValue::iri(RDF_REIFIES),
            TermValue::Triple {
                s: iri_val("s").into(),
                p: iri_val("p").into(),
                o: iri_val("o").into(),
            },
        );
        for noise in 0..32 {
            let mut mutable = MutableDataset::new(Arc::clone(&base));
            for index in 0..noise {
                let row = q(&format!("noise{index}"), "p", "o");
                assert!(mutable.insert(row.clone()).unwrap());
                assert!(mutable.remove(&row));
            }
            assert!(mutable.insert(q("r", "p", "c")).unwrap());
            assert!(mutable.insert(q("r", "p", "d")).unwrap());
            for _ in 0..3 {
                assert!(mutable.insert(declaration.clone()).unwrap());
                assert_eq!(
                    mutable.quads_for_pattern(None, None, None, GraphMatchValue::Any),
                    [
                        q("r", "p", "c"),
                        q("r", "p", "d"),
                        declaration.clone(),
                        q("r", "p", "b"),
                        q("r", "p", "a")
                    ]
                );
                assert!(mutable.remove(&declaration));
                assert_eq!(
                    mutable.quads_for_pattern(None, None, None, GraphMatchValue::Any),
                    [
                        q("r", "p", "b"),
                        q("r", "p", "a"),
                        q("r", "p", "c"),
                        q("r", "p", "d")
                    ]
                );
            }
        }
    }

    #[test]
    fn typed_base_three_roles_restore_each_exactly_and_remove_base_plus_delta() {
        let quad = QuadValues::triple(
            iri_val("r"),
            TermValue::iri(RDF_REIFIES),
            TermValue::Triple {
                s: iri_val("s").into(),
                p: iri_val("p").into(),
                o: iri_val("o").into(),
            },
        );
        let mut builder = RdfDatasetBuilder::new();
        for kind in RecordKind::ALL {
            builder
                .push_record(&RecordValues {
                    kind,
                    quad: quad.clone(),
                })
                .unwrap();
        }
        let base = builder.freeze().unwrap();
        for kind in RecordKind::ALL {
            let mut mutable = MutableDataset::new(Arc::clone(&base));
            assert!(mutable.remove(&quad));
            assert_eq!(mutable.suppressed_len(), 1);
            let record = RecordValues {
                kind,
                quad: quad.clone(),
            };
            assert!(mutable.insert_record(&record).unwrap());
            assert_eq!(typed_image(&mutable), std::iter::once(record).collect());
            assert_eq!(mutable.suppressed_len(), 1);
        }
        let mut ordinary = RdfDatasetBuilder::new();
        ordinary
            .push_record(&RecordValues {
                kind: RecordKind::Ordinary,
                quad: quad.clone(),
            })
            .unwrap();
        let mut mutable = MutableDataset::new(ordinary.freeze().unwrap());
        assert!(
            mutable
                .insert_record(&RecordValues {
                    kind: RecordKind::Annotation,
                    quad: quad.clone()
                })
                .unwrap()
        );
        assert!(mutable.remove(&quad));
        assert_eq!(mutable.added_len(), 0);
        assert_eq!(mutable.suppressed_len(), 1);
        assert!(typed_image(&mutable).is_empty());
    }
    #[test]
    fn public_mutation_metrics_exclude_derived_classification() {
        let row = q("r", "p", "o");
        let declaration = QuadValues::triple(
            iri_val("r"),
            TermValue::iri(RDF_REIFIES),
            TermValue::Triple {
                s: iri_val("s").into(),
                p: iri_val("p").into(),
                o: iri_val("o").into(),
            },
        );
        for original in [RecordKind::Ordinary, RecordKind::Annotation] {
            let mut builder = RdfDatasetBuilder::new();
            builder
                .push_record(&RecordValues {
                    kind: original,
                    quad: row.clone(),
                })
                .unwrap();
            if original == RecordKind::Annotation {
                builder
                    .push_record(&RecordValues {
                        kind: RecordKind::Reifier,
                        quad: declaration.clone(),
                    })
                    .unwrap();
            }
            let mut mutable = MutableDataset::new(builder.freeze().unwrap());
            if original == RecordKind::Ordinary {
                assert!(mutable.insert(declaration.clone()).unwrap());
                assert_eq!((mutable.added_len(), mutable.suppressed_len()), (1, 0));
                assert_eq!(
                    typed_image(&mutable),
                    [
                        RecordValues {
                            kind: RecordKind::Annotation,
                            quad: row.clone()
                        },
                        RecordValues {
                            kind: RecordKind::Reifier,
                            quad: declaration.clone()
                        },
                    ]
                    .into_iter()
                    .collect()
                );
                assert!(mutable.remove(&declaration));
            } else {
                assert!(mutable.remove(&declaration));
                assert_eq!((mutable.added_len(), mutable.suppressed_len()), (0, 1));
                assert_eq!(
                    typed_image(&mutable),
                    std::iter::once(RecordValues {
                        kind: RecordKind::Ordinary,
                        quad: row.clone()
                    })
                    .collect()
                );
                assert!(mutable.insert(declaration.clone()).unwrap());
            }
            assert_eq!((mutable.added_len(), mutable.suppressed_len()), (0, 0));
            assert!(mutable.remove(&row));
            assert_eq!(mutable.added_len(), 0);
            assert_eq!(mutable.suppressed_len(), 1);
            assert!(mutable.insert(row.clone()).unwrap());
            assert_eq!((mutable.added_len(), mutable.suppressed_len()), (0, 0));
        }
        let mut mutable = MutableDataset::new(RdfDatasetBuilder::new().freeze().unwrap());
        assert!(mutable.insert(row.clone()).unwrap());
        assert!(mutable.insert(declaration.clone()).unwrap());
        assert_eq!((mutable.added_len(), mutable.suppressed_len()), (2, 0));
        assert!(mutable.remove(&row));
        assert_eq!((mutable.added_len(), mutable.suppressed_len()), (1, 0));
        assert!(mutable.remove(&declaration));
        assert_eq!((mutable.added_len(), mutable.suppressed_len()), (0, 0));
    }

    #[test]
    fn normalization_preserves_delta_target_ordinal_through_undo() {
        let original = q("r", "p", "o");
        let first = q("z", "p", "z");
        let mut mutable = MutableDataset::new(RdfDatasetBuilder::new().freeze().unwrap());
        assert!(mutable.insert(original.clone()).unwrap());
        for quad in [&first, &original] {
            assert!(
                mutable
                    .insert_record(&RecordValues {
                        kind: RecordKind::Annotation,
                        quad: quad.clone()
                    })
                    .unwrap()
            );
        }
        let original_order = mutable.added_in_order();
        let original_ordinals: Vec<_> = original_order
            .iter()
            .map(|key| mutable.added_ord[key])
            .collect();
        // Native annotation indexes sort by term identity. The earlier ordinary
        // row already admitted r, so compare the real public stream across the
        // conversion rather than assuming annotation insertion order.
        let annotations = |mutable: &MutableDataset| {
            let view = mutable.snapshot_view().unwrap();
            super::super::import::record_values(&view)
                .unwrap()
                .into_iter()
                .filter(|record| record.kind == RecordKind::Annotation)
                .map(|record| record.quad)
                .collect::<Vec<_>>()
        };
        let original_annotations = annotations(&mutable);
        assert_eq!(original_annotations, [original, first]);
        let declaration = QuadValues::triple(
            iri_val("r"),
            TermValue::iri(RDF_REIFIES),
            TermValue::Triple {
                s: iri_val("s").into(),
                p: iri_val("p").into(),
                o: iri_val("o").into(),
            },
        );
        assert!(mutable.insert(declaration.clone()).unwrap());
        assert_eq!(mutable.added_ord[&original_order[2]], original_ordinals[2]);
        assert_eq!(annotations(&mutable), original_annotations);
        assert!(mutable.remove(&declaration));
        assert_eq!(annotations(&mutable), original_annotations);
        assert_eq!(mutable.added_in_order(), original_order);
        assert_eq!(
            original_order
                .iter()
                .map(|key| mutable.added_ord[key])
                .collect::<Vec<_>>(),
            original_ordinals
        );
        let mut ordinals: Vec<_> = mutable.added_ord.values().copied().collect();
        ordinals.sort_unstable();
        ordinals.dedup();
        assert_eq!(ordinals.len(), mutable.added_ord.len());
    }

    #[test]
    fn normalization_does_not_reorder_an_independently_added_target() {
        let original = q("r", "p", "o");
        let first = q("z", "p", "z");
        let mut builder = RdfDatasetBuilder::new();
        builder
            .push_record(&RecordValues {
                kind: RecordKind::Ordinary,
                quad: original.clone(),
            })
            .unwrap();
        let mut mutable = MutableDataset::new(builder.freeze().unwrap());
        for quad in [&first, &original] {
            assert!(
                mutable
                    .insert_record(&RecordValues {
                        kind: RecordKind::Annotation,
                        quad: quad.clone()
                    })
                    .unwrap()
            );
        }
        let declaration = QuadValues::triple(
            iri_val("r"),
            TermValue::iri(RDF_REIFIES),
            TermValue::Triple {
                s: iri_val("s").into(),
                p: iri_val("p").into(),
                o: iri_val("o").into(),
            },
        );
        assert!(mutable.insert(declaration.clone()).unwrap());
        let view = mutable.snapshot_view().unwrap();
        let annotations: Vec<_> = super::super::import::record_values(&view)
            .unwrap()
            .into_iter()
            .filter(|record| record.kind == RecordKind::Annotation)
            .map(|record| record.quad)
            .collect();
        assert_eq!(annotations, [first.clone(), original.clone()]);
        assert_eq!(mutable.effective_count(), 3);
        assert!(mutable.remove(&declaration));
        assert_eq!(
            typed_image(&mutable),
            [
                RecordValues {
                    kind: RecordKind::Ordinary,
                    quad: original.clone()
                },
                RecordValues {
                    kind: RecordKind::Annotation,
                    quad: first
                },
                RecordValues {
                    kind: RecordKind::Annotation,
                    quad: original
                }
            ]
            .into_iter()
            .collect()
        );
    }

    #[test]
    fn ordinary_restoration_classifies_suppressed_base_roles() {
        for original in [RecordKind::Ordinary, RecordKind::Annotation] {
            let mut builder = RdfDatasetBuilder::new();
            let s = builder.intern_iri("http://example.org/r");
            let p = builder.intern_iri("http://example.org/p");
            let o = builder.intern_iri("http://example.org/o");
            let triple = builder.intern_triple(s, p, o);
            if original == RecordKind::Ordinary {
                builder.push_quad(s, p, o, None);
            } else {
                builder.push_annotation(s, p, o);
                builder.push_reifier(s, triple);
            }
            let mut mutable = MutableDataset::new(builder.freeze().unwrap());
            let record = RecordValues {
                kind: original,
                quad: q("r", "p", "o"),
            };
            assert!(mutable.remove_record(&record));
            let declaration = QuadValues::triple(
                iri_val("r"),
                TermValue::iri(RDF_REIFIES),
                TermValue::Triple {
                    s: iri_val("r").into(),
                    p: iri_val("p").into(),
                    o: iri_val("o").into(),
                },
            );
            if original == RecordKind::Ordinary {
                assert!(mutable.insert(declaration).unwrap());
            } else {
                assert!(mutable.remove(&declaration));
            }
            assert!(mutable.insert(record.quad.clone()).unwrap());
            let desired = if original == RecordKind::Ordinary {
                RecordKind::Annotation
            } else {
                RecordKind::Ordinary
            };
            assert!(typed_image(&mutable).contains(&RecordValues {
                kind: desired,
                quad: record.quad.clone()
            }));
            assert!(!typed_image(&mutable).contains(&record));
        }
    }
    #[test]
    fn classification_restores_only_targets_it_owned() {
        let mut builder = RdfDatasetBuilder::new();
        let s = builder.intern_iri("http://example.org/r");
        let p = builder.intern_iri("http://example.org/p");
        let o = builder.intern_iri("http://example.org/o");
        builder.push_quad(s, p, o, None);
        builder.push_annotation(s, p, o);
        let mut mutable = MutableDataset::new(builder.freeze().unwrap());
        let record = RecordValues {
            kind: RecordKind::Annotation,
            quad: q("r", "p", "o"),
        };
        assert!(mutable.remove_record(&record));
        let declaration = QuadValues::triple(
            iri_val("r"),
            TermValue::iri(RDF_REIFIES),
            TermValue::Triple {
                s: iri_val("s").into(),
                p: iri_val("p").into(),
                o: iri_val("o").into(),
            },
        );
        assert!(mutable.insert(declaration.clone()).unwrap());
        assert_eq!(mutable.freeze().unwrap().quad_count(), 0);
        assert_eq!(mutable.freeze().unwrap().annotations().count(), 1);
        assert!(mutable.remove(&declaration));
        assert_eq!(
            typed_image(&mutable),
            std::iter::once(RecordValues {
                kind: RecordKind::Ordinary,
                quad: record.quad
            })
            .collect()
        );
        assert_eq!(mutable.effective_count(), 1);
        assert_eq!(mutable.suppressed_len(), 1);
    }
    fn typed_image(mutable: &MutableDataset) -> std::collections::BTreeSet<RecordValues> {
        let values = mutable
            .records_for_pattern(None, None, None, GraphMatchValue::Any)
            .unwrap();
        let frozen = mutable.freeze().unwrap();
        let from_frozen = super::super::import::record_values(frozen.as_ref()).unwrap();
        assert_eq!(values.len(), mutable.effective_count());
        let values: std::collections::BTreeSet<_> = values.into_iter().collect();
        assert_eq!(values.len(), mutable.effective_count());
        assert_eq!(values, from_frozen.into_iter().collect());
        values
    }

    #[test]
    fn typed_three_role_state_machine_matches_physical_model() {
        for mode in [
            GraphExistenceMode::Implicit,
            GraphExistenceMode::RememberEmpty,
        ] {
            let mut mutable = MutableDataset::new_with_graph_existence(
                RdfDatasetBuilder::new().freeze().unwrap(),
                mode,
            );
            let mut model = std::collections::BTreeSet::new();
            for step in 0..120usize {
                let kind = RecordKind::ALL[step % 3];
                let object = TermValue::Triple {
                    s: iri_val("s").into(),
                    p: iri_val("p").into(),
                    o: iri_val(&format!("o{}", step % 5)).into(),
                };
                let quad = QuadValues::quad(
                    iri_val("r"),
                    TermValue::iri(RDF_REIFIES),
                    object,
                    iri_val(&format!("g{}", step % 2)),
                );
                let record = RecordValues { kind, quad };
                if (step / 3) % 4 == 0 {
                    assert_eq!(mutable.remove_record(&record), model.remove(&record));
                } else {
                    let changed = model.insert(record.clone());
                    assert_eq!(mutable.insert_record(&record).unwrap(), changed);
                }
                assert_eq!(typed_image(&mutable), model, "{mode:?} step {step}");
            }
        }
    }

    #[test]
    fn typed_exact_ingress_does_not_classify_and_snapshots_are_independent() {
        let mut mutable = MutableDataset::new(RdfDatasetBuilder::new().freeze().unwrap());
        let quad = QuadValues::triple(
            iri_val("r"),
            TermValue::iri(RDF_REIFIES),
            TermValue::Triple {
                s: iri_val("s").into(),
                p: iri_val("p").into(),
                o: iri_val("o").into(),
            },
        );
        for kind in RecordKind::ALL {
            assert!(
                mutable
                    .insert_record(&RecordValues {
                        kind,
                        quad: quad.clone()
                    })
                    .unwrap()
            );
        }
        assert_eq!(mutable.added_len(), 1);
        assert_eq!(mutable.effective_count(), 3);
        let retained = mutable.snapshot_view().unwrap();
        let image = super::super::import::record_values(&retained).unwrap();
        assert_eq!(image.len(), 3);
        assert!(mutable.remove(&quad));
        assert_eq!(mutable.effective_count(), 0);
        assert_eq!(
            super::super::import::record_values(&retained).unwrap(),
            image
        );
        assert!(
            mutable
                .insert_record(&RecordValues {
                    kind: RecordKind::Annotation,
                    quad
                })
                .unwrap()
        );
        assert_eq!(typed_image(&mutable).len(), 1);
        let fresh = MutableDataset::new(mutable.freeze().unwrap());
        assert_eq!(typed_image(&fresh), typed_image(&mutable));
    }

    #[test]
    fn typed_ingress_refuses_invalid_roles_without_membership_or_term_publication() {
        let mut mutable = MutableDataset::new(RdfDatasetBuilder::new().freeze().unwrap());
        for (kind, quad) in [
            (RecordKind::Reifier, q("s", "p", "o")),
            (
                RecordKind::Ordinary,
                QuadValues::triple(TermValue::simple_literal("bad"), iri_val("p"), iri_val("o")),
            ),
            (
                RecordKind::Annotation,
                QuadValues::triple(iri_val("s"), TermValue::blank("p"), iri_val("o")),
            ),
            (
                RecordKind::Ordinary,
                QuadValues::quad(
                    iri_val("s"),
                    iri_val("p"),
                    iri_val("o"),
                    TermValue::simple_literal("graph"),
                ),
            ),
            (
                RecordKind::Ordinary,
                QuadValues::triple(TermValue::iri("relative"), iri_val("p"), iri_val("o")),
            ),
        ] {
            assert!(mutable.insert_record(&RecordValues { kind, quad }).is_err());
            assert_eq!(mutable.delta.values.len(), 0);
            assert_eq!(mutable.effective_count(), 0);
        }
    }

    #[test]
    fn typed_snapshot_limits_price_role_masks_and_refusal_adds_no_successful_work() {
        let mut builder = RdfDatasetBuilder::new();
        let s = builder.intern_iri("http://example.org/s");
        let p = builder.intern_iri("http://example.org/p");
        let o = builder.intern_iri("http://example.org/o");
        builder.push_quad(s, p, o, None);
        builder.push_annotation(s, p, o);
        let mut mutable = MutableDataset::new(builder.freeze().unwrap());
        assert!(mutable.remove(&q("s", "p", "o")));
        let view = mutable.snapshot_view().unwrap();
        let stats = view.stats();
        assert_eq!(
            stats.auxiliary_bytes,
            2 * 4 * size_of::<(RecordKind, super::super::QuadIds)>()
        );
        let before = mutable.work_stats();
        assert!(
            mutable
                .snapshot_view_with_limits(crate::ViewLimits {
                    max_auxiliary_bytes: stats.auxiliary_bytes - 1,
                    ..Default::default()
                })
                .is_err()
        );
        assert_eq!(mutable.work_stats(), before);
        assert!(
            mutable
                .snapshot_view_with_limits(crate::ViewLimits {
                    max_auxiliary_bytes: stats.auxiliary_bytes,
                    ..Default::default()
                })
                .is_ok()
        );
    }
    #[test]
    fn typed_roles_restore_one_and_ordinary_mutation_restores_all() {
        for mode in [
            GraphExistenceMode::Implicit,
            GraphExistenceMode::RememberEmpty,
        ] {
            let mut builder = RdfDatasetBuilder::new();
            let s = builder.intern_iri("http://example.org/s");
            let p = builder.intern_iri("http://example.org/p");
            let o = builder.intern_iri("http://example.org/o");
            let g = builder.intern_iri("http://example.org/g");
            builder.push_quad(s, p, o, Some(g));
            builder.push_annotation_in_graph(s, p, o, Some(g));
            let mut mutable =
                MutableDataset::new_with_graph_existence(builder.freeze().unwrap(), mode);
            let quad = QuadValues::quad(iri_val("s"), iri_val("p"), iri_val("o"), iri_val("g"));
            assert!(mutable.remove(&quad));
            assert_eq!(mutable.effective_count(), 0);
            assert_eq!(mutable.suppressed_len(), 1);
            let record = RecordValues {
                kind: RecordKind::Annotation,
                quad: quad.clone(),
            };
            assert!(mutable.insert_record(&record).unwrap());
            assert!(!mutable.insert_record(&record).unwrap());
            assert_eq!(mutable.effective_count(), 1);
            assert_eq!(mutable.suppressed_len(), 1);
            assert_eq!(mutable.added_len(), 0);
            assert_eq!(mutable.freeze().unwrap().quad_count(), 0);
            assert_eq!(mutable.freeze().unwrap().annotations().count(), 1);
            assert!(mutable.insert(quad.clone()).unwrap());
            assert_eq!(mutable.effective_count(), 2);
            assert_eq!(mutable.suppressed_len(), 0);
            assert_eq!(
                mutable
                    .records_for_pattern(None, None, None, GraphMatchValue::Any)
                    .unwrap()
                    .len(),
                2
            );
            assert!(mutable.remove_record(&record));
            assert_eq!(mutable.effective_count(), 1);
            assert_eq!(mutable.freeze().unwrap().quad_count(), 1);
        }
    }
    #[test]
    fn classification_collision_counts_the_actual_physical_rows() {
        let mut builder = RdfDatasetBuilder::new();
        let r = builder.intern_iri("http://example.org/r");
        let p = builder.intern_iri("http://example.org/p");
        let o = builder.intern_iri("http://example.org/o");
        builder.push_quad(r, p, o, None);
        builder.push_annotation(r, p, o);
        let mut mutable = MutableDataset::new(builder.freeze().unwrap());
        assert!(
            mutable
                .insert(QuadValues::triple(
                    iri_val("r"),
                    TermValue::iri(RDF_REIFIES),
                    TermValue::Triple {
                        s: iri_val("r").into(),
                        p: iri_val("p").into(),
                        o: iri_val("o").into()
                    },
                ))
                .unwrap()
        );
        let frozen = mutable.freeze().unwrap();
        assert_eq!(frozen.rdf_row_count(), 2);
        assert_eq!(mutable.effective_count(), frozen.rdf_row_count());
    }
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
        let mut m =
            MutableDataset::new_with_graph_existence(empty_base(), GraphExistenceMode::Implicit);
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
        let mut m =
            MutableDataset::new_with_graph_existence(empty_base(), GraphExistenceMode::Implicit);
        m.declare_named_graph(iri_val("g")).expect("declares");
        let row = QuadValues::quad(iri_val("s"), iri_val("p"), iri_val("o"), iri_val("g"));
        ins(&mut m, row.clone());
        m.withdraw_graph_declaration(&iri_val("g"));
        assert_eq!(names(&m), std::collections::BTreeSet::from([iri_val("g")]));
        let mut m =
            MutableDataset::new_with_graph_existence(empty_base(), GraphExistenceMode::Implicit);
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
            let mut m = MutableDataset::new_with_graph_existence(
                base.expect("base freezes"),
                GraphExistenceMode::Implicit,
            );
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
        let m =
            MutableDataset::new_with_graph_existence(graph_base(), GraphExistenceMode::Implicit);
        assert_eq!(graph_names(&m), ["empty", "one", "stmt", "two"]);
    }

    #[test]
    fn removing_the_last_quad_of_a_graph_withdraws_the_graph() {
        let mut m =
            MutableDataset::new_with_graph_existence(graph_base(), GraphExistenceMode::Implicit);
        assert!(m.remove(&in_graph("a", "c", "one")));
        assert_eq!(graph_names(&m), ["empty", "stmt", "two"]);
    }

    #[test]
    fn removing_some_but_not_all_quads_of_a_graph_keeps_it() {
        let mut m =
            MutableDataset::new_with_graph_existence(graph_base(), GraphExistenceMode::Implicit);
        assert!(m.remove(&in_graph("a", "c", "two")));
        assert_eq!(graph_names(&m), ["empty", "one", "stmt", "two"]);
        assert!(m.remove(&in_graph("a", "d", "two")));
        assert_eq!(graph_names(&m), ["empty", "one", "stmt"]);
    }

    #[test]
    fn removing_the_only_reifier_of_a_graph_withdraws_the_graph() {
        let mut m =
            MutableDataset::new_with_graph_existence(graph_base(), GraphExistenceMode::Implicit);
        assert!(m.remove(&reifier_in("stmt")));
        assert_eq!(graph_names(&m), ["empty", "one", "two"]);
    }

    #[test]
    fn a_graph_repopulated_after_losing_its_last_quad_exists() {
        // Re-inserting the same base quad (un-suppression) and inserting a different
        // quad (a delta row in a base graph) both bring the graph back.
        let mut m =
            MutableDataset::new_with_graph_existence(graph_base(), GraphExistenceMode::Implicit);
        assert!(m.remove(&in_graph("a", "c", "one")));
        assert!(ins(&mut m, in_graph("a", "c", "one")));
        assert_eq!(graph_names(&m), ["empty", "one", "stmt", "two"]);
        assert!(m.remove(&in_graph("a", "c", "one")));
        assert!(ins(&mut m, in_graph("x", "y", "one")));
        assert_eq!(graph_names(&m), ["empty", "one", "stmt", "two"]);
    }

    #[test]
    fn an_unrelated_insert_keeps_the_declared_empty_graph() {
        let mut m =
            MutableDataset::new_with_graph_existence(graph_base(), GraphExistenceMode::Implicit);
        assert!(ins(&mut m, q("x", "p", "y")));
        assert!(ins(&mut m, in_graph("x", "y", "fresh")));
        assert_eq!(graph_names(&m), ["empty", "fresh", "one", "stmt", "two"]);
    }

    #[test]
    fn a_delta_graph_emptied_again_is_not_enumerated() {
        let mut m =
            MutableDataset::new_with_graph_existence(graph_base(), GraphExistenceMode::Implicit);
        assert!(ins(&mut m, in_graph("x", "y", "fresh")));
        assert!(m.remove(&in_graph("x", "y", "fresh")));
        assert_eq!(graph_names(&m), ["empty", "one", "stmt", "two"]);
    }

    #[test]
    fn emptying_a_declared_empty_graph_after_populating_it_withdraws_it() {
        // The declared empty graph gains a quad and then loses it: the removal of its
        // last quad is an operation that leaves it empty, so it is withdrawn.
        let mut m =
            MutableDataset::new_with_graph_existence(graph_base(), GraphExistenceMode::Implicit);
        assert!(ins(&mut m, in_graph("x", "y", "empty")));
        assert_eq!(graph_names(&m), ["empty", "one", "stmt", "two"]);
        assert!(m.remove(&in_graph("x", "y", "empty")));
        assert_eq!(graph_names(&m), ["one", "stmt", "two"]);
    }

    #[test]
    fn withdrawing_a_declaration_hides_only_an_empty_graph() {
        let mut m =
            MutableDataset::new_with_graph_existence(graph_base(), GraphExistenceMode::Implicit);
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
        let mut m = MutableDataset::new_with_graph_existence(base, GraphExistenceMode::Implicit);
        assert!(m.remove(&in_graph("a", "c", "mix")));
        assert_eq!(graph_names(&m), ["mix"], "a reifier and annotation remain");
        assert!(m.remove(&reifier_in("mix")));
        assert_eq!(graph_names(&m), ["mix"], "the (demoted) annotation remains");
        assert!(m.remove(&annotation));
        assert!(graph_names(&m).is_empty(), "every row is gone");
        // Order does not matter: the annotation first, then the reifier.
        let mut m = MutableDataset::new_with_graph_existence(
            m.base().clone(),
            GraphExistenceMode::Implicit,
        );
        assert!(m.remove(&annotation));
        assert!(m.remove(&in_graph("a", "c", "mix")));
        assert_eq!(graph_names(&m), ["mix"], "the reifier remains");
        assert!(m.remove(&reifier_in("mix")));
        assert!(graph_names(&m).is_empty(), "every row is gone");
    }

    #[test]
    fn withdrawing_every_declaration_keeps_only_populated_graphs() {
        let mut m =
            MutableDataset::new_with_graph_existence(graph_base(), GraphExistenceMode::Implicit);
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
            let mut m = MutableDataset::new_with_graph_existence(base, GraphExistenceMode::Implicit);
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

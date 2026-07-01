// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The copy-on-write, suppression-delta **mutable dataset** (purrdf P5, #839).
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
//! and [`MutableDataset::freeze`] is the **compaction** pass that re-interns the
//! effective set (terms, reifiers, annotations, graph names, locations) into a fresh
//! frozen [`RdfDataset`] through the existing [`RdfDatasetBuilder`].
//!
//! # Term identity — TAGGED handles, never a numeric threshold
//!
//! A plain two-tier numeric `TermId` would break the load-bearing invariant that a
//! [`TermId`] belongs to exactly ONE frozen dataset (C0.8). So the mutable layer
//! never widens `TermId`; instead it works in [`MutTermId`], a tagged enum of
//! `Base(TermId)` (an id into the frozen base) and `Delta(DeltaTermId)` (an index
//! into the delta's own small interner). When a quad mentions a term, the layer asks
//! the base `term_id_by_value(&TermValue)`: a hit binds `Base`, a miss mints a
//! `Delta`. `MutTermId`/`DeltaTermId` are strictly INTERNAL — the outside world only
//! ever sees frozen base `TermId`s (pre-mutation) or post-`freeze()` dense `TermId`s.
//!
//! # The four mutation rules (P5, explicit + unit-tested)
//!
//! 1. insert of a currently-SUPPRESSED *base* quad → **un-suppresses** it (removes
//!    it from `suppressed`), and does NOT also add it to `added`.
//! 2. remove of a *delta-added* quad → **drops it from `added`**, and does NOT
//!    create a suppression.
//! 3. remove of a *base* quad (not in `added`) → **creates a suppression**.
//! 4. reinsert-after-removal is consistent with both orders (insert→remove→insert
//!    and remove→insert→… both return to "present").

use std::collections::HashSet;
use std::sync::Arc;

use crate::dataset_view::{DatasetMut, GraphMatch, GraphMatchValue};
use crate::ir::{RdfDataset, RdfDatasetBuilder, TermValue};
use crate::model::RdfLiteral;

use super::dataset::{QuadHandle, TermRef};
use super::term::TermId;

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
    /// Reverse hash→id index, mirroring the base's `value_index` in `dataset.rs`:
    /// keyed by a canonical hash of the term VALUE with `Vec<DeltaTermId>` collision
    /// buckets, so interning and lookup are O(1) expected instead of a linear scan.
    /// The hash is in-memory only (never persisted), so a fixed-seed `DefaultHasher`
    /// is fine and matches the `dataset.rs` precedent.
    index: std::collections::HashMap<u64, Vec<DeltaTermId>>,
}

impl DeltaBuilder {
    /// Canonical hash of a [`TermValue`], matching the base's `value_index` keying.
    fn hash_of(value: &TermValue) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    /// Intern a delta term BY VALUE, returning its [`DeltaTermId`]. Idempotent: equal
    /// values dedup to one id (probe the hash bucket, compare candidates by `==`).
    fn intern(&mut self, value: TermValue) -> DeltaTermId {
        let h = Self::hash_of(&value);
        let bucket = self.index.entry(h).or_default();
        for &did in bucket.iter() {
            if self.values[did.index()] == value {
                return did;
            }
        }
        let i = u32::try_from(self.values.len()).expect("delta term table exceeds u32::MAX");
        let did = DeltaTermId(i);
        bucket.push(did);
        self.values.push(value);
        did
    }

    /// Find an already-interned [`TermValue`] WITHOUT minting; `None` if absent.
    fn find(&self, value: &TermValue) -> Option<DeltaTermId> {
        let bucket = self.index.get(&Self::hash_of(value))?;
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

/// A copy-on-write mutable RDF dataset (purrdf P5, #839). Branches cheaply off a
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
    /// The delta's own term interner (mints `Delta` ids for brand-new terms).
    delta: DeltaBuilder,
    /// Quads added on top of the base, in [`MutTermId`] space, deduplicated by value.
    added: HashSet<QuadKey>,
    /// Base quads suppressed (logically removed). A base quad is effective iff it is
    /// NOT in this set.
    suppressed: HashSet<QuadKey>,
}

impl MutableDataset {
    /// Branch a fresh mutable dataset off a shared frozen `base`. O(1): only the
    /// `Arc` refcount is touched; no quad/term is copied.
    #[must_use]
    pub fn new(base: Arc<RdfDataset>) -> Self {
        Self {
            base,
            delta: DeltaBuilder::default(),
            added: HashSet::new(),
            suppressed: HashSet::new(),
        }
    }

    /// The shared frozen base this dataset branched from.
    #[must_use]
    pub fn base(&self) -> &Arc<RdfDataset> {
        &self.base
    }

    // -- value ↔ MutTermId resolution -------------------------------------------------

    /// Resolve a base [`TermId`] to its dataset-independent [`TermValue`], recursing
    /// through datatype ids and triple components. The inverse of interning a value.
    fn base_value(&self, id: TermId) -> TermValue {
        Self::base_value_of(&self.base, id)
    }

    /// `base_value` without `&self`, so it can be reused by `freeze`'s remap closures.
    fn base_value_of(base: &RdfDataset, id: TermId) -> TermValue {
        match base.resolve(id) {
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
                let datatype = match base.resolve(datatype) {
                    TermRef::Iri(dt) => dt.to_string(),
                    // Unreachable for a validated dataset (a literal datatype is always
                    // an IRI); fall back to a debug string rather than panic.
                    other => format!("{other:?}"),
                };
                TermValue::Literal {
                    lexical_form: lexical.to_string(),
                    datatype,
                    language: language.map(str::to_string),
                    direction,
                }
            }
            TermRef::Triple { s, p, o } => TermValue::Triple {
                s: Box::new(Self::base_value_of(base, s)),
                p: Box::new(Self::base_value_of(base, p)),
                o: Box::new(Self::base_value_of(base, o)),
            },
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
    fn resolve_value(&mut self, value: &TermValue) -> MutTermId {
        if let Some(id) = self.base.term_id_by_value(value) {
            return MutTermId::Base(id);
        }
        MutTermId::Delta(self.delta.intern(value.clone()))
    }

    /// Build a [`QuadKey`] from a value-quad, resolving each component to a
    /// [`MutTermId`] (minting delta ids for new terms as a side effect).
    fn key_of(&mut self, quad: &QuadValues) -> QuadKey {
        QuadKey {
            s: self.resolve_value(&quad.s),
            p: self.resolve_value(&quad.p),
            o: self.resolve_value(&quad.o),
            g: quad.g.as_ref().map(|g| self.resolve_value(g)),
        }
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
        // their canonical hash, just like the base's `value_index`.
        self.delta.find(value).map(MutTermId::Delta)
    }

    /// Whether a base [`QuadKey`] (all components `Base`) names a quad in the base.
    fn base_contains(&self, key: &QuadKey) -> bool {
        let (MutTermId::Base(s), MutTermId::Base(p), MutTermId::Base(o)) = (key.s, key.p, key.o)
        else {
            return false;
        };
        let g = match key.g {
            None => GraphMatch::Default,
            Some(MutTermId::Base(g)) => GraphMatch::Named(g),
            // A delta graph id can never name a base quad.
            Some(MutTermId::Delta(_)) => return false,
        };
        RdfDataset::quads_for_pattern_indexed(&self.base, Some(s), Some(p), Some(o), g)
            .next()
            .is_some()
    }

    // -- mutation core ----------------------------------------------------------------

    /// Insert an effective quad (the four rules, insert side). Returns `true` if the
    /// effective set changed.
    fn insert_key(&mut self, key: QuadKey) -> bool {
        // Rule 1: inserting a currently-suppressed base quad un-suppresses it (and
        // does NOT also push to `added`).
        if self.suppressed.remove(&key) {
            return true;
        }
        // Already effective (present in base-and-not-suppressed, or already added)?
        if self.contains_key(&key) {
            return false;
        }
        self.added.insert(key)
    }

    /// Remove an effective quad (the four rules, remove side). Returns `true` if the
    /// effective set changed.
    fn remove_key(&mut self, key: QuadKey) -> bool {
        // Rule 2: removing a delta-added quad drops it from `added` (no suppression).
        if self.added.remove(&key) {
            return true;
        }
        // Rule 3: removing a base quad (not in `added`) creates a suppression — but
        // only if it is actually an effective base quad and not already suppressed.
        if self.base_contains(&key) && !self.suppressed.contains(&key) {
            return self.suppressed.insert(key);
        }
        false
    }

    /// Whether a [`QuadKey`] is in the effective set: `(base ∪ added) − suppressed`.
    fn contains_key(&self, key: &QuadKey) -> bool {
        if self.suppressed.contains(key) {
            return false;
        }
        self.added.contains(key) || self.base_contains(key)
    }

    /// A signal that the delta has grown enough that compacting (re-`freeze()` to a
    /// fresh base) is worthwhile — when the added + suppressed churn exceeds a
    /// fraction (here ½) of the base quad count. Advisory only; correctness never
    /// depends on it. An empty base always signals once churn appears, so the first
    /// build off a trivial base still compacts.
    #[must_use]
    pub fn should_compact(&self) -> bool {
        let churn = self.added.len() + self.suppressed.len();
        let base = self.base.quad_count();
        churn * 2 > base
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

    /// Iterate the effective quads as value-quads — the independent test/proptest
    /// oracle for the effective set. `freeze` builds the effective set directly (so it
    /// can carry per-base-quad source locations), so this is a test-only helper; the
    /// public surface is [`DatasetMut`].
    #[cfg(test)]
    fn effective_value_quads(&self) -> Vec<QuadValues> {
        let mut out: Vec<QuadValues> = Vec::new();
        // Base quads that are not suppressed.
        for q in self.base.quads() {
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
        // Delta-added quads.
        for key in &self.added {
            out.push(self.quad_values_of(key));
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
    /// The mechanism re-uses the existing compaction/validation engine: every
    /// effective quad's component is RESOLVED to its dataset-independent value and
    /// RE-INTERNED into a fresh [`RdfDatasetBuilder`] (recursively for triple terms),
    /// then pushed. The base's reifiers/annotations are carried through the SAME
    /// resolve→re-intern path so they survive compaction with remapped ids; a base
    /// annotation whose reifier binds a triple-term that is no longer effective is
    /// still carried (reification metadata is independent of quad suppression, matching
    /// the base's own freeze semantics).
    ///
    /// Source LOCATIONS of the surviving base quads are carried too: a base quad is
    /// pushed in base order (so a base ordinal maps to a running new ordinal), and its
    /// location — if the base recorded one — is re-attached to that new handle. The
    /// builder's own freeze sort then remaps the handle to the dense frozen position
    /// (the `attach_location` contract). Delta-added quads were minted in memory, not
    /// parsed from a source, so they carry no location.
    pub fn freeze(&self) -> Result<Arc<RdfDataset>, crate::RdfDiagnostic> {
        let mut builder = RdfDatasetBuilder::new();
        let base = &*self.base;

        // Running count of quads PUSHED so far == the next quad's builder ordinal.
        // Base quads are distinct and `added` quads are non-base, so no push collapses
        // by dedup; the counter therefore tracks the pre-freeze pushed-quad ordinal
        // that `attach_location` keys off (freeze's own sort then remaps it to the
        // dense frozen position — see `location_follows_quad_through_freeze_sort`).
        let mut new_ord: u32 = 0;

        // 1. Surviving BASE quads, remapped via value re-intern, carrying any source
        //    location across the base-ordinal -> new-handle mapping.
        for (base_ord, q) in base.quads().enumerate() {
            // The base quad's effective key (all components are `Base`). Skip it if it
            // is suppressed — gone from the effective set.
            let key = QuadKey {
                s: MutTermId::Base(q.s),
                p: MutTermId::Base(q.p),
                o: MutTermId::Base(q.o),
                g: q.g.map(MutTermId::Base),
            };
            if self.suppressed.contains(&key) {
                continue;
            }
            let s = self.intern_base(&mut builder, q.s);
            let p = self.intern_base(&mut builder, q.p);
            let o = self.intern_base(&mut builder, q.o);
            let g = q.g.map(|g| self.intern_base(&mut builder, g));
            builder.push_quad(s, p, o, g);
            // Carry the base quad's source location, if any, keyed to its NEW pushed
            // ordinal (`new_ord`), not its base ordinal.
            if let Some(loc) = base.location_of(QuadHandle::from_index(base_ord as u32)) {
                builder.attach_location(QuadHandle::from_index(new_ord), loc.clone());
            }
            new_ord += 1;
        }

        // 2. DELTA-added quads (no source location — they were minted in memory, not
        //    parsed from a source, so the `new_ord` mapping ends here). Remapped via
        //    value re-intern.
        let _ = new_ord; // last value consumed by the base loop; delta quads add none.
        for key in &self.added {
            let q = self.quad_values_of(key);
            let s = intern_value(&mut builder, &q.s);
            let p = intern_value(&mut builder, &q.p);
            let o = intern_value(&mut builder, &q.o);
            let g = q.g.as_ref().map(|g| intern_value(&mut builder, g));
            builder.push_quad(s, p, o, g);
        }

        // Carry the base's reifiers + annotations through the same path. Their term
        // ids are BASE ids, so resolve each to a value and re-intern into the builder.
        for (reifier, triple) in base.reifiers() {
            let reifier = self.intern_base(&mut builder, reifier);
            let triple = self.intern_base(&mut builder, triple);
            builder.push_reifier(reifier, triple);
        }
        for (reifier, pred, obj) in base.annotations() {
            let reifier = self.intern_base(&mut builder, reifier);
            let pred = self.intern_base(&mut builder, pred);
            let obj = self.intern_base(&mut builder, obj);
            builder.push_annotation(reifier, pred, obj);
        }

        builder.freeze()
    }

    /// Resolve a BASE term id to its value and re-intern it into `builder`, returning
    /// the builder's fresh dense id. Used to carry reifiers/annotations across freeze.
    fn intern_base(&self, builder: &mut RdfDatasetBuilder, id: TermId) -> TermId {
        let value = self.base_value(id);
        intern_value(builder, &value)
    }
}

/// Re-intern a dataset-independent [`TermValue`] into a fresh builder, recursing for
/// triple terms, and return the builder's dense [`TermId`]. The single remap
/// primitive `freeze` drives every table through.
fn intern_value(builder: &mut RdfDatasetBuilder, value: &TermValue) -> TermId {
    match value {
        TermValue::Iri(iri) => builder.intern_iri(iri.clone()),
        TermValue::Blank { label, scope } => builder.intern_blank(label.clone(), *scope),
        TermValue::Literal {
            lexical_form,
            datatype,
            language,
            direction,
        } => {
            // Rebuild the owned literal. The C0.1 policy was already applied when the
            // value was produced (datatype expanded, language lowercased), and the
            // builder re-applies it idempotently, so this round-trips to the same id.
            let lit = RdfLiteral {
                lexical_form: lexical_form.clone(),
                datatype: Some(datatype.clone()),
                language: language.clone(),
                direction: *direction,
            };
            builder.intern_literal(lit)
        }
        TermValue::Triple { s, p, o } => {
            let s = intern_value(builder, s);
            let p = intern_value(builder, p);
            let o = intern_value(builder, o);
            builder.intern_triple(s, p, o)
        }
    }
}

/// An owned, dataset-independent quad value — the argument type of
/// [`DatasetMut::insert`]/[`remove`](DatasetMut::remove)/[`contains`](DatasetMut::contains).
///
/// Insert/remove take terms BY VALUE (each component a [`TermValue`]) rather than by
/// id because a `MutableDataset`'s caller does not hold dataset-local ids for
/// brand-new terms (those don't exist until minted), and a value is the only
/// identity that is well-defined across the base/delta boundary (C0.8). The mutable
/// layer resolves each value to a [`MutTermId`] (base hit, or a freshly-minted delta
/// id) internally.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct QuadValues {
    pub s: TermValue,
    pub p: TermValue,
    pub o: TermValue,
    pub g: Option<TermValue>,
}

impl QuadValues {
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
}

impl DatasetMut for MutableDataset {
    type Quad = QuadValues;

    fn insert(&mut self, quad: Self::Quad) -> bool {
        let key = self.key_of(&quad);
        self.insert_key(key)
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
        let (sb, pb, ob) = match (resolve_bound(s), resolve_bound(p), resolve_bound(o)) {
            (Ok(sb), Ok(pb), Ok(ob)) => (sb, pb, ob),
            _ => return Vec::new(),
        };
        // The graph filter, in MutTermId space. The named graph is matched BY VALUE
        // (resolved without minting), so both a base-named and a delta-only-named
        // graph are expressible. A `Named` value interned nowhere — in neither base
        // nor delta — names no graph at all, so the whole pattern yields empty
        // (mirroring a bound `s`/`p`/`o` miss above).
        let gb: GraphMatchMut = match g {
            GraphMatchValue::Any => GraphMatchMut::Any,
            GraphMatchValue::Default => GraphMatchMut::Default,
            GraphMatchValue::Named(value) => match self.find_value(value) {
                Some(id) => GraphMatchMut::Named(id),
                None => return Vec::new(),
            },
        };

        self.effective_keys()
            .into_iter()
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

/// The [`GraphMatch`] equivalent in [`MutTermId`] space (the mutable view's named
/// graph can be a base OR delta id).
#[derive(Clone, Copy)]
enum GraphMatchMut {
    Any,
    Default,
    Named(MutTermId),
}

impl GraphMatchMut {
    #[inline]
    fn matches(self, g: Option<MutTermId>) -> bool {
        match self {
            GraphMatchMut::Any => true,
            GraphMatchMut::Default => g.is_none(),
            GraphMatchMut::Named(id) => g == Some(id),
        }
    }
}

impl MutableDataset {
    /// All effective quad KEYS (base-not-suppressed ∪ added), in MutTermId space.
    fn effective_keys(&self) -> Vec<QuadKey> {
        let mut out: Vec<QuadKey> = Vec::new();
        for q in self.base.quads() {
            let key = QuadKey {
                s: MutTermId::Base(q.s),
                p: MutTermId::Base(q.p),
                o: MutTermId::Base(q.o),
                g: q.g.map(MutTermId::Base),
            };
            if !self.suppressed.contains(&key) {
                out.push(key);
            }
        }
        out.extend(self.added.iter().copied());
        out
    }

    /// The effective quads as `Copy` base-or-frozen `QuadIds` is NOT exposed: ids
    /// straddling base/delta have no single dataset to be local to (C0.8). Consumers
    /// read values via [`DatasetMut::quads_for_pattern`] or `freeze()` to a frozen
    /// dataset and read `QuadIds` there.
    #[doc(hidden)]
    pub fn effective_count(&self) -> usize {
        // O(1) from the mutation invariants (no base scan):
        //   • every key in `suppressed` is a base quad (rule 3 inserts only when
        //     `base_contains`), so `suppressed.len()` base quads are removed;
        //   • every key in `added` is a non-base, non-suppressed quad (insert adds
        //     only when `!contains_key`), so `added.len()` quads are net-new;
        //   • `added` and `suppressed` are disjoint.
        // Hence effective = base ∪ added − suppressed has exactly this cardinality.
        self.base.quad_count() + self.added.len() - self.suppressed.len()
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
    use pretty_assertions::assert_eq;
    use proptest::prelude::*;

    // -- helpers ----------------------------------------------------------------------

    fn iri_val(n: &str) -> TermValue {
        TermValue::Iri(format!("http://example.org/{n}"))
    }

    fn q(s: &str, p: &str, o: &str) -> QuadValues {
        QuadValues::triple(iri_val(s), iri_val(p), iri_val(o))
    }

    /// A base with three quads: (a,p,b), (a,p,c), (b,p,c) — and one reifier+annotation.
    fn base3() -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let a = b.intern_iri("http://example.org/a".to_string());
        let p = b.intern_iri("http://example.org/p".to_string());
        let bb = b.intern_iri("http://example.org/b".to_string());
        let c = b.intern_iri("http://example.org/c".to_string());
        b.push_quad(a, p, bb, None);
        b.push_quad(a, p, c, None);
        b.push_quad(bb, p, c, None);
        // A reifier + annotation over the (a,p,b) triple term.
        let triple = b.intern_triple(a, p, bb);
        let r = b.intern_iri("http://example.org/r".to_string());
        let conf = b.intern_iri("http://example.org/confidence".to_string());
        let score = b.intern_literal(RdfLiteral::typed(
            "0.9",
            "http://www.w3.org/2001/XMLSchema#decimal",
        ));
        b.push_reifier(r, triple);
        b.push_annotation(r, conf, score);
        b.freeze().expect("base freezes")
    }

    /// The effective value-quad set as a comparable `BTreeSet` of stringy tuples.
    fn eff_set(m: &MutableDataset) -> std::collections::BTreeSet<String> {
        m.effective_value_quads()
            .iter()
            .map(|q| format!("{:?}|{:?}|{:?}|{:?}", q.s, q.p, q.o, q.g))
            .collect()
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
        assert!(m.insert(quad.clone()));
        assert_eq!(m.suppressed_len(), 0, "un-suppressed");
        assert_eq!(m.added_len(), 0, "not pushed to added");
        assert!(m.contains(&quad));
    }

    #[test]
    fn rule2_remove_of_delta_added_drops_from_added() {
        let mut m = MutableDataset::new(base3());
        let quad = q("x", "p", "y"); // brand-new (delta) quad
        assert!(m.insert(quad.clone()));
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
        assert!(m.insert(nq.clone()));
        assert!(m.remove(&nq));
        assert!(m.insert(nq.clone()));
        assert!(m.contains(&nq));
        assert_eq!(m.added_len(), 1);
        assert_eq!(m.suppressed_len(), 0);

        // remove → insert returns to present (base quad).
        let mut m = MutableDataset::new(base3());
        let bq = q("b", "p", "c");
        assert!(m.remove(&bq));
        assert!(!m.contains(&bq));
        assert!(m.insert(bq.clone()));
        assert!(m.contains(&bq));
        assert_eq!(m.suppressed_len(), 0);
        assert_eq!(m.added_len(), 0, "base quad re-presented by un-suppress");
    }

    #[test]
    fn insert_existing_base_quad_is_noop() {
        let mut m = MutableDataset::new(base3());
        let quad = q("a", "p", "b");
        assert!(!m.insert(quad), "already effective → no change");
        assert_eq!(m.added_len(), 0);
    }

    // -- contains / quads_for_pattern reflect the effective set -----------------------

    #[test]
    fn contains_and_pattern_reflect_effective_set() {
        let mut m = MutableDataset::new(base3());
        // Add a quad, remove a base quad.
        m.insert(q("z", "p", "w"));
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
        assert!(gone.is_empty());

        // A bound value interned nowhere matches nothing.
        let nothing = m.quads_for_pattern(
            Some(&iri_val("never-seen")),
            None,
            None,
            GraphMatchValue::Any,
        );
        assert!(nothing.is_empty());
    }

    #[test]
    fn named_graph_quads_round_trip() {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_iri("http://example.org/s".to_string());
        let p = b.intern_iri("http://example.org/p".to_string());
        let o = b.intern_iri("http://example.org/o".to_string());
        let g = b.intern_iri("http://example.org/g".to_string());
        b.push_quad(s, p, o, Some(g));
        let base = b.freeze().unwrap();

        let mut m = MutableDataset::new(base);
        // Add a named-graph quad in a NEW delta graph.
        let nq = QuadValues::quad(iri_val("s2"), iri_val("p"), iri_val("o2"), iri_val("g2"));
        assert!(m.insert(nq.clone()));
        assert!(m.contains(&nq));

        // Default-graph match excludes both named quads.
        let dflt = m.quads_for_pattern(None, None, None, GraphMatchValue::Default);
        assert!(dflt.is_empty());
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
        let s = b.intern_iri("http://example.org/s".to_string());
        let p = b.intern_iri("http://example.org/p".to_string());
        let o = b.intern_iri("http://example.org/o".to_string());
        b.push_quad(s, p, o, None); // default-graph base quad; no g2 anywhere
        let base = b.freeze().unwrap();

        let mut m = MutableDataset::new(base);
        let g2 = iri_val("g2"); // a graph term interned NOWHERE in the base
        let nq = QuadValues::quad(iri_val("s2"), iri_val("p"), iri_val("o2"), g2.clone());
        assert!(m.insert(nq.clone()));

        // Query the delta-only named graph by VALUE — it must return that quad.
        let hits = m.quads_for_pattern(None, None, None, GraphMatchValue::Named(&g2));
        assert_eq!(hits.len(), 1, "delta-only named graph is now queryable");
        assert_eq!(hits[0], nq);

        // A graph value interned nowhere still matches nothing.
        let none = m.quads_for_pattern(None, None, None, GraphMatchValue::Named(&iri_val("nope")));
        assert!(none.is_empty());
    }

    // -- freeze round-trip ------------------------------------------------------------

    #[test]
    fn freeze_round_trip_quads_reifiers_annotations() {
        let mut m = MutableDataset::new(base3());
        // Insert a brand-new quad with a brand-new term, remove a base quad.
        m.insert(q("new", "p", "thing"));
        m.remove(&q("a", "p", "b"));

        let want = eff_set(&m);
        let frozen = m.freeze().expect("freeze compacts");

        // The frozen quad set equals the effective set (compared by value).
        let frozen_set: std::collections::BTreeSet<String> = frozen
            .quads()
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
        let a = b.intern_iri("http://example.org/a".to_string());
        let p = b.intern_iri("http://example.org/p".to_string());
        let bb = b.intern_iri("http://example.org/b".to_string());
        let c = b.intern_iri("http://example.org/c".to_string());

        // (a,p,b) gets a location; (a,p,c) does not.
        let h_ab = b.next_quad_handle();
        b.push_quad(a, p, bb, None);
        b.push_quad(a, p, c, None);
        b.attach_location(h_ab, RdfLocation::logical("loc-a-p-b"));
        let base = b.freeze().expect("base freezes");

        let mut m = MutableDataset::new(base);
        // Insert a brand-new delta quad and suppress a DIFFERENT base quad (a,p,c).
        m.insert(q("x", "p", "y"));
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

        m1.insert(q("only", "in", "one"));
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
        let mut m = MutableDataset::new(base3()); // base of 3 quads
        assert!(!m.should_compact());
        // Add 2 quads: churn 2, base 3 → 2*2=4 > 3 → signal.
        m.insert(q("e1", "p", "o"));
        m.insert(q("e2", "p", "o"));
        assert!(m.should_compact());
    }

    // -- differential proptest --------------------------------------------------------

    // Mirror of `proptest_indexed_pattern_matches_linear_scan`: apply a random
    // sequence of insert/remove ops to BOTH a `MutableDataset` and a reference
    // `HashSet` model of the effective quad-value set, then assert `contains` and the
    // effective set agree, and that `freeze()`'s quad-value set equals the model.
    proptest! {
        #[test]
        fn proptest_mutations_match_hashset_model(
            ops in prop::collection::vec(
                // (is_insert, s, p, o) over a small pool; subjects/objects 0..6 so some
                // collide with the base's a/b/c terms (ids 0..2) and some are new.
                (any::<bool>(), 0u8..6, 0u8..3, 0u8..6),
                0..60,
            )
        ) {
            use std::collections::HashSet;

            let names = ["a", "b", "c", "d", "e", "f"];
            let preds = ["p", "q", "r"];
            let mut m = MutableDataset::new(base3());
            // The reference model: the effective set of (s,p,o) string tuples. Seed it
            // with the base's three quads.
            let mut model: HashSet<(String, String, String)> = HashSet::new();
            model.insert(("a".into(), "p".into(), "b".into()));
            model.insert(("a".into(), "p".into(), "c".into()));
            model.insert(("b".into(), "p".into(), "c".into()));

            for (is_insert, s, p, o) in ops {
                let (sn, pn, on) =
                    (names[s as usize], preds[p as usize], names[o as usize]);
                let quad = q(sn, pn, on);
                let tup = (sn.to_string(), pn.to_string(), on.to_string());
                if is_insert {
                    m.insert(quad);
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
                .quads()
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

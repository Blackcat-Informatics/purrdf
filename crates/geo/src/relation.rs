// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The GeoSPARQL 1.1 **Query Rewrite** extension (Clause 13), as a family of
//! property functions over a projection of the dataset.
//!
//! # What the standard actually says
//!
//! Clause 13 defines each `geo:` spatial relation in RIF Core as the
//! **disjunction of four rules**. Written out for one relation:
//!
//! ```text
//! ?so1[ogc:relation->?so2] :- Or (
//!   And( ?so1[geo:hasDefaultGeometry->?g1]  ?so2[geo:hasDefaultGeometry->?g2]
//!        ?g1[ogc:asGeomLiteral->?s1]  ?g2[ogc:asGeomLiteral->?s2]
//!        External(ogc:function(?s1, ?s2)) )                   # feature - feature
//!   And( ?so1[geo:hasDefaultGeometry->?g1] ?g1[ogc:asGeomLiteral->?s1]
//!        ?so2[ogc:asGeomLiteral->?s2]      External(...) )     # feature - geometry
//!   And( ?so1[ogc:asGeomLiteral->?s1]
//!        ?so2[geo:hasDefaultGeometry->?g2] ?g2[ogc:asGeomLiteral->?s2]
//!        External(...) )                                       # geometry - feature
//!   And( ?so1[ogc:asGeomLiteral->?s1] ?so2[ogc:asGeomLiteral->?s2]
//!        External(...) )                                       # geometry - geometry
//! )
//! ```
//!
//! Three things in that rule are easy to read past, and each one is a
//! wrong-answer channel if it is:
//!
//! * **`?so1` and `?so2` are `geo:SpatialObject`s — either a Feature or a
//!   Geometry.** The four branches exist precisely because either side may be
//!   either kind. Indexing only Features is the classic bug, and the answer it
//!   produces is a **short bag the engine reads as complete**: no error, no
//!   warning, and no symptom other than a row that never arrives.
//! * **The dereferencing property is `geo:hasDefaultGeometry`, not
//!   `geo:hasGeometry`.** A feature may carry many geometries; only the default
//!   one participates in the rewrite. The GeoSPARQL 1.0 spelling
//!   `geo:defaultGeometry` is kept by the ontology as an `owl:equivalentProperty`
//!   of it, so it is accepted here too.
//! * **`:-` is an ENTAILMENT rule, not a definition.** Triples of the relation
//!   that are *asserted* in the data still match, in addition to the computed
//!   ones, so [`GeoIndex`] collects them alongside the geometries.
//!
//! [`GeoIndex`] collapses the four branches into one statement: **a spatial
//! object contributes its own serializations and the serializations of its
//! default geometries.** Index both, and the four branches become one lookup —
//! there is no per-branch code here for one of four cases to be wrong in.
//!
//! # Which serialization properties are in play is the caller's decision
//!
//! `ogc:asGeomLiteral` in the rule above stands for whichever serialization
//! property the **conformance class** names — `geo:asWKT`, `geo:asGeoJSON`, or
//! another. PurRDF mints no vocabulary, so that set is [`GeoIndexConfig`]'s
//! caller-supplied parameter and has no default to fall back on. A configured
//! property whose object carries a serialization this crate does not implement
//! (`geo:gmlLiteral`, `geo:kmlLiteral`, `geo:dggsLiteral`) is a
//! [`GeoError::Unsupported`] naming the datatype, never a skipped row: the caller
//! explicitly put that property in the conformance class, so passing over it
//! silently would drop answers the caller asked for.
//!
//! # The dataset the seam cannot reach
//!
//! [`PropertyFunction::open`] receives no dataset — its signature is
//! `open(&self, args, ceiling)`. So the geometries have to be projected out of the
//! dataset **ahead of time**, into a [`GeoIndex`] the relation holds behind an
//! [`Arc`]. That is what makes an index paired with the *wrong* dataset a silent
//! wrong answer rather than a failure, and it is why [`verify_binding`] exists.

use core::slice;
use std::collections::BTreeMap;
use std::sync::Arc;

use purrdf_core::binding_pattern::BindingPattern;
use purrdf_core::{BlankScope, DatasetView, GraphMatch, RdfTextDirection, TermRef, TermValue};
use purrdf_sparql_eval::{
    EvalError, PfArgs, PfArity, PfCursor, PfRow, PropertyFunction, PropertyFunctionRegistry,
    Volatility,
};

use crate::error::GeoError;
use crate::geom::GeometryLiteral;
use crate::relations::{RelationFamily, SpatialRelation};
use crate::topology::{relate, topological_dimension};
use crate::vocab::{GeoTerm, GeoVocab};
use crate::{geojson, wkt};

/// The subject-side flattened position of a Query Rewrite call — `?so1`.
const SUBJECT: usize = 0;
/// The object-side flattened position of a Query Rewrite call — `?so2`.
const OBJECT: usize = 1;
/// The single access pattern a Query Rewrite relation declares: all-free.
const ALL_FREE_MODE: &str = "ff";
/// The declared arity of every Query Rewrite relation: `?so1 <rel> ?so2`.
const REWRITE_ARITY: PfArity = PfArity::new(1, 1);

// ---------------------------------------------------------------------------
// Digest
// ---------------------------------------------------------------------------

/// FNV-1a's 64-bit offset basis.
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
/// FNV-1a's 64-bit prime.
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
/// The domain-separation prefix of the index source digest.
const DIGEST_DOMAIN: &str = "purrdf-geo/index-source/v1";

/// Digest tag for [`GraphSelector::Any`].
const SELECTOR_ANY: u8 = 0x01;
/// Digest tag for [`GraphSelector::Default`].
const SELECTOR_DEFAULT: u8 = 0x02;
/// Digest tag for [`GraphSelector::Named`].
const SELECTOR_NAMED: u8 = 0x03;

/// Digest tag for an IRI term.
const TERM_IRI: u8 = 0x11;
/// Digest tag for a blank-node term.
const TERM_BLANK: u8 = 0x12;
/// Digest tag for a literal term.
const TERM_LITERAL: u8 = 0x13;
/// Digest tag for a triple term.
const TERM_TRIPLE: u8 = 0x14;
/// Digest presence byte for an absent optional field.
const ABSENT: u8 = 0x20;
/// Digest presence byte for a present optional field.
const PRESENT: u8 = 0x21;

/// A hand-rolled FNV-1a accumulator.
///
/// FNV-1a rather than `std::hash::DefaultHasher` or the workspace's `ahash`
/// because a fingerprint is compared against one computed by a *different run* of
/// this code: `DefaultHasher`'s algorithm is explicitly unspecified across
/// releases, and `ahash`'s output is a function of its version. Either would make
/// [`verify_binding`] answer "different dataset" for a dataset that is in fact
/// identical, the moment a toolchain moved. FNV-1a is a few lines of fully
/// specified integer arithmetic, so the fingerprint is a pure function of the
/// bytes fed to it on every target and every release.
///
/// Every variable-length field is written **length-prefixed**, so no two distinct
/// field sequences can produce the same byte stream by concatenation.
#[derive(Clone, Copy, Debug)]
struct Digest {
    /// The running FNV-1a state.
    state: u64,
}

impl Digest {
    /// A fresh accumulator, domain-separated.
    fn new() -> Self {
        let mut digest = Self {
            state: FNV_OFFSET_BASIS,
        };
        digest.field(DIGEST_DOMAIN);
        digest
    }

    /// Absorb raw bytes.
    fn bytes(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.state ^= u64::from(byte);
            self.state = self.state.wrapping_mul(FNV_PRIME);
        }
    }

    /// Absorb a one-byte tag.
    fn tag(&mut self, tag: u8) {
        self.bytes(&[tag]);
    }

    /// Absorb a `usize` count as eight big-endian bytes.
    fn count(&mut self, count: usize) {
        self.bytes(&(count as u64).to_be_bytes());
    }

    /// Absorb a length-prefixed string.
    fn field(&mut self, text: &str) {
        self.count(text.len());
        self.bytes(text.as_bytes());
    }

    /// Absorb an optional length-prefixed string, presence byte first.
    fn optional(&mut self, text: Option<&str>) {
        match text {
            Some(text) => {
                self.tag(PRESENT);
                self.field(text);
            }
            None => self.tag(ABSENT),
        }
    }

    /// Absorb a term value, tag first, recursing through a triple term's
    /// components.
    ///
    /// The recursion is unbounded by design and by the same contract every other
    /// term walker in this workspace relies on: [`DatasetView`]'s documented
    /// termination obligation makes a triple term's components strictly smaller
    /// than the term naming them, so the structure is finite. A depth cap here
    /// would add a refusal that rejects legal RDF 1.2 data while protecting
    /// against nothing this crate can actually receive.
    fn term(&mut self, value: &TermValue) {
        match value {
            TermValue::Iri(iri) => {
                self.tag(TERM_IRI);
                self.field(iri);
            }
            TermValue::Blank { label, scope } => {
                self.tag(TERM_BLANK);
                self.field(label);
                self.scope(*scope);
            }
            TermValue::Literal {
                lexical_form,
                datatype,
                language,
                direction,
            } => {
                self.tag(TERM_LITERAL);
                self.field(lexical_form);
                self.field(datatype);
                self.optional(language.as_deref());
                self.optional(direction.map(RdfTextDirection::as_str));
            }
            TermValue::Triple { s, p, o } => {
                self.tag(TERM_TRIPLE);
                self.term(s);
                self.term(p);
                self.term(o);
            }
        }
    }

    /// Absorb a blank node's scope ordinal.
    fn scope(&mut self, scope: BlankScope) {
        self.bytes(&scope.ordinal().to_be_bytes());
    }

    /// The accumulated digest.
    const fn finish(self) -> u64 {
        self.state
    }
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Which graph's serializations an index is built over.
///
/// Deliberately in **value** space rather than [`GraphMatch`] space.
/// `GraphMatch::Named` holds a dataset-local term id, which means something only
/// inside the one dataset that minted it; a configuration is a statement the
/// caller writes down once and may apply to several datasets, so it names a graph
/// by its IRI. [`GeoIndex::from_dataset`] resolves the selector against the
/// dataset in hand.
///
/// Deliberately exhaustive, like `GraphMatch`: a quad's graph is the default
/// graph or exactly one named graph, so the three cases are closed.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GraphSelector {
    /// Read serializations from every graph, default and named alike.
    Any,
    /// Read serializations from the default graph only.
    Default,
    /// Read serializations from the one named graph this IRI identifies.
    Named(TermValue),
}

/// The caller's complete, dataset-independent statement of what to project out of
/// a dataset for the Query Rewrite extension.
///
/// There is deliberately no [`Default`] implementation and there never will be
/// one: a default would have to name the `geo:as*` serialization property IRIs,
/// and PurRDF mints no vocabulary IRIs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeoIndexConfig {
    /// The `geo:as*` serialization properties in play, sorted and known to be
    /// distinct IRIs.
    serializations: Vec<TermValue>,
    /// Which graph the index is drawn from.
    graph: GraphSelector,
}

impl GeoIndexConfig {
    /// A configuration over `serializations`, restricted to `graph`.
    ///
    /// `serializations` are the `geo:as*` property IRIs in play for this
    /// conformance class — the rule's `ogc:asGeomLiteral` made concrete. They are
    /// sorted on the way in, so two callers naming the same set in different
    /// orders build byte-identical indexes with equal fingerprints.
    ///
    /// # Errors
    ///
    /// [`GeoError::Config`] if `serializations` is empty (PurRDF mints no
    /// vocabulary, so there is no default serialization property set to fall back
    /// on), if any entry is not an IRI (only an IRI can occupy the predicate
    /// position of an RDF statement, so anything else would index nothing while
    /// looking like it indexed something), if any entry is repeated (a repeat is
    /// a caller mistake, and silently deduplicating it hides the mistake), or if
    /// a [`GraphSelector::Named`] does not hold an IRI.
    pub fn new(serializations: Vec<TermValue>, graph: GraphSelector) -> Result<Self, GeoError> {
        if serializations.is_empty() {
            return Err(GeoError::config(
                "no serialization properties supplied; PurRDF mints no vocabulary, so there is no \
                 default serialization property set to fall back on — name the geo:as* properties \
                 your conformance class puts in play",
            ));
        }
        for property in &serializations {
            if !matches!(property, TermValue::Iri(_)) {
                return Err(GeoError::config(format!(
                    "serialization property {property:?} is not an IRI; only an IRI can occupy \
                     the predicate position of an RDF statement"
                )));
            }
        }
        if let GraphSelector::Named(name) = &graph
            && !matches!(name, TermValue::Iri(_))
        {
            return Err(GeoError::config(format!(
                "named graph selector {name:?} is not an IRI"
            )));
        }

        let mut serializations = serializations;
        serializations.sort();
        for pair in serializations.windows(2) {
            let [left, right] = pair else {
                unreachable!("windows(2) yields pairs")
            };
            if left == right {
                return Err(GeoError::config(format!(
                    "serialization property {left:?} is listed more than once; a repeat is a \
                     caller mistake, and silently deduplicating it would hide the mistake"
                )));
            }
        }

        Ok(Self {
            serializations,
            graph,
        })
    }

    /// The serialization properties, in sorted order.
    #[must_use]
    pub fn serializations(&self) -> &[TermValue] {
        &self.serializations
    }

    /// The graph this configuration draws from.
    #[must_use]
    pub const fn graph(&self) -> &GraphSelector {
        &self.graph
    }

    /// Absorb this configuration into `digest`.
    fn absorb(&self, digest: &mut Digest) {
        digest.count(self.serializations.len());
        for property in &self.serializations {
            digest.term(property);
        }
        match &self.graph {
            GraphSelector::Any => digest.tag(SELECTOR_ANY),
            GraphSelector::Default => digest.tag(SELECTOR_DEFAULT),
            GraphSelector::Named(name) => {
                digest.tag(SELECTOR_NAMED);
                digest.term(name);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The index
// ---------------------------------------------------------------------------

/// One indexed spatial object: its subject, and every geometry that reaches it.
///
/// "Reaches it" is the four-branch collapse of the module docs: the geometries of
/// its own serializations **and** the geometries of every object of its
/// `geo:hasDefaultGeometry` (or the legacy `geo:defaultGeometry`) statements. A
/// `geo:Feature` and a `geo:Geometry` are both `geo:SpatialObject`s and both get
/// an entry, which is what makes the feature/feature, feature/geometry,
/// geometry/feature and geometry/geometry branches one lookup.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeoEntry {
    /// The spatial object itself — an IRI, a blank node, or an RDF 1.2 triple
    /// term, carried verbatim into the emitted row.
    subject: TermValue,
    /// Its geometries, ordered by their canonical WKT rendering and free of exact
    /// duplicates.
    geometries: Vec<GeometryLiteral>,
}

impl GeoEntry {
    /// The spatial object.
    #[must_use]
    pub const fn subject(&self) -> &TermValue {
        &self.subject
    }

    /// Its geometries, in the canonical order [`GeoIndex`] fixes.
    #[must_use]
    pub fn geometries(&self) -> &[GeometryLiteral] {
        &self.geometries
    }
}

/// The projection of a dataset that a Query Rewrite relation is answered from.
///
/// Built once per `(dataset, vocabulary, configuration)` triple and shared behind
/// an [`Arc`], because [`PropertyFunction::open`] receives no dataset and so
/// cannot reach the geometries at query time.
///
/// Everything it holds is **sorted**: the entries by subject, each entry's
/// geometries by their canonical rendering, and each relation's asserted pairs
/// lexicographically. No map's iteration order reaches a result, so two hosts
/// that ingest the same triples in different orders build indexes with the same
/// contents *and* the same [`source_fingerprint`](Self::source_fingerprint).
#[derive(Clone, Debug)]
pub struct GeoIndex {
    /// The configuration this index was built under.
    config: GeoIndexConfig,
    /// Every indexed spatial object, sorted by subject.
    entries: Vec<GeoEntry>,
    /// The asserted pairs of each relation, indexed by that relation's position
    /// in [`SpatialRelation::ALL`], each vector sorted and deduplicated.
    asserted: Vec<Vec<(TermValue, TermValue)>>,
    /// The digest of the source data this index was built from.
    source_fingerprint: u64,
}

impl GeoIndex {
    /// Project `dataset` into an index under `vocab` and `config`.
    ///
    /// # The algorithm, and why each step is there
    ///
    /// 1. For every configured serialization property `P` and every quad
    ///    `(g, P, lit)` in the selected graph, parse `lit` according to its
    ///    **datatype** and record `g -> geometry`.
    /// 2. For every `(f, geo:hasDefaultGeometry, g)` and every
    ///    `(f, geo:defaultGeometry, g)`, `f` inherits every geometry recorded for
    ///    `g` in step 1. The legacy spelling is accepted because the ontology
    ///    keeps it as an `owl:equivalentProperty` of the current one.
    /// 3. **Both** the step-1 subjects and the step-2 subjects become entries,
    ///    because `?so1` may be a Feature or a Geometry. Indexing only the
    ///    features is the short-bag bug the module docs open with.
    /// 4. Asserted `(s, o)` pairs are collected for every relation whose `geo:`
    ///    property IRI appears as a predicate, because the rule is an entailment
    ///    rather than a definition.
    /// 5. Everything is sorted, so no ingestion order can reach a result.
    ///
    /// A spatial object that ends up with **no** geometries is not an entry: the
    /// existential over geometry pairings in the rule's `And` is false for it
    /// under every relation, so it can contribute no computed row, and keeping it
    /// would only inflate the declared row bound. Its asserted triples are
    /// unaffected — those are collected separately, in step 4.
    ///
    /// # Errors
    ///
    /// * [`GeoError::Config`] if a [`GraphSelector::Named`] graph is not interned
    ///   in `dataset` at all — a configuration pointing at a graph that is not
    ///   there is a wiring mistake, not an empty index.
    /// * [`GeoError::Unsupported`], naming the datatype, if a configured
    ///   serialization property's object carries `geo:gmlLiteral`,
    ///   `geo:kmlLiteral` or `geo:dggsLiteral`. The caller put that property in
    ///   the conformance class, so skipping it silently would drop rows.
    /// * [`GeoError::Literal`] if such an object is not a literal at all, or
    ///   carries a datatype that is none of the five GeoSPARQL serializations, or
    ///   is malformed for the datatype it does carry.
    pub fn from_dataset<D: DatasetView>(
        dataset: &D,
        vocab: &GeoVocab,
        config: &GeoIndexConfig,
    ) -> Result<Self, GeoError> {
        let graph = resolve_graph(dataset, config.graph())?;
        let datatypes = Datatypes::of(vocab);

        // Step 1 — the geometry nodes, keyed by dataset id so step 2 can join
        // against them without resolving anything twice.
        let mut by_node: BTreeMap<D::Id, Vec<Keyed>> = BTreeMap::new();
        for property in config.serializations() {
            let Some(predicate) = dataset.term_id_by_value(property) else {
                // A conformance class may name `geo:asGeoJSON` over a dataset
                // that holds only WKT. That is an ordinary empty match, not a
                // configuration error.
                continue;
            };
            for quad in dataset.quads_for_pattern(None, Some(predicate), None, graph) {
                let object = resolve_value(dataset, quad.o);
                let literal = parse_serialization(&object, property, &datatypes, vocab)?;
                by_node
                    .entry(quad.s)
                    .or_default()
                    .push(Keyed::of(literal, vocab));
            }
        }

        // Steps 2 and 3 — every geometry node is an entry in its own right, and
        // every default-geometry subject inherits its geometries.
        let mut by_subject = by_node.clone();
        for term in [GeoTerm::HasDefaultGeometry, GeoTerm::DefaultGeometry] {
            let iri = TermValue::iri(vocab.term(term));
            let Some(predicate) = dataset.term_id_by_value(&iri) else {
                continue;
            };
            for quad in dataset.quads_for_pattern(None, Some(predicate), None, graph) {
                let Some(inherited) = by_node.get(&quad.o).cloned() else {
                    continue;
                };
                by_subject.entry(quad.s).or_default().extend(inherited);
            }
        }

        let entries = finish_entries(dataset, by_subject);
        let asserted = collect_asserted(dataset, vocab, graph);
        let source_fingerprint = fingerprint(config, &entries, &asserted);
        Ok(Self {
            config: config.clone(),
            entries,
            asserted,
            source_fingerprint,
        })
    }

    /// The configuration this index was built under.
    #[must_use]
    pub const fn config(&self) -> &GeoIndexConfig {
        &self.config
    }

    /// Every indexed spatial object, sorted, with its parsed geometries.
    #[must_use]
    pub fn entries(&self) -> &[GeoEntry] {
        &self.entries
    }

    /// The number of indexed spatial objects.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether this index holds no spatial objects.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The asserted triples of `relation` found in the dataset, sorted and
    /// deduplicated.
    ///
    /// These are the rows the entailment rule contributes over and above the
    /// computed ones: `:-` is an entailment, so `ex:a geo:sfWithin ex:b` written
    /// in the data still matches whether or not the geometries satisfy
    /// `sfWithin`, and whether or not either side carries a geometry at all.
    #[must_use]
    pub fn asserted(&self, relation: SpatialRelation) -> &[(TermValue, TermValue)] {
        &self.asserted[relation_position(relation)]
    }

    /// A digest of the source data this index was built from.
    ///
    /// A hand-rolled FNV-1a over the configuration, the sorted entries (each
    /// subject and each of its geometries in canonical WKT), and the sorted
    /// asserted pairs. FNV-1a rather than a hasher whose output is a function of
    /// its own version, because the value is compared against one computed by a
    /// different run of this code; see [`verify_binding`] for what the comparison
    /// is for.
    #[must_use]
    pub const fn source_fingerprint(&self) -> u64 {
        self.source_fingerprint
    }

    /// The entry for `subject`, by binary search over the sorted entries.
    fn entry_of(&self, subject: &TermValue) -> Option<&GeoEntry> {
        self.entries
            .binary_search_by(|entry| entry.subject.cmp(subject))
            .ok()
            .map(|at| &self.entries[at])
    }

    /// The candidate entries for one side of an invocation: the single entry a
    /// bound argument names, or every entry when the position is free.
    ///
    /// This restriction is what keeps a bound-subject call from scanning the
    /// whole table, and it is *also* what keeps the coordinate-reference-system
    /// check scoped: a both-bound call over a mixed-system index examines exactly
    /// the one pair it was asked about.
    fn candidates(&self, bound: Option<&TermValue>) -> &[GeoEntry] {
        match bound {
            Some(subject) => self.entry_of(subject).map_or(&[][..], slice::from_ref),
            None => &self.entries,
        }
    }
}

/// A geometry paired with the key that orders it.
///
/// The key is carried beside the geometry rather than recomputed at every
/// comparison, and it is what makes "sorted" a well-defined statement about a
/// type that has no [`Ord`]. Two ingestion orders therefore produce the identical
/// geometry sequence for a subject. The rendering includes the coordinate
/// reference system, so two geometries that differ only in their system are two
/// distinct keys rather than one.
///
/// # The key must be LOSSLESS, and the WKT rendering alone is not
///
/// The key is used for two things — sorting, and [`canonical`]'s `dedup_by` —
/// and the second makes losslessness a soundness requirement rather than a
/// nicety. `wkt::write` rounds: [`crate::wkt::write`] renders each ordinate at a
/// fixed scale and "a scale too small rounds; it never fails". So two *distinct*
/// geometries that agree to `coordinate_scale` fraction digits render alike, and
/// a dedup on the rendering alone would delete one of them. That is a candidate
/// dropped from a bag the caller then reads as complete — a pairing that would
/// have satisfied the relation simply never gets tried — and it would be steered
/// by `coordinate_scale`, a knob documented as controlling *emitted* text.
///
/// It is also what would make the sort ingestion-order-dependent: a stable sort
/// leaves colliding keys in insertion order, so which geometry survived the
/// dedup would be a function of dataset intern order, contradicting
/// [`GeoIndex::from_dataset`]'s guarantee that no ingestion order reaches a
/// result.
///
/// So the key is the rendering *followed by* every ordinate as an exact
/// numerator/denominator pair. The rendering pins the coordinate reference
/// system, the geometry kind, the nesting and the empties exactly; the suffix
/// pins the coordinate values exactly, because [`Rat`](crate::exact::Rat) keeps
/// one canonical reduced representation per value. Equal keys therefore mean
/// equal geometries, which is exactly the premise `dedup_by` needs. The
/// rendering stays *first* so that the resulting order is still the readable
/// WKT order; the suffix only decides collisions.
#[derive(Clone, Debug)]
struct Keyed {
    /// The canonical rendering, CRS prefix included, followed by the exact
    /// ordinate suffix that makes the key lossless.
    key: String,
    /// The geometry itself.
    literal: GeometryLiteral,
}

impl Keyed {
    /// Render and pair `literal`.
    fn of(literal: GeometryLiteral, vocab: &GeoVocab) -> Self {
        let mut key = wkt::write(&literal, vocab.coordinate_scale());
        push_exact_ordinates(&mut key, &literal);
        Self { key, literal }
    }
}

/// Append every ordinate of `literal`, in traversal order, as an exact
/// `numerator/denominator` pair.
///
/// This is what turns a *rendered* geometry string — which rounds — into a
/// lossless identity for that geometry. [`Rat`](crate::exact::Rat) keeps one
/// canonical reduced representation per value, so equal numerator/denominator
/// sequences mean equal coordinates, exactly.
///
/// The separator characters cannot occur in a rendered integer, so no two
/// distinct ordinate sequences can produce the same suffix. An absent `z` or `m`
/// is written as `_`, distinguishing "no elevation" from any value.
fn push_exact_ordinates(key: &mut String, literal: &GeometryLiteral) {
    // The unit separator keeps the exact suffix out of the rendering's alphabet,
    // so the rendering remains the primary sort key.
    key.push('\u{1}');
    for coord in literal.geometry().coords() {
        for ordinate in [Some(coord.x()), Some(coord.y()), coord.z(), coord.m()] {
            match ordinate {
                Some(value) => {
                    key.push('|');
                    key.push_str(&value.numerator().to_string());
                    key.push('/');
                    key.push_str(&value.denominator().to_string());
                }
                None => key.push_str("|_"),
            }
        }
    }
}

/// The lossless identity of one geometry literal, independent of any vocabulary
/// setting.
///
/// The structural half is rendered at scale zero so that no vocabulary knob can
/// change the digest of an index whose rows did not change; the exact half then
/// restores every ordinate value, so the identity is complete rather than a
/// rounding of one.
fn exact_identity(literal: &GeometryLiteral) -> String {
    let mut key = wkt::write(literal, 0);
    push_exact_ordinates(&mut key, literal);
    key
}

/// The five GeoSPARQL serialization datatypes, resolved once per build.
#[derive(Clone, Debug)]
struct Datatypes {
    /// `geo:wktLiteral`.
    wkt: String,
    /// `geo:geoJSONLiteral`.
    geojson: String,
    /// The three serializations this crate does not implement, each with the
    /// local name its refusal reports.
    unimplemented: [(String, &'static str); 3],
}

impl Datatypes {
    /// Resolve the datatype IRIs from `vocab`.
    fn of(vocab: &GeoVocab) -> Self {
        Self {
            wkt: vocab.term(GeoTerm::WktLiteral),
            geojson: vocab.term(GeoTerm::GeoJsonLiteral),
            unimplemented: [
                (
                    vocab.term(GeoTerm::GmlLiteral),
                    GeoTerm::GmlLiteral.local_name(),
                ),
                (
                    vocab.term(GeoTerm::KmlLiteral),
                    GeoTerm::KmlLiteral.local_name(),
                ),
                (
                    vocab.term(GeoTerm::DggsLiteral),
                    GeoTerm::DggsLiteral.local_name(),
                ),
            ],
        }
    }
}

/// Parse the object of a configured serialization property, dispatching on its
/// datatype.
///
/// # Errors
///
/// See [`GeoIndex::from_dataset`]'s error list: this is where each of those three
/// refusals is raised.
fn parse_serialization(
    object: &TermValue,
    property: &TermValue,
    datatypes: &Datatypes,
    vocab: &GeoVocab,
) -> Result<GeometryLiteral, GeoError> {
    let TermValue::Literal {
        lexical_form,
        datatype,
        ..
    } = object
    else {
        return Err(GeoError::literal(format!(
            "the object of the configured serialization property {property:?} is {object:?}, \
             which is not a literal; only a literal can carry a geometry serialization, so this \
             row names no geometry and passing over it would drop an answer the configuration \
             asked for"
        )));
    };
    if datatype == &datatypes.wkt {
        return wkt::parse(lexical_form, vocab.default_wkt_crs());
    }
    if datatype == &datatypes.geojson {
        return geojson::parse(lexical_form, vocab.geojson_crs());
    }
    for (iri, local_name) in &datatypes.unimplemented {
        if datatype == iri {
            return Err(GeoError::unsupported(format!(
                "a geometry serialized as geo:{local_name} (<{datatype}>) reached the Query \
                 Rewrite index through the configured property {property:?}; purrdf-geo reads \
                 geo:wktLiteral and geo:geoJSONLiteral only. The configuration put this property \
                 in the conformance class, so this is refused by name rather than skipped — a \
                 skipped serialization is a missing answer with no symptom"
            )));
        }
    }
    Err(GeoError::literal(format!(
        "the object of the configured serialization property {property:?} carries datatype \
         <{datatype}>, which is none of the five GeoSPARQL serialization datatypes; a geometry is \
         read according to its datatype, and there is nothing here to read it as"
    )))
}

/// Resolve the caller's [`GraphSelector`] against the dataset in hand.
///
/// # Errors
///
/// [`GeoError::Config`] when a named graph is not interned in `dataset` at all.
fn resolve_graph<D: DatasetView>(
    dataset: &D,
    selector: &GraphSelector,
) -> Result<GraphMatch<D::Id>, GeoError> {
    Ok(match selector {
        GraphSelector::Any => GraphMatch::Any,
        GraphSelector::Default => GraphMatch::Default,
        GraphSelector::Named(name) => {
            let Some(id) = dataset.term_id_by_value(name) else {
                return Err(GeoError::config(format!(
                    "the configured named graph {name:?} is not present in the dataset; a \
                     configuration pointing at a graph that is not there is a wiring mistake, and \
                     answering it with an empty index would hide that"
                )));
            };
            GraphMatch::Named(id)
        }
    })
}

/// Resolve a dataset-local id to its dataset-independent [`TermValue`].
///
/// Recurses through a literal's datatype and a triple term's components; the
/// recursion carries no depth cap, for the reason `Digest::term` gives.
///
/// A literal whose datatype does not resolve to an IRI cannot occur in a
/// well-formed dataset — the IR expands the datatype at intern time — so that
/// branch falls back to the empty IRI rather than raising a refusal for a state
/// no [`DatasetView`] implementor in this workspace can produce.
fn resolve_value<D: DatasetView>(dataset: &D, id: D::Id) -> TermValue {
    match dataset.resolve(id) {
        TermRef::Iri(iri) => TermValue::iri(iri),
        TermRef::Blank { label, scope } => TermValue::Blank {
            label: label.to_owned(),
            scope,
        },
        TermRef::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => TermValue::Literal {
            lexical_form: lexical.to_owned(),
            datatype: match dataset.resolve(datatype) {
                TermRef::Iri(iri) => iri.to_owned(),
                TermRef::Blank { .. } | TermRef::Literal { .. } | TermRef::Triple { .. } => {
                    String::new()
                }
            },
            language: language.map(str::to_owned),
            direction,
        },
        TermRef::Triple { s, p, o } => TermValue::Triple {
            s: Box::new(resolve_value(dataset, s)),
            p: Box::new(resolve_value(dataset, p)),
            o: Box::new(resolve_value(dataset, o)),
        },
    }
}

/// Turn the id-keyed accumulation into the sorted, deduplicated entry table.
///
/// Sorting happens in **[`TermValue`] space**, not id space: an id order is
/// dataset-local, so two datasets holding the same triples in different intern
/// orders would emit rows in different orders if the sort read ids. Subjects that
/// resolve to the same value are coalesced, so the table is a strict total order
/// and `GeoIndex::entry_of`'s binary search is exact.
fn finish_entries<D: DatasetView>(
    dataset: &D,
    by_subject: BTreeMap<D::Id, Vec<Keyed>>,
) -> Vec<GeoEntry> {
    let mut rows: Vec<(TermValue, Vec<Keyed>)> = by_subject
        .into_iter()
        .map(|(id, geometries)| (resolve_value(dataset, id), geometries))
        .collect();
    rows.sort_by(|left, right| left.0.cmp(&right.0));

    let mut entries: Vec<GeoEntry> = Vec::with_capacity(rows.len());
    let mut current: Option<(TermValue, Vec<Keyed>)> = None;
    for (subject, geometries) in rows {
        match current.take() {
            // Two ids resolving to one value cannot happen in an interned
            // dataset, but coalescing costs one comparison and keeps the
            // binary-search precondition true by construction rather than by
            // assumption.
            Some((held, mut collected)) if held == subject => {
                collected.extend(geometries);
                current = Some((held, collected));
            }
            Some(previous) => {
                push_entry(&mut entries, previous);
                current = Some((subject, geometries));
            }
            None => current = Some((subject, geometries)),
        }
    }
    if let Some(last) = current {
        push_entry(&mut entries, last);
    }
    entries
}

/// Canonicalize one subject's geometries and push it, unless it has none.
///
/// A spatial object with no geometry satisfies no relation's existential over
/// pairings, so it can contribute no computed row; keeping it would only inflate
/// the declared row bound.
fn push_entry(entries: &mut Vec<GeoEntry>, row: (TermValue, Vec<Keyed>)) {
    let (subject, geometries) = row;
    let geometries = canonical(geometries);
    if geometries.is_empty() {
        return;
    }
    entries.push(GeoEntry {
        subject,
        geometries,
    });
}

/// Sort by [`Keyed::key`] and drop exact duplicates.
///
/// Dropping duplicates is sound rather than merely tidy, but only because the key
/// is lossless (see [`Keyed`]): two geometries with the identical key are the
/// identical geometry in the identical coordinate reference system, so they
/// decide every relation identically and removing one cannot change whether the
/// existential over pairings holds. Were the key the rounded WKT rendering alone,
/// this `dedup_by` would delete geometries that merely *look* alike at the
/// configured scale, and the relation would answer a short bag as though it were
/// complete.
///
/// The sort is total for the same reason, so it does not fall back on the input
/// order for any pair — which is what lets [`GeoIndex::from_dataset`] promise
/// that no ingestion order reaches a result.
fn canonical(mut geometries: Vec<Keyed>) -> Vec<GeometryLiteral> {
    geometries.sort_by(|left, right| left.key.cmp(&right.key));
    geometries.dedup_by(|left, right| left.key == right.key);
    geometries.into_iter().map(|keyed| keyed.literal).collect()
}

/// This relation's index into `GeoIndex::asserted`.
///
/// [`SpatialRelation::ALL`] lists every variant, so the search always succeeds;
/// the fallback exists only because `position` returns an `Option` and a panic
/// here would be a worse answer than the first relation's slot.
fn relation_position(relation: SpatialRelation) -> usize {
    SpatialRelation::ALL
        .iter()
        .position(|candidate| *candidate == relation)
        .unwrap_or_default()
}

/// Collect the asserted `(subject, object)` pairs of every relation.
///
/// Walked over [`SpatialRelation::ALL`] in its fixed order — never a map's
/// iteration order — and each vector is sorted and deduplicated, because a
/// duplicate asserted triple in the data is still one entailed triple and BGP
/// matching over a set of triples yields one solution.
fn collect_asserted<D: DatasetView>(
    dataset: &D,
    vocab: &GeoVocab,
    graph: GraphMatch<D::Id>,
) -> Vec<Vec<(TermValue, TermValue)>> {
    let mut out: Vec<Vec<(TermValue, TermValue)>> = Vec::with_capacity(SpatialRelation::ALL.len());
    for relation in SpatialRelation::ALL {
        let iri = TermValue::iri(format!(
            "{}{}",
            vocab.core_namespace(),
            relation.local_name()
        ));
        let mut pairs: Vec<(TermValue, TermValue)> = Vec::new();
        if let Some(predicate) = dataset.term_id_by_value(&iri) {
            for quad in dataset.quads_for_pattern(None, Some(predicate), None, graph) {
                pairs.push((
                    resolve_value(dataset, quad.s),
                    resolve_value(dataset, quad.o),
                ));
            }
        }
        pairs.sort();
        pairs.dedup();
        out.push(pairs);
    }
    out
}

/// The source digest of a built index.
///
/// Every geometry contributes its [`exact_identity`]: the structure rendered at a
/// FIXED scale of zero, so that a vocabulary setting which does not change the
/// index's rows cannot change the digest, followed by every ordinate as an exact
/// rational. Both halves are needed. The fixed scale alone would round every
/// coordinate to the nearest integer, so `POINT(1.4 1.4)` and `POINT(1.2 1.2)`
/// would digest alike and [`GeoIndex::verify_binding`] would accept an index
/// built from a different dataset — the precise silent wrong answer it exists to
/// refuse.
fn fingerprint(
    config: &GeoIndexConfig,
    entries: &[GeoEntry],
    asserted: &[Vec<(TermValue, TermValue)>],
) -> u64 {
    let mut digest = Digest::new();
    config.absorb(&mut digest);
    digest.count(entries.len());
    for entry in entries {
        digest.term(&entry.subject);
        digest.count(entry.geometries.len());
        for geometry in &entry.geometries {
            digest.field(&exact_identity(geometry));
        }
    }
    for (at, pairs) in asserted.iter().enumerate() {
        digest.count(at);
        digest.count(pairs.len());
        for (subject, object) in pairs {
            digest.term(subject);
            digest.term(object);
        }
    }
    digest.finish()
}

/// Prove an index and a dataset are the pairing the caller intends.
///
/// # The channel this closes
///
/// [`PropertyFunction::open`] receives no dataset. The engine validates *registry*
/// identity when a plan is prepared, but nothing anywhere validates *dataset*
/// identity — so an index paired with the wrong dataset is a silent wrong answer
/// rather than a failure. The relation emits perfectly well-formed
/// spatial-object subjects; those subjects join back by basic graph pattern
/// against a dataset that never held them; zero rows come out; and no layer has
/// anything to report. A mismatch has to be found by asking, because it will
/// never announce itself.
///
/// This rebuilds the index over `dataset` under `config` and compares the two
/// [`source_fingerprint`](GeoIndex::source_fingerprint)s. Rebuilding, rather than
/// running a separate lighter digest, is deliberate: a second walk would be a
/// second implementation of the projection, and the two could drift into
/// disagreeing about what "the same dataset" means.
///
/// # When to call it
///
/// It is **O(dataset)** — it re-walks every configured serialization property and
/// re-parses every geometry. Run it once per `(index, dataset)` pairing, where
/// the host wires the registry. Never per query, and never per invocation.
///
/// # Errors
///
/// * [`GeoError::Config`] if `config` is not the configuration `index` was built
///   under. Projecting `dataset` under a different configuration would compare
///   two different questions, so the mismatch is reported rather than producing a
///   verdict that means nothing.
/// * Whatever [`GeoIndex::from_dataset`] raises over `dataset` — which is itself
///   a wrong-dataset symptom when the index built cleanly.
/// * [`GeoError::Config`] if the digests differ, naming both so a host can see
///   which pairing it made.
pub fn verify_binding<D: DatasetView>(
    index: &GeoIndex,
    dataset: &D,
    vocab: &GeoVocab,
    config: &GeoIndexConfig,
) -> Result<(), GeoError> {
    if config != index.config() {
        return Err(GeoError::config(
            "the configuration supplied to verify_binding is not the one this index was built \
             under; the two would project different rows, so the comparison would answer a \
             different question than the one asked",
        ));
    }
    let rebuilt = GeoIndex::from_dataset(dataset, vocab, config)?;
    let expected = index.source_fingerprint();
    let actual = rebuilt.source_fingerprint();
    if expected == actual {
        return Ok(());
    }
    Err(GeoError::config(format!(
        "this GeoSPARQL index was built over a different dataset: its source digest is \
         {expected:#018x} and the supplied dataset digests to {actual:#018x}. Rebuild the index \
         from the dataset the query runs against, or pair the query with the dataset the index \
         was built from — an index joined to the wrong dataset returns no rows and reports nothing"
    )))
}

// ---------------------------------------------------------------------------
// The relation
// ---------------------------------------------------------------------------

/// The row maxima `GeoRelation::rows_per_invocation` is computed from, measured
/// once at relation construction.
///
/// Measured, never guessed: the seam holds this declaration to the same honesty
/// contract as a cardinality estimate, because a bound that understates reality
/// turns an admission decision into a wrong one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RowBounds {
    /// The bound when neither position is bound (`ff`).
    free: u64,
    /// The bound when exactly one position is bound (`bf` or `fb`).
    half: u64,
}

impl RowBounds {
    /// Measure `index` for `relation`.
    fn of(index: &GeoIndex, relation: SpatialRelation) -> Self {
        let entries = index.len() as u64;
        let asserted = index.asserted(relation).len() as u64;
        Self {
            // Every accumulation saturates, so no bound can wrap to a dishonest
            // zero on a very large index.
            free: entries.saturating_mul(entries).saturating_add(asserted),
            half: entries.saturating_add(asserted),
        }
    }
}

/// One GeoSPARQL spatial relation, reachable from predicate position through the
/// Query Rewrite extension.
///
/// A call reads
///
/// ```text
/// ?so1 <http://example.org/geo#sfWithin> ?so2 .
/// ```
///
/// over two flattened positions:
///
/// | pos | name | role | emitted term |
/// |---|---|---|---|
/// | 0 | `?so1` | the left spatial object | its subject verbatim — an IRI, a blank node, or an RDF 1.2 triple term |
/// | 1 | `?so2` | the right spatial object | likewise |
///
/// # It declares exactly one mode, the all-free one
///
/// [`PfArity::all_free_mode`] (code `"ff"`) subsumes every access pattern of this
/// arity, so every invocation is feasible. That is honest rather than optimistic:
/// the whole geometry table is in memory, so the relation genuinely can enumerate
/// either side from the other, or both from nothing. A bound position is applied
/// twice — pushed into the candidate selection so the scan is not quadratic when
/// it need not be, and re-checked as term equality by the cursor — and the
/// engine's own equality filter on bound positions then has nothing left to
/// remove.
///
/// # Emission order is `(?so1, ?so2)` ascending, and that is the contract
///
/// Rows are sorted lexicographically on the pair, in [`TermValue`]'s own total
/// order, and deduplicated. The engine preserves that order verbatim into the
/// query's answer, so it is part of this relation's public behaviour rather than
/// an implementation detail.
///
/// # Why the rows are deduplicated
///
/// A spatial object may carry several default geometries and several
/// serializations, and the four RIF branches of the rewrite rule overlap. But the
/// entailed triple `(a, <rel>, b)` either holds or it does not, and BGP matching
/// over a *set* of triples yields exactly one solution. The engine does not
/// deduplicate property-function rows, so emitting a pair twice would produce a
/// duplicate solution that no query text explains.
///
/// # It is [`Volatility::Stable`]
///
/// The index is frozen and every geometric decision beneath it is exact integer
/// arithmetic over exact rationals, so an invocation's rows are a pure function
/// of its arguments for the lifetime of a query — the same answer on the main
/// thread and on a fork-join worker, and the same answer on
/// `wasm32-unknown-unknown` as on a native build. That is exactly what the stable
/// class asserts, so the relation may run across workers.
#[derive(Clone, Debug)]
pub struct GeoRelation {
    /// The projection every invocation is answered from.
    index: Arc<GeoIndex>,
    /// Which of the twenty-four relations this is.
    relation: SpatialRelation,
    /// The single declared mode, materialized once so [`PropertyFunction::modes`]
    /// can hand out a slice.
    modes: [BindingPattern; 1],
    /// The row maxima, measured once at construction.
    bounds: RowBounds,
}

impl GeoRelation {
    /// The Query Rewrite relation for `relation` over `index`.
    ///
    /// The row bounds are measured here, once, rather than recomputed per
    /// invocation: they are a function of the index, and the index is frozen.
    #[must_use]
    pub fn new(index: Arc<GeoIndex>, relation: SpatialRelation) -> Self {
        let bounds = RowBounds::of(&index, relation);
        Self {
            index,
            relation,
            modes: [BindingPattern::from_code(ALL_FREE_MODE)],
            bounds,
        }
    }

    /// Which of the twenty-four relations this is.
    #[must_use]
    pub const fn relation(&self) -> SpatialRelation {
        self.relation
    }

    /// The projection this relation answers from.
    #[must_use]
    pub fn index(&self) -> &GeoIndex {
        &self.index
    }

    /// Whether any pairing of `left`'s geometries with `right`'s satisfies this
    /// relation — the existential in the RIF rule's `And`.
    ///
    /// # Errors
    ///
    /// [`GeoError::Domain`] when a pairing crosses two coordinate reference
    /// systems **and no same-system pairing satisfied the relation**. This crate
    /// reprojects nothing, so a cross-system pairing cannot be decided; the
    /// question is only whether that undecidable pairing has to poison the answer.
    ///
    /// It does not, when some other pairing already answered `true`. The rule
    /// this implements is an existential over pairings — `∃ s1,s2 :
    /// relation(s1,s2)` — so one same-system witness entails the row no matter
    /// what the remaining pairings would have said. Refusing anyway would be an
    /// over-refusal of a perfectly ordinary dataset: carrying one feature in two
    /// coordinate reference systems (a geographic one plus a projected one) is
    /// normal GeoSPARQL, and under a blanket check the *presence of a second
    /// serialization* would break a query that works without it.
    ///
    /// A `false`, by contrast, is only returned when every pairing was actually
    /// evaluated. If any pairing was skipped as undecidable, the honest answer is
    /// the refusal rather than a `false` that cannot be distinguished from "the
    /// geometries genuinely do not relate".
    fn holds(&self, left: &GeoEntry, right: &GeoEntry) -> Result<bool, GeoError> {
        let mut undecidable: Option<GeoError> = None;
        for a in &left.geometries {
            for b in &right.geometries {
                if let Err(error) = a.require_same_crs(b) {
                    undecidable.get_or_insert(error);
                    continue;
                }
                let matrix = relate(a.geometry(), b.geometry());
                if self.relation.holds(
                    &matrix,
                    topological_dimension(a.geometry()),
                    topological_dimension(b.geometry()),
                ) {
                    return Ok(true);
                }
            }
        }
        // No witness. Only now does an undecidable pairing matter: without one,
        // `false` would be indistinguishable from a pairing that was never tried.
        undecidable.map_or(Ok(false), Err)
    }

    /// Materialize this invocation's rows: sorted, deduplicated, and restricted
    /// by whichever positions the call bound.
    ///
    /// The engine's row ceiling is deliberately *not* pushed in here. A relation
    /// that ignores the licence is correct and merely less efficient, and cutting
    /// the materialization would mean the sort and the deduplication ran over a
    /// prefix rather than over the answer — which is exactly how a short bag gets
    /// offered as a complete one. The cursor spends the licence instead, on the
    /// rows it actually emits.
    ///
    /// # Errors
    ///
    /// [`GeoError::Domain`] from [`Self::holds`].
    fn rows(&self, args: &PfArgs<'_>) -> Result<Vec<PfRow>, GeoError> {
        let subject = args.get(SUBJECT);
        let object = args.get(OBJECT);
        let lefts = self.index.candidates(subject);
        let rights = self.index.candidates(object);

        // Reserve for the CROSS PRODUCT's shape, not its size. The product is
        // quadratic and a `(TermValue, TermValue)` is not small, so reserving it
        // up front would commit gigabytes for a large all-free invocation —
        // before a single `relate` has run, and even when the answer is a handful
        // of rows or the caller wanted `LIMIT 1`. The satisfying pairs are a
        // subset of unknown size, so the honest starting point is the larger side
        // and then growth; that keeps the common small case allocation-free
        // without betting the process on the worst case.
        let mut pairs: Vec<(TermValue, TermValue)> =
            Vec::with_capacity(lefts.len().max(rights.len()));
        for left in lefts {
            for right in rights {
                if self.holds(left, right)? {
                    pairs.push((left.subject.clone(), right.subject.clone()));
                }
            }
        }

        // The entailment half: an asserted triple matches whether or not the
        // geometries satisfy the relation, and neither side need be an index
        // entry at all.
        for (asserted_subject, asserted_object) in self.index.asserted(self.relation) {
            if subject.is_some_and(|bound| bound != asserted_subject)
                || object.is_some_and(|bound| bound != asserted_object)
            {
                continue;
            }
            pairs.push((asserted_subject.clone(), asserted_object.clone()));
        }

        pairs.sort();
        pairs.dedup();
        Ok(pairs
            .into_iter()
            .map(|(left, right)| vec![left, right])
            .collect())
    }
}

impl PropertyFunction for GeoRelation {
    fn volatility(&self) -> Volatility {
        // A frozen index plus exact integer arithmetic makes the answer a pure
        // function of the arguments for the lifetime of a query, so this may run
        // across fork-join workers. See the type's docs.
        Volatility::Stable
    }

    fn arity(&self) -> PfArity {
        REWRITE_ARITY
    }

    fn modes(&self) -> &[BindingPattern] {
        &self.modes
    }

    /// The declared row bound, as a real function of the mode, measured from the
    /// index at construction.
    ///
    /// With `n` the number of indexed spatial objects and `a` the number of
    /// asserted pairs for **this** relation:
    ///
    /// | bound positions | declared bound | why |
    /// |---|---|---|
    /// | neither (`ff`) | `n*n + a` | every ordered pair of entries may qualify, plus every asserted pair |
    /// | position 0 only (`bf`) | `n + a` | one entry on the left against every entry on the right, plus the asserted pairs |
    /// | position 1 only (`fb`) | `n + a` | the mirror image |
    /// | both (`bb`) | `1` | the row *is* the pair, and the rows are deduplicated, so it is emitted at most once |
    ///
    /// Every accumulation uses `saturating_mul`/`saturating_add`, so a very large
    /// index can never wrap the product to a dishonest zero. This is an
    /// **admission input**: the planner both orders this call against its
    /// neighbours and admits it against a row ceiling using this number, so a
    /// bound that understates reality turns an admission decision into a wrong
    /// one.
    fn rows_per_invocation(&self, mode: BindingPattern) -> u64 {
        match (mode.is_bound(SUBJECT), mode.is_bound(OBJECT)) {
            (true, true) => 1,
            (false, false) => self.bounds.free,
            _ => self.bounds.half,
        }
    }

    /// Begin one Query Rewrite invocation.
    ///
    /// # Errors
    ///
    /// * [`EvalError::Function`] if the call site's argument vectors do not match
    ///   the declared arity. The engine checks this before any host code runs;
    ///   repeating it here means a direct caller gets the same answer rather than
    ///   an out-of-range read.
    /// * The evaluator's rendering of [`GeoError::Domain`] when two geometries
    ///   that must be compared are in different coordinate reference systems.
    ///   This crate reprojects nothing, so the pair is refused rather than
    ///   skipped: a skipped pair is a missing answer, and a missing answer from a
    ///   spatial relation is indistinguishable from an honest one.
    fn open(
        &self,
        args: &PfArgs<'_>,
        ceiling: Option<u64>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        let supplied = args.arity();
        if supplied != REWRITE_ARITY {
            return Err(EvalError::function(format!(
                "the GeoSPARQL Query Rewrite relation geo:{} expects {REWRITE_ARITY} argument(s), \
                 got {supplied}",
                self.relation.local_name()
            )));
        }
        let rows = self.rows(args)?;
        Ok(Box::new(GeoCursor {
            rows,
            at: 0,
            bound: args.flattened().map(<Option<&TermValue>>::cloned).collect(),
            remaining: ceiling,
        }))
    }
}

/// The cursor `GeoRelation::open` returns: an indexed walk over the materialized,
/// sorted, deduplicated row vector.
///
/// Two properties make it sound under the seam's ceiling contract, and both are
/// load-bearing.
///
/// * **It filters on every bound position itself**, rather than trusting the
///   candidate pre-selection. The two agree today, and the check is one term
///   comparison per row; but a relation is entitled to generate candidates and
///   let the engine's equality filter cut them, and one that also *spends a
///   ceiling* on candidates it can itself see are doomed hands back fewer usable
///   rows than the engine asked for. The only way to be sure that never happens
///   is for the code that spends the licence to be the code that applies the
///   filter.
/// * **It decrements the licence only on rows it actually emits.** A row this
///   cursor skips disagrees with a bound position and would have been dropped by
///   the engine anyway, so counting it would spend the licence on rows the engine
///   was never going to keep — a stop at `k` would then yield fewer than `k`
///   usable rows, and the engine reads a short bag as an exhausted one.
///
/// It stops *producing*; it never reports an error or a short-but-different bag.
/// The rows already emitted are the first rows of the full sorted answer, in the
/// same order, which is exactly what the licence was granted against.
#[derive(Debug)]
struct GeoCursor {
    /// The materialized rows, in `(?so1, ?so2)` ascending order.
    rows: Vec<PfRow>,
    /// How far into `rows` the cursor has read.
    at: usize,
    /// The invocation's bound values by flattened position (`None` = free).
    bound: Vec<Option<TermValue>>,
    /// The rows this invocation may still emit under the engine's licence, or
    /// `None` when it was given no ceiling.
    remaining: Option<u64>,
}

impl PfCursor for GeoCursor {
    fn next(&mut self) -> Result<Option<PfRow>, EvalError> {
        if self.remaining == Some(0) {
            return Ok(None);
        }
        while let Some(row) = self.rows.get(self.at) {
            self.at += 1;
            let agrees = self
                .bound
                .iter()
                .zip(row.iter())
                .all(|(bound, value)| bound.as_ref().is_none_or(|bound| bound == value));
            if agrees {
                if let Some(remaining) = self.remaining.as_mut() {
                    *remaining = remaining.saturating_sub(1);
                }
                return Ok(Some(row.clone()));
            }
        }
        Ok(None)
    }
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

/// Register a relation for every [`SpatialRelation`] in `families`, under the
/// caller's `geo:` namespace.
///
/// The walk is over [`SpatialRelation::ALL`] in its fixed order — never a map's
/// iteration order — so the registry a host ends up with is a pure function of
/// `families` rather than of anything's hashing. Each relation is registered
/// under `format!("{}{}", vocab.core_namespace(), relation.local_name())`, which
/// is the `geo:` property IRI the standard's Tables 9, 10 and 11 give it.
///
/// # Calling this twice with overlapping families is a host misconfiguration
///
/// [`PropertyFunctionRegistry::register`] **panics** on a duplicate IRI, and
/// deliberately so: a shadowed relation silently changes which rows a graph
/// pattern produces, both spellings of the call are identical, and the only
/// observable difference is the answer. Two calls whose `families` overlap
/// therefore abort rather than letting the second registration win.
///
/// # Errors
///
/// [`GeoError::Config`] if `families` is empty. Registering nothing while
/// returning `Ok` looks like success, and the symptom arrives much later as a
/// query whose `geo:sfWithin` was parsed as an ordinary triple pattern and
/// matched nothing.
pub fn register(
    registry: &mut PropertyFunctionRegistry,
    vocab: &GeoVocab,
    index: &Arc<GeoIndex>,
    families: &[RelationFamily],
) -> Result<(), GeoError> {
    if families.is_empty() {
        return Err(GeoError::config(
            "no relation families supplied to register; registering nothing and returning success \
             would surface much later as a query whose geo: relation was parsed as an ordinary \
             triple pattern and matched nothing",
        ));
    }
    for relation in SpatialRelation::ALL {
        if !families.contains(&relation.family()) {
            continue;
        }
        let iri = format!("{}{}", vocab.core_namespace(), relation.local_name());
        registry.register(iri, Arc::new(GeoRelation::new(Arc::clone(index), relation)));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use purrdf_core::binding_pattern::BindingPattern;
    use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral, TermValue};
    use purrdf_sparql_eval::{
        EvalError, PfArgs, PfRow, PropertyFunction, PropertyFunctionRegistry, Volatility,
    };

    use super::{
        GeoEntry, GeoIndex, GeoIndexConfig, GeoRelation, GraphSelector, register, verify_binding,
    };
    use crate::error::GeoError;
    use crate::geom::Crs;
    use crate::relations::{RelationFamily, SpatialRelation};
    use crate::vocab::{GeoVocab, GeoVocabBuilder};

    // ---- fixtures --------------------------------------------------------

    /// The caller's `geo:` namespace. A fixture, never a default.
    const GEO: &str = "http://example.org/geo#";
    /// The caller's `geof:` namespace.
    const GEOF: &str = "http://example.org/geof/";
    /// The coordinate reference system every fixture geometry is in.
    const CRS: &str = "http://example.org/crs/planar";
    /// A second system, for the mixed-CRS refusal.
    const CRS_OTHER: &str = "http://example.org/crs/other";
    /// The named graph the graph-selector fixtures write into.
    const GRAPH: &str = "http://example.org/g1";

    /// A four-by-four square at the origin.
    const SQUARE: &str = "POLYGON((0 0,4 0,4 4,0 4,0 0))";
    /// A point strictly inside [`SQUARE`].
    const INSIDE: &str = "POINT(1 1)";
    /// A second point strictly inside [`SQUARE`], distinct from [`INSIDE`].
    const INSIDE_TOO: &str = "POINT(2 2)";
    /// A point well outside [`SQUARE`].
    const OUTSIDE: &str = "POINT(9 9)";

    fn crs(iri: &str) -> Crs {
        Crs::new(iri).expect("a non-empty IRI")
    }

    fn vocab() -> GeoVocab {
        GeoVocabBuilder::new(GEO, GEOF, crs(CRS), crs(CRS))
            .expect("non-empty namespaces")
            .build()
    }

    /// The full IRI of an `example.org` local name.
    fn iri(local: &str) -> TermValue {
        TermValue::iri(format!("http://example.org/{local}"))
    }

    /// The full IRI of a `geo:` local name, in the fixture namespace.
    fn geo(local: &str) -> String {
        format!("{GEO}{local}")
    }

    /// The object of a fixture triple.
    #[derive(Clone, Debug)]
    enum Obj {
        /// An IRI object, spelled as an `example.org` local name.
        Node(String),
        /// A literal object: lexical form and datatype IRI.
        Lit(String, String),
    }

    /// One fixture triple.
    #[derive(Clone, Debug)]
    struct Row {
        /// The subject's `example.org` local name.
        subject: String,
        /// The predicate's full IRI.
        predicate: String,
        /// The object.
        object: Obj,
    }

    fn row(subject: &str, predicate: String, object: Obj) -> Row {
        Row {
            subject: subject.to_owned(),
            predicate,
            object,
        }
    }

    fn node(local: &str) -> Obj {
        Obj::Node(local.to_owned())
    }

    /// A `geo:wktLiteral` object.
    fn wkt(lexical: &str) -> Obj {
        Obj::Lit(lexical.to_owned(), geo("wktLiteral"))
    }

    /// Build a dataset from `rows`, interning in the order given.
    fn dataset_of(rows: &[Row]) -> Arc<RdfDataset> {
        dataset_in(rows, None)
    }

    /// Build a dataset from `rows`, placing every quad in `graph`.
    fn dataset_in(rows: &[Row], graph: Option<&str>) -> Arc<RdfDataset> {
        let mut builder = RdfDatasetBuilder::new();
        let graph = graph.map(|name| builder.intern_iri(name));
        for entry in rows {
            let s = builder.intern_iri(&format!("http://example.org/{}", entry.subject));
            let p = builder.intern_iri(&entry.predicate);
            let o = match &entry.object {
                Obj::Node(local) => builder.intern_iri(&format!("http://example.org/{local}")),
                Obj::Lit(lexical, datatype) => {
                    builder.intern_literal(RdfLiteral::typed(lexical, datatype))
                }
            };
            builder.push_quad(s, p, o, graph);
        }
        builder.freeze().expect("a well-formed fixture")
    }

    /// The configuration naming `geo:asWKT` alone, over every graph.
    fn config() -> GeoIndexConfig {
        GeoIndexConfig::new(vec![TermValue::iri(geo("asWKT"))], GraphSelector::Any)
            .expect("one IRI is a valid serialization set")
    }

    /// The four-branch fixture: two features, each with a default geometry, and
    /// both geometries also standing alone as spatial objects.
    ///
    /// `ex:fp` is a feature over the point `ex:gp`; `ex:fq` is a feature over the
    /// square `ex:gq`. The point is strictly inside the square, so each of the
    /// four RIF branches of `sfWithin` has exactly one witness.
    fn four_branch_rows() -> Vec<Row> {
        vec![
            row("fp", geo("hasDefaultGeometry"), node("gp")),
            row("gp", geo("asWKT"), wkt(INSIDE)),
            row("fq", geo("hasDefaultGeometry"), node("gq")),
            row("gq", geo("asWKT"), wkt(SQUARE)),
        ]
    }

    fn index_of(rows: &[Row]) -> GeoIndex {
        GeoIndex::from_dataset(&*dataset_of(rows), &vocab(), &config()).expect("a clean fixture")
    }

    fn relation_of(rows: &[Row], relation: SpatialRelation) -> GeoRelation {
        GeoRelation::new(Arc::new(index_of(rows)), relation)
    }

    /// Open `relation` with the given per-position bindings and drain it.
    fn invoke(
        relation: &GeoRelation,
        bound: &[Option<TermValue>],
        ceiling: Option<u64>,
    ) -> Result<Vec<PfRow>, EvalError> {
        let refs: Vec<Option<&TermValue>> = bound.iter().map(Option::as_ref).collect();
        let (subject, object) = refs.split_at(1);
        let args = PfArgs::new(subject, object);
        let mut cursor = relation.open(&args, ceiling)?;
        let mut out = Vec::new();
        while let Some(emitted) = cursor.next()? {
            out.push(emitted);
        }
        Ok(out)
    }

    /// The all-free rows of `relation`, as `(local, local)` pairs for
    /// readability.
    fn pairs(relation: &GeoRelation) -> Vec<(String, String)> {
        invoke(relation, &[None, None], None)
            .expect("no refusal")
            .into_iter()
            .map(|emitted| (render(&emitted[0]), render(&emitted[1])))
            .collect()
    }

    /// A term's `example.org` local name.
    fn render(value: &TermValue) -> String {
        match value {
            TermValue::Iri(full) => full
                .strip_prefix("http://example.org/")
                .unwrap_or(full)
                .to_owned(),
            other => format!("{other:?}"),
        }
    }

    fn pair(left: &str, right: &str) -> (String, String) {
        (left.to_owned(), right.to_owned())
    }

    // ---- 1. the headline: all four RIF branches produce rows ---------------

    /// GeoSPARQL 1.1 Clause 13 defines each relation as the disjunction of four
    /// rules — feature/feature, feature/geometry, geometry/feature and
    /// geometry/geometry. Indexing only features produces a **short bag the
    /// engine reads as complete**, so this test names each branch and then
    /// asserts the exact, complete row list rather than a lower bound.
    #[test]
    fn all_four_rewrite_branches_produce_rows_and_the_bag_is_exactly_complete() {
        let relation = relation_of(&four_branch_rows(), SpatialRelation::SfWithin);
        assert_eq!(
            relation.index().len(),
            4,
            "a feature and a geometry are both geo:SpatialObjects, so all four are entries"
        );

        let rows = pairs(&relation);

        // The four branches, named.
        assert!(
            rows.contains(&pair("fp", "fq")),
            "branch 1, feature/feature: {rows:?}"
        );
        assert!(
            rows.contains(&pair("fp", "gq")),
            "branch 2, feature/geometry: {rows:?}"
        );
        assert!(
            rows.contains(&pair("gp", "fq")),
            "branch 3, geometry/feature: {rows:?}"
        );
        assert!(
            rows.contains(&pair("gp", "gq")),
            "branch 4, geometry/geometry: {rows:?}"
        );

        // And the whole bag, exactly. `sfWithin` is reflexive, so each object is
        // within itself and within its twin; the point is within both squares,
        // and no square is within a point.
        assert_eq!(
            rows,
            vec![
                pair("fp", "fp"),
                pair("fp", "fq"),
                pair("fp", "gp"),
                pair("fp", "gq"),
                pair("fq", "fq"),
                pair("fq", "gq"),
                pair("gp", "fp"),
                pair("gp", "fq"),
                pair("gp", "gp"),
                pair("gp", "gq"),
                pair("gq", "fq"),
                pair("gq", "gq"),
            ],
            "the exact bag, in the (?so1, ?so2) ascending order the relation contracts to"
        );
    }

    /// The legacy `geo:defaultGeometry` is an `owl:equivalentProperty` of
    /// `geo:hasDefaultGeometry` in the shipped ontology, so it must dereference
    /// identically. Accepting only the current spelling would silently drop
    /// every GeoSPARQL 1.0 feature in a dataset.
    #[test]
    fn the_legacy_default_geometry_alias_dereferences_exactly_as_the_current_one_does() {
        let current = index_of(&four_branch_rows());
        let legacy: Vec<Row> = four_branch_rows()
            .into_iter()
            .map(|mut entry| {
                if entry.predicate == geo("hasDefaultGeometry") {
                    entry.predicate = geo("defaultGeometry");
                }
                entry
            })
            .collect();
        let aliased = index_of(&legacy);

        assert_eq!(
            aliased.len(),
            current.len(),
            "the alias must index the same spatial objects"
        );
        assert_eq!(
            aliased.entries(),
            current.entries(),
            "and index them identically"
        );
        assert_eq!(
            aliased.source_fingerprint(),
            current.source_fingerprint(),
            "the two spellings project the identical index, so they digest identically"
        );
    }

    // ---- 2. the short-bag control ------------------------------------------

    /// A dataset with no `hasDefaultGeometry` at all still yields the
    /// geometry/geometry branch — the control that proves the count above turns
    /// on the dereferencing rather than on everything being indexed twice.
    #[test]
    fn bare_geometries_alone_still_produce_the_geometry_geometry_branch() {
        let rows = vec![
            row("gp", geo("asWKT"), wkt(INSIDE)),
            row("gq", geo("asWKT"), wkt(SQUARE)),
        ];
        let relation = relation_of(&rows, SpatialRelation::SfWithin);
        assert_eq!(
            pairs(&relation),
            vec![pair("gp", "gp"), pair("gp", "gq"), pair("gq", "gq")],
            "two bare geometries, three within-pairs, and nothing invented"
        );
    }

    // ---- 3. deduplication ---------------------------------------------------

    /// A spatial object may carry several default geometries, and the four RIF
    /// branches overlap. The entailed triple either holds or it does not, and
    /// BGP matching over a set of triples yields ONE solution — the engine does
    /// not deduplicate property-function rows, so a pair emitted twice becomes a
    /// duplicate solution that no query text explains.
    #[test]
    fn a_subject_with_two_satisfying_geometries_yields_exactly_one_row() {
        let rows = vec![
            row("fd", geo("hasDefaultGeometry"), node("gd1")),
            row("fd", geo("hasDefaultGeometry"), node("gd2")),
            row("gd1", geo("asWKT"), wkt(INSIDE)),
            row("gd2", geo("asWKT"), wkt(INSIDE_TOO)),
            row("gq", geo("asWKT"), wkt(SQUARE)),
        ];
        let index = index_of(&rows);
        let entry = index
            .entries()
            .iter()
            .find(|entry| entry.subject() == &iri("fd"))
            .expect("the feature is indexed");
        assert_eq!(
            entry.geometries().len(),
            2,
            "both default geometries must reach the feature, or there is nothing to deduplicate"
        );

        let relation = GeoRelation::new(Arc::new(index), SpatialRelation::SfWithin);
        let emitted = pairs(&relation);
        let hits = emitted
            .iter()
            .filter(|candidate| **candidate == pair("fd", "gq"))
            .count();
        assert_eq!(
            hits, 1,
            "two satisfying geometries entail one triple, so exactly one row: {emitted:?}"
        );
    }

    /// Two identical serializations of one geometry collapse, because they are
    /// the same geometry and decide every relation identically.
    #[test]
    fn duplicate_serializations_of_one_geometry_collapse_to_one() {
        let rows = vec![
            row("g1", geo("asWKT"), wkt(INSIDE)),
            row("g1", geo("asWKT"), wkt(INSIDE)),
        ];
        let index = index_of(&rows);
        assert_eq!(index.len(), 1);
        assert_eq!(
            index.entries()[0].geometries().len(),
            1,
            "the identical geometry decides every relation identically, so one copy is enough"
        );
    }

    // ---- 4. asserted triples still match ------------------------------------

    /// `:-` is an entailment rule, not a definition, so an asserted
    /// `ex:a geo:sfWithin ex:b` matches even when the geometries refute it. The
    /// converse control is in the same test: a computed pair with no asserted
    /// triple also yields a row, so the assertion is not doing all the work.
    #[test]
    fn an_asserted_triple_matches_even_when_the_geometries_refute_it() {
        let rows = vec![
            row("a", geo("asWKT"), wkt(OUTSIDE)),
            row("b", geo("asWKT"), wkt(INSIDE)),
            row("q", geo("asWKT"), wkt(SQUARE)),
            row("a", geo("sfWithin"), node("b")),
        ];
        let relation = relation_of(&rows, SpatialRelation::SfWithin);

        assert_eq!(
            relation.index().asserted(SpatialRelation::SfWithin),
            &[(iri("a"), iri("b"))],
            "the asserted pair is collected"
        );

        let emitted = pairs(&relation);
        assert!(
            emitted.contains(&pair("a", "b")),
            "the asserted triple must still match: POINT(9 9) is NOT within POINT(1 1), and the \
             rule is an entailment rather than a definition: {emitted:?}"
        );
        // The converse control: a purely computed pair, with no assertion.
        assert!(
            emitted.contains(&pair("b", "q")),
            "and a computed pair with no assertion behind it is a row too: {emitted:?}"
        );
        // And a pair that is neither computed nor asserted is absent.
        assert!(
            !emitted.contains(&pair("a", "q")),
            "POINT(9 9) is not within the square and nobody asserted that it was: {emitted:?}"
        );
        // The asserted pair reaches every mode, not only the all-free one.
        assert_eq!(
            invoke(&relation, &[Some(iri("a")), Some(iri("b"))], None).expect("no refusal"),
            vec![vec![iri("a"), iri("b")]],
            "a bb call on the asserted pair still matches"
        );
    }

    /// An asserted pair whose sides carry no serialization at all still matches.
    /// Requiring an index entry would silently drop an entailed triple.
    #[test]
    fn an_asserted_pair_matches_even_when_neither_side_carries_a_geometry() {
        let rows = vec![
            row("q", geo("asWKT"), wkt(SQUARE)),
            row("nogeom1", geo("sfWithin"), node("nogeom2")),
        ];
        let relation = relation_of(&rows, SpatialRelation::SfWithin);
        assert_eq!(
            relation.index().len(),
            1,
            "only the square is an indexed spatial object"
        );
        assert!(
            pairs(&relation).contains(&pair("nogeom1", "nogeom2")),
            "the asserted triple is entailed regardless of the index"
        );
    }

    // ---- 5. every mode ------------------------------------------------------

    /// The all-free declaration subsumes every access pattern of this arity, and
    /// each of the four must return the rows the engine would have got by
    /// filtering the all-free answer.
    #[test]
    fn every_mode_returns_the_rows_the_all_free_answer_would_have_been_filtered_to() {
        let relation = relation_of(&four_branch_rows(), SpatialRelation::SfWithin);
        for code in ["ff", "bf", "fb", "bb"] {
            assert!(
                relation.admits(BindingPattern::from_code(code)),
                "the all-free declaration must admit {code}"
            );
        }

        // ff — the whole answer.
        assert_eq!(pairs(&relation).len(), 12, "the fixture's full answer");

        // bf — a bound subject.
        let bf = invoke(&relation, &[Some(iri("gp")), None], None).expect("no refusal");
        assert_eq!(
            bf.iter()
                .map(|emitted| render(&emitted[1]))
                .collect::<Vec<_>>(),
            vec!["fp", "fq", "gp", "gq"],
            "the point is within itself, its own feature, and both squares"
        );

        // fb — a bound object.
        let fb = invoke(&relation, &[None, Some(iri("gq"))], None).expect("no refusal");
        assert_eq!(
            fb.iter()
                .map(|emitted| render(&emitted[0]))
                .collect::<Vec<_>>(),
            vec!["fp", "fq", "gp", "gq"],
            "everything is within the square, including the square itself"
        );

        // bb — both bound and agreeing.
        assert_eq!(
            invoke(&relation, &[Some(iri("gp")), Some(iri("gq"))], None).expect("no refusal"),
            vec![vec![iri("gp"), iri("gq")]],
            "one row, the pair itself"
        );

        // bb — both bound and disagreeing: the square is not within the point.
        assert!(
            invoke(&relation, &[Some(iri("gq")), Some(iri("gp"))], None)
                .expect("no refusal")
                .is_empty(),
            "a disagreeing bb pair returns no row"
        );

        // bb — a subject that is not indexed at all.
        assert!(
            invoke(&relation, &[Some(iri("absent")), Some(iri("gq"))], None)
                .expect("no refusal")
                .is_empty(),
            "an unindexed subject with no asserted triple names nothing"
        );
    }

    // ---- 6. the ceiling -----------------------------------------------------

    /// The licence's prefix property: a ceiling of `k` yields the FIRST `k` rows
    /// of the unbounded answer, in the same order, and then reports exhaustion.
    #[test]
    fn a_ceiling_yields_exactly_the_prefix_of_the_unbounded_answer() {
        let relation = relation_of(&four_branch_rows(), SpatialRelation::SfWithin);
        let full = invoke(&relation, &[None, None], None).expect("no refusal");
        assert_eq!(full.len(), 12, "the fixture's full answer");

        for k in 0..=(full.len() + 2) {
            let capped = invoke(&relation, &[None, None], Some(k as u64)).expect("no refusal");
            let expected = &full[..k.min(full.len())];
            assert_eq!(
                capped, expected,
                "a ceiling of {k} must yield exactly the first {k} rows"
            );
        }
    }

    /// The accounting that makes the licence sound: rows the cursor SKIPS
    /// disagree with a bound position and would have been cut by the engine's
    /// own equality filter anyway, so spending the ceiling on them would hand
    /// back fewer usable rows than the engine asked for.
    #[test]
    fn the_ceiling_counts_emitted_rows_not_skipped_ones() {
        let relation = relation_of(&four_branch_rows(), SpatialRelation::SfWithin);
        // `gq` is the LAST subject in sorted order, so in the all-free answer
        // every one of the ten rows before its first is a row this cursor skips.
        let rows = invoke(&relation, &[Some(iri("gq")), None], Some(1)).expect("no refusal");
        assert_eq!(
            rows,
            vec![vec![iri("gq"), iri("fq")]],
            "the skipped rows must not have consumed the single-row licence"
        );
    }

    // ---- 7. the declared row bounds -----------------------------------------

    /// The documented table, pinned. This is an admission input: the planner
    /// both orders this call and admits it against a row ceiling using these
    /// numbers, so a bound that understates reality turns an admission decision
    /// into a wrong one.
    #[test]
    fn the_row_bound_table_is_the_documented_one() {
        let rows = {
            let mut rows = four_branch_rows();
            rows.push(row("fp", geo("sfWithin"), node("fq")));
            rows
        };
        let relation = relation_of(&rows, SpatialRelation::SfWithin);
        let entries = relation.index().len() as u64;
        assert_eq!(entries, 4);
        let asserted = relation.index().asserted(SpatialRelation::SfWithin).len() as u64;
        assert_eq!(asserted, 1, "the asserted pair must be in the count");

        assert_eq!(
            relation.rows_per_invocation(BindingPattern::from_code("ff")),
            entries * entries + asserted,
            "ff: every ordered pair, plus every asserted pair"
        );
        assert_eq!(
            relation.rows_per_invocation(BindingPattern::from_code("bf")),
            entries + asserted,
            "bf: one entry against every entry, plus the asserted pairs"
        );
        assert_eq!(
            relation.rows_per_invocation(BindingPattern::from_code("fb")),
            entries + asserted,
            "fb: the mirror image"
        );
        assert_eq!(
            relation.rows_per_invocation(BindingPattern::from_code("bb")),
            1,
            "bb: the row IS the pair, and the rows are deduplicated"
        );
    }

    /// The declared bound must never be exceeded by the actual row count, in
    /// any mode, over several fixtures and several relations.
    #[test]
    fn the_declared_bound_is_never_exceeded_by_the_actual_row_count() {
        let fixtures = vec![
            four_branch_rows(),
            vec![
                row("gp", geo("asWKT"), wkt(INSIDE)),
                row("gq", geo("asWKT"), wkt(SQUARE)),
                row("gr", geo("asWKT"), wkt(OUTSIDE)),
                row("gp", geo("sfWithin"), node("gr")),
                row("gx", geo("sfDisjoint"), node("gy")),
            ],
            vec![row("gq", geo("asWKT"), wkt(SQUARE))],
            vec![row(
                "unrelated",
                "http://example.org/p".to_owned(),
                node("x"),
            )],
        ];
        let probes = [
            SpatialRelation::SfWithin,
            SpatialRelation::SfIntersects,
            SpatialRelation::SfDisjoint,
            SpatialRelation::EhCovers,
            SpatialRelation::Rcc8Po,
        ];
        for rows in &fixtures {
            let index = Arc::new(index_of(rows));
            let subjects: Vec<TermValue> = index
                .entries()
                .iter()
                .map(|entry| entry.subject().clone())
                .collect();
            for probe in probes {
                let relation = GeoRelation::new(Arc::clone(&index), probe);
                let mut cases: Vec<(&str, Vec<Option<TermValue>>)> = vec![("ff", vec![None, None])];
                if let (Some(first), Some(last)) = (subjects.first(), subjects.last()) {
                    cases.push(("bf", vec![Some(first.clone()), None]));
                    cases.push(("fb", vec![None, Some(last.clone())]));
                    cases.push(("bb", vec![Some(first.clone()), Some(last.clone())]));
                }
                for (code, bound) in cases {
                    let declared = relation.rows_per_invocation(BindingPattern::from_code(code));
                    let actual = invoke(&relation, &bound, None).expect("no refusal").len() as u64;
                    assert!(
                        actual <= declared,
                        "{probe:?} under {code}: emitted {actual} rows against a declared bound \
                         of {declared}"
                    );
                }
            }
        }
    }

    // ---- 8. determinism -----------------------------------------------------

    /// Two datasets built by interning the same triples in OPPOSITE orders must
    /// produce identical fingerprints and identical row sequences. The first
    /// assertion proves the two datasets genuinely differ, or there would be
    /// nothing for determinism to survive.
    #[test]
    fn opposite_ingestion_orders_produce_the_same_fingerprint_and_the_same_rows() {
        let forward = four_branch_rows();
        let mut backward = forward.clone();
        backward.reverse();

        let forward_dataset = dataset_of(&forward);
        let backward_dataset = dataset_of(&backward);
        assert_ne!(
            forward_dataset.term_id_by_value(&iri("gq")),
            backward_dataset.term_id_by_value(&iri("gq")),
            "the two datasets must genuinely differ in intern order, or this test proves nothing"
        );

        let forward_index =
            GeoIndex::from_dataset(&*forward_dataset, &vocab(), &config()).expect("clean");
        let backward_index =
            GeoIndex::from_dataset(&*backward_dataset, &vocab(), &config()).expect("clean");
        assert_eq!(
            forward_index.source_fingerprint(),
            backward_index.source_fingerprint(),
            "the fingerprint is a function of the data, not of the ingestion order"
        );
        assert_eq!(
            forward_index.entries(),
            backward_index.entries(),
            "and so is the entry table"
        );

        let left = GeoRelation::new(Arc::new(forward_index), SpatialRelation::SfWithin);
        let right = GeoRelation::new(Arc::new(backward_index), SpatialRelation::SfWithin);
        assert_eq!(
            pairs(&left),
            pairs(&right),
            "emission order is part of the seam's contract"
        );
    }

    // ---- 9. refusals, each with its neighbouring VALID case ------------------

    /// PurRDF mints no vocabulary, so there is no default serialization property
    /// set. The neighbouring valid case is a one-element list.
    #[test]
    fn an_empty_serialization_list_is_refused_and_a_one_element_list_is_not() {
        assert!(matches!(
            GeoIndexConfig::new(Vec::new(), GraphSelector::Any),
            Err(GeoError::Config(_))
        ));
        // The neighbouring VALID case.
        assert!(
            GeoIndexConfig::new(vec![TermValue::iri(geo("asWKT"))], GraphSelector::Any).is_ok()
        );
    }

    /// Only an IRI can occupy the predicate position, so a literal or a blank
    /// node names nothing. The neighbouring valid case is the same list with an
    /// IRI in it.
    #[test]
    fn a_non_iri_serialization_entry_is_refused_and_an_iri_is_not() {
        for bad in [TermValue::simple_literal("asWKT"), TermValue::blank("b0")] {
            assert!(
                matches!(
                    GeoIndexConfig::new(vec![bad.clone()], GraphSelector::Any),
                    Err(GeoError::Config(_))
                ),
                "{bad:?} is not an IRI"
            );
        }
        // The neighbouring VALID case.
        assert!(
            GeoIndexConfig::new(vec![TermValue::iri(geo("asGeoJSON"))], GraphSelector::Any).is_ok()
        );
    }

    /// A repeated entry is a caller mistake, and silently deduplicating it would
    /// hide the mistake. The neighbouring valid case is two distinct entries,
    /// which are also sorted so two callers agree byte for byte.
    #[test]
    fn a_repeated_serialization_entry_is_refused_and_two_distinct_ones_are_not() {
        let repeated = vec![TermValue::iri(geo("asWKT")), TermValue::iri(geo("asWKT"))];
        assert!(matches!(
            GeoIndexConfig::new(repeated, GraphSelector::Any),
            Err(GeoError::Config(_))
        ));
        // The neighbouring VALID case.
        let distinct = vec![
            TermValue::iri(geo("asWKT")),
            TermValue::iri(geo("asGeoJSON")),
        ];
        let config = GeoIndexConfig::new(distinct, GraphSelector::Any).expect("distinct entries");
        assert_eq!(
            config.serializations(),
            &[
                TermValue::iri(geo("asGeoJSON")),
                TermValue::iri(geo("asWKT")),
            ],
            "the list is sorted, so two callers naming the same set agree byte for byte"
        );
    }

    /// A named graph selector must hold an IRI, and the graph must be present in
    /// the dataset. Both neighbours are exercised.
    #[test]
    fn a_named_graph_selector_is_checked_and_a_present_graph_is_accepted() {
        assert!(matches!(
            GeoIndexConfig::new(
                vec![TermValue::iri(geo("asWKT"))],
                GraphSelector::Named(TermValue::simple_literal("g1"))
            ),
            Err(GeoError::Config(_))
        ));

        let named = GeoIndexConfig::new(
            vec![TermValue::iri(geo("asWKT"))],
            GraphSelector::Named(TermValue::iri(GRAPH)),
        )
        .expect("an IRI selector");

        // Absent from the dataset: a wiring mistake, not an empty index.
        let default_graph = dataset_of(&four_branch_rows());
        assert!(matches!(
            GeoIndex::from_dataset(&*default_graph, &vocab(), &named),
            Err(GeoError::Config(_))
        ));

        // The neighbouring VALID case: the same configuration over a dataset
        // that actually holds the graph.
        let in_graph = dataset_in(&four_branch_rows(), Some(GRAPH));
        let index = GeoIndex::from_dataset(&*in_graph, &vocab(), &named).expect("the graph exists");
        assert_eq!(index.len(), 4, "and it indexes the graph's spatial objects");
    }

    /// A serialization this crate does not implement is refused **by name**: the
    /// caller put that property in the conformance class, so skipping it would
    /// drop rows with no symptom. The neighbouring valid case is the same
    /// property carrying a `wktLiteral`.
    #[test]
    fn a_gml_literal_under_a_configured_property_is_refused_by_name_and_wkt_is_not() {
        let gml = vec![row(
            "g1",
            geo("asWKT"),
            Obj::Lit("<gml:Point/>".to_owned(), geo("gmlLiteral")),
        )];
        let error = GeoIndex::from_dataset(&*dataset_of(&gml), &vocab(), &config())
            .expect_err("gmlLiteral is not implemented here");
        assert!(
            matches!(error, GeoError::Unsupported(_)),
            "got {error:?}: an unimplemented serialization must be loud"
        );
        assert!(
            error.detail().contains("gmlLiteral"),
            "the refusal must name the datatype: {error}"
        );

        // The neighbouring VALID case.
        let ok = vec![row("g1", geo("asWKT"), wkt(INSIDE))];
        assert_eq!(
            GeoIndex::from_dataset(&*dataset_of(&ok), &vocab(), &config())
                .expect("wktLiteral is implemented")
                .len(),
            1
        );
    }

    /// A datatype that is none of the five GeoSPARQL serializations is bad data,
    /// and so is a non-literal object and a malformed lexical form. The
    /// neighbouring valid case follows all three.
    #[test]
    fn an_unreadable_serialization_object_is_refused_and_a_readable_one_is_not() {
        let foreign = vec![row(
            "g1",
            geo("asWKT"),
            Obj::Lit(
                INSIDE.to_owned(),
                "http://www.w3.org/2001/XMLSchema#string".to_owned(),
            ),
        )];
        assert!(matches!(
            GeoIndex::from_dataset(&*dataset_of(&foreign), &vocab(), &config()),
            Err(GeoError::Literal(_))
        ));

        let not_a_literal = vec![row("g1", geo("asWKT"), node("g2"))];
        assert!(matches!(
            GeoIndex::from_dataset(&*dataset_of(&not_a_literal), &vocab(), &config()),
            Err(GeoError::Literal(_))
        ));

        let malformed = vec![row("g1", geo("asWKT"), wkt("POINT(1"))];
        assert!(matches!(
            GeoIndex::from_dataset(&*dataset_of(&malformed), &vocab(), &config()),
            Err(GeoError::Literal(_))
        ));

        // The neighbouring VALID case.
        let ok = vec![row("g1", geo("asWKT"), wkt(INSIDE))];
        assert!(GeoIndex::from_dataset(&*dataset_of(&ok), &vocab(), &config()).is_ok());
    }

    /// Two geometries in different coordinate reference systems are refused
    /// rather than skipped: this crate reprojects nothing, and skipping the pair
    /// would be a missing answer. The neighbouring valid case is a `bb` call on
    /// a same-CRS pair from the very same index — the refusal must be scoped to
    /// the pairs actually compared, not poison the whole relation.
    #[test]
    fn a_mixed_crs_pair_is_refused_and_a_same_crs_pair_from_that_index_is_not() {
        let mixed = vec![
            row("ga", geo("asWKT"), wkt(INSIDE)),
            row("gb", geo("asWKT"), wkt(SQUARE)),
            row("gz", geo("asWKT"), wkt(&format!("<{CRS_OTHER}> {INSIDE}"))),
        ];
        let relation = relation_of(&mixed, SpatialRelation::SfWithin);
        assert_eq!(relation.index().len(), 3, "all three are indexed");

        let error = invoke(&relation, &[None, None], None)
            .expect_err("an all-free call must compare across the two systems");
        assert!(
            matches!(error, EvalError::Function(_)),
            "got {error:?}: a domain error reaches the evaluator as a function failure"
        );
        assert!(
            error
                .to_string()
                .contains("different coordinate reference systems"),
            "the refusal must say what it refused: {error}"
        );

        // The neighbouring VALID case, from the SAME index: a bb call whose two
        // sides share a system compares only that pair.
        assert_eq!(
            invoke(&relation, &[Some(iri("ga")), Some(iri("gb"))], None)
                .expect("both sides are in the default system"),
            vec![vec![iri("ga"), iri("gb")]],
            "a same-CRS pair must still answer; over-refusal here would break every query that \
             never crosses the two systems"
        );
    }

    /// An index paired with the wrong dataset is a silent wrong answer, because
    /// the seam hands `open` no dataset to check against. The neighbouring valid
    /// case is the dataset the index was actually built from.
    #[test]
    fn verify_binding_refuses_the_wrong_dataset_and_accepts_the_right_one() {
        let source = dataset_of(&four_branch_rows());
        let index = GeoIndex::from_dataset(&*source, &vocab(), &config()).expect("a clean fixture");

        // The neighbouring VALID case, first.
        verify_binding(&index, &*source, &vocab(), &config())
            .expect("the index was built from this dataset");

        let other = dataset_of(&[
            row("gp", geo("asWKT"), wkt(OUTSIDE)),
            row("gq", geo("asWKT"), wkt(SQUARE)),
        ]);
        let error = verify_binding(&index, &*other, &vocab(), &config())
            .expect_err("a different dataset must be caught");
        assert!(matches!(error, GeoError::Config(_)), "got {error:?}");
        assert!(
            error.detail().contains("built over a different dataset"),
            "the refusal must say what it found: {error}"
        );

        // A configuration that is not the one the index was built under is
        // refused before any digest is computed, because it would answer a
        // different question.
        let narrower =
            GeoIndexConfig::new(vec![TermValue::iri(geo("asWKT"))], GraphSelector::Default)
                .expect("a valid configuration");
        assert!(matches!(
            verify_binding(&index, &*source, &vocab(), &narrower),
            Err(GeoError::Config(_))
        ));
    }

    /// Registering nothing while returning success looks like success. The
    /// neighbouring valid case is one family, which registers its eight
    /// relations under the caller's namespace and nothing else.
    #[test]
    fn registering_no_families_is_refused_and_registering_one_is_not() {
        let index = Arc::new(index_of(&four_branch_rows()));
        let mut empty = PropertyFunctionRegistry::new();
        assert!(matches!(
            register(&mut empty, &vocab(), &index, &[]),
            Err(GeoError::Config(_))
        ));
        assert!(empty.is_empty(), "and nothing was registered");

        // The neighbouring VALID case.
        let mut registry = PropertyFunctionRegistry::new();
        register(
            &mut registry,
            &vocab(),
            &index,
            &[RelationFamily::SimpleFeatures],
        )
        .expect("one family");
        assert_eq!(registry.len(), 8, "each family holds eight relations");
        assert!(
            registry.resolve(&geo("sfWithin")).is_some(),
            "registered under the caller's geo: namespace"
        );
        assert!(
            registry.resolve(&geo("rcc8po")).is_none(),
            "and only the family asked for"
        );

        // All three families is the full twenty-four.
        let mut all = PropertyFunctionRegistry::new();
        register(&mut all, &vocab(), &index, &RelationFamily::ALL).expect("every family");
        assert_eq!(all.len(), 24);
        let described = all.describe().expect("no relation panics");
        assert_eq!(described.len(), 24);
        assert!(
            described
                .iter()
                .all(|d| d.subject_arity == 1 && d.object_arity == 1 && d.modes.len() == 1),
            "every registered relation declares the same one-in/one-out shape"
        );
    }

    /// Overlapping registrations are a host misconfiguration the registry
    /// catches where it is committed.
    #[test]
    #[should_panic(expected = "already registered as a property function")]
    fn registering_overlapping_families_twice_panics() {
        let index = Arc::new(index_of(&four_branch_rows()));
        let mut registry = PropertyFunctionRegistry::new();
        register(
            &mut registry,
            &vocab(),
            &index,
            &[RelationFamily::SimpleFeatures],
        )
        .expect("first");
        drop(register(
            &mut registry,
            &vocab(),
            &index,
            &[RelationFamily::SimpleFeatures],
        ));
    }

    /// A call whose argument vectors do not match the declared arity is refused
    /// before anything is scanned; the two-argument call is the valid
    /// neighbour.
    #[test]
    fn a_wrong_argument_count_is_refused_and_the_declared_one_is_not() {
        let relation = relation_of(&four_branch_rows(), SpatialRelation::SfWithin);
        let subject: [Option<&TermValue>; 0] = [];
        let object = [None];
        let args = PfArgs::new(&subject, &object);
        assert!(matches!(
            relation.open(&args, None),
            Err(EvalError::Function(_))
        ));
        // The neighbouring VALID case.
        assert!(invoke(&relation, &[None, None], None).is_ok());
    }

    // ---- the declared shape -------------------------------------------------

    /// The seam reads these four declarations on every prepare; this pins them.
    #[test]
    fn the_declared_shape_is_the_documented_one() {
        let relation = relation_of(&four_branch_rows(), SpatialRelation::SfWithin);
        assert_eq!(relation.relation(), SpatialRelation::SfWithin);
        assert_eq!(relation.arity().subject, 1);
        assert_eq!(relation.arity().object, 1);
        assert_eq!(relation.volatility(), Volatility::Stable);
        assert_eq!(
            relation
                .modes()
                .iter()
                .map(|mode| BindingPattern::code(*mode))
                .collect::<Vec<_>>(),
            vec!["ff".to_owned()],
            "exactly one declared mode: the all-free one"
        );
    }

    /// An index with no spatial objects at all is an honest empty relation, not
    /// a failure — and its declared bounds are zero rather than wrapping.
    #[test]
    fn an_empty_index_is_an_honest_empty_relation() {
        let empty = GeoIndex::from_dataset(
            &*dataset_of(&[row(
                "a",
                "http://example.org/unrelated".to_owned(),
                node("b"),
            )]),
            &vocab(),
            &config(),
        )
        .expect("a dataset with no geometry is not an error");
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);

        let relation = GeoRelation::new(Arc::new(empty), SpatialRelation::SfWithin);
        assert_eq!(
            relation.rows_per_invocation(BindingPattern::from_code("ff")),
            0
        );
        assert_eq!(
            pairs(&relation),
            Vec::new(),
            "an index with no entries yields no rows"
        );
    }

    /// A GeoJSON serialization reaches the index exactly as a WKT one does — the
    /// rule's `ogc:asGeomLiteral` is whichever property the conformance class
    /// names, and the datatype decides how the object is read.
    #[test]
    fn a_geojson_serialization_is_indexed_beside_a_wkt_one() {
        let config = GeoIndexConfig::new(
            vec![
                TermValue::iri(geo("asWKT")),
                TermValue::iri(geo("asGeoJSON")),
            ],
            GraphSelector::Any,
        )
        .expect("two distinct IRIs");
        let rows = vec![
            row("gq", geo("asWKT"), wkt(SQUARE)),
            row(
                "gj",
                geo("asGeoJSON"),
                Obj::Lit(
                    r#"{"type":"Point","coordinates":[1,1]}"#.to_owned(),
                    geo("geoJSONLiteral"),
                ),
            ),
        ];
        let index = GeoIndex::from_dataset(&*dataset_of(&rows), &vocab(), &config)
            .expect("both datatypes are implemented");
        assert_eq!(index.len(), 2);

        let relation = GeoRelation::new(Arc::new(index), SpatialRelation::SfWithin);
        assert!(
            pairs(&relation).contains(&pair("gj", "gq")),
            "the GeoJSON point is within the WKT square"
        );
    }

    /// The entry accessors are the ones the module documents, and the table is
    /// stored in subject order so the binary search behind a bound position is
    /// exact.
    #[test]
    fn an_entry_carries_its_subject_and_its_geometries_in_subject_order() {
        let index = index_of(&four_branch_rows());
        let entries: &[GeoEntry] = index.entries();
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].subject(), &iri("fp"));
        assert_eq!(entries[0].geometries().len(), 1);
        let subjects: Vec<String> = entries.iter().map(|e| render(e.subject())).collect();
        let mut sorted = subjects.clone();
        sorted.sort();
        assert_eq!(subjects, sorted, "entries are stored in subject order");
    }

    // -----------------------------------------------------------------------
    // Regressions: the three ways this index used to answer a short or wrong bag
    // -----------------------------------------------------------------------

    /// Two geometries that RENDER alike at the coordinate scale but are not the
    /// same geometry must both survive into the index.
    ///
    /// The ordering key used to be the rendered WKT alone, and `canonical`
    /// deduplicates on it. `wkt::write` rounds, so two distinct geometries that
    /// agreed to `coordinate_scale` fraction digits collapsed into one and the
    /// surviving choice depended on ingestion order. The pairing that would have
    /// satisfied the relation then never got tried, and the relation answered a
    /// SHORT BAG that the engine reads as complete — no error, just missing rows.
    ///
    /// The assertion is exact set equality, because a `>=` or a `contains` would
    /// pass for the very bag this test exists to reject.
    #[test]
    fn geometries_that_render_alike_are_kept_apart_and_true_duplicates_are_still_merged() {
        // Distinct beyond the default scale of 15 fraction digits, so the two
        // render identically and only the exact key tells them apart.
        const NEAR_A: &str = "POINT(1.0000000000000001 1)";
        const NEAR_B: &str = "POINT(1.0000000000000002 1)";
        let relation = relation_of(
            &[
                row("g1", geo("asWKT"), wkt(NEAR_A)),
                row("g1", geo("asWKT"), wkt(NEAR_B)),
                row("g2", geo("asWKT"), wkt(NEAR_B)),
            ],
            SpatialRelation::SfEquals,
        );
        let mut got = pairs(&relation);
        got.sort();
        assert_eq!(
            got,
            vec![
                pair("g1", "g1"),
                pair("g1", "g2"),
                pair("g2", "g1"),
                pair("g2", "g2"),
            ],
            "g1 carries a geometry equal to g2's, so both cross pairs are entailed; \
             dropping one as a rendering duplicate loses them silently"
        );

        // The neighbouring VALID case: a geometry asserted TWICE with the same
        // lexical form really is one geometry, and deduplication must still
        // happen — the fix must not have turned the dedup off.
        let deduped = relation_of(
            &[
                row("g1", geo("asWKT"), wkt(INSIDE)),
                row("g1", geo("asWKT"), wkt(INSIDE)),
            ],
            SpatialRelation::SfEquals,
        );
        assert_eq!(
            pairs(&deduped),
            vec![pair("g1", "g1")],
            "one subject with one distinct geometry yields exactly one row"
        );
    }

    /// The source digest must distinguish datasets that differ BELOW the integer.
    ///
    /// The digest used to render every geometry at coordinate scale zero, which
    /// rounds every ordinate to the nearest integer. `POINT(1.4 1.4)` and
    /// `POINT(1.2 1.2)` therefore digested alike and `verify_binding` returned
    /// `Ok(())` for an index built over a different dataset — accepting a value
    /// it had discarded before comparing it, which is exactly the silent wrong
    /// answer the function exists to refuse.
    #[test]
    fn verify_binding_catches_a_dataset_that_differs_only_below_the_integer() {
        let source = dataset_of(&[row("g1", geo("asWKT"), wkt("POINT(1.4 1.4)"))]);
        let index = GeoIndex::from_dataset(&*source, &vocab(), &config()).expect("a clean fixture");

        // The neighbouring VALID case first: the very dataset it was built from.
        verify_binding(&index, &*source, &vocab(), &config())
            .expect("the index was built from this dataset");

        for different in ["POINT(1.2 1.2)", "POINT(0.4 0.4)", "POINT(1.44 1.4)"] {
            let other = dataset_of(&[row("g1", geo("asWKT"), wkt(different))]);
            let error = verify_binding(&index, &*other, &vocab(), &config())
                .expect_err("a dataset differing below the integer must be caught");
            assert!(
                matches!(error, GeoError::Config(_)),
                "got {error:?} for {different}"
            );
        }
    }

    /// A cross-system pairing must not refuse a row that a same-system pairing
    /// already entails.
    ///
    /// The rule is an existential over pairings, so one same-system witness
    /// settles it. The check used to run over EVERY pairing before any relate
    /// did, so merely carrying a feature in a second coordinate reference system
    /// — ordinary GeoSPARQL — broke a query that worked without it.
    #[test]
    fn a_cross_crs_pairing_does_not_refuse_a_row_a_same_crs_pairing_entails() {
        let other_crs_point = format!("<{CRS_OTHER}> {INSIDE}");
        let relation = relation_of(
            &[
                // ga is inside gq in the DEFAULT system, and also carries a
                // serialization in a second system that cannot be compared.
                row("ga", geo("asWKT"), wkt(INSIDE)),
                row("ga", geo("asWKT"), wkt(&other_crs_point)),
                row("gq", geo("asWKT"), wkt(SQUARE)),
            ],
            SpatialRelation::SfWithin,
        );
        let rows = invoke(&relation, &[Some(iri("ga")), Some(iri("gq"))], None)
            .expect("a same-system witness entails the row, so this must not refuse");
        assert_eq!(rows.len(), 1, "the row is entailed exactly once");

        // The neighbouring case that MUST still refuse: no same-system pairing
        // exists at all, so a `false` would be indistinguishable from "these
        // geometries genuinely do not relate".
        let unanswerable = relation_of(
            &[
                row("gz", geo("asWKT"), wkt(&other_crs_point)),
                row("gq", geo("asWKT"), wkt(SQUARE)),
            ],
            SpatialRelation::SfWithin,
        );
        assert!(
            invoke(&unanswerable, &[Some(iri("gz")), Some(iri("gq"))], None).is_err(),
            "with no comparable pairing the honest answer is a refusal, not false"
        );
    }
}

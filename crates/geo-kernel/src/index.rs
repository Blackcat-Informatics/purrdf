// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Frozen dataset geometry projection and exact spatial-relation queries.

mod geographic;
mod projection;
pub use geographic::{GeographicGeoIndex, GeographicRelationReceipt};
use projection::ProjectionWork;

use core::convert::Infallible;
use core::ops::ControlFlow;
use core::slice;
use purrdf_hash::{Domain, fnv};
use std::collections::BTreeMap;

use crate::topology::{relate, topological_dimension};
use crate::{GeoError, GeoTerm, GeoVocab, GeometryLiteral, SpatialRelation};
use purrdf_core::{BlankScope, DatasetView, GraphMatch, RdfTextDirection, TermValue};

/// The domain-separation prefix of the index source digest.
const DIGEST_DOMAIN: Domain = Domain::new(b"purrdf-geo/index-source/v1");

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

/// An FNV-1a accumulator over [`purrdf_hash::fnv`].
///
/// FNV-1a rather than std's default SipHash hasher or the workspace's `FixedHasher`
/// because a fingerprint is compared against one computed by a *different run* of
/// this code: std's default hasher is explicitly unspecified across releases, and `FixedHasher` computes a different function on a build whose
/// target enables AES than on one that does not. Either would make
/// [`verify_binding`] answer "different dataset" for a dataset that is in fact
/// identical, the moment a toolchain moved. FNV-1a is fully specified integer
/// arithmetic, pinned by its reference test values, so the fingerprint is a pure
/// function of the bytes fed to it on every target and every release.
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
        let mut digest = Self { state: fnv::BASIS };
        digest.field(DIGEST_DOMAIN.as_str());
        digest
    }

    /// Absorb raw bytes.
    fn bytes(&mut self, bytes: &[u8]) {
        self.state = fnv::fold(self.state, bytes);
    }

    /// Absorb a one-byte tag.
    fn tag(&mut self, tag: u8) {
        self.bytes(&[tag]);
    }

    /// Absorb a `usize` count as eight big-endian bytes.
    fn count(&mut self, count: usize) {
        self.bytes(&(count as u64).to_be_bytes());
    }

    /// Absorb a length-prefixed string: its length as eight **big-endian** bytes,
    /// then its bytes.
    ///
    /// Not `purrdf_hash::frame::frame_le`'s little-endian framing, and never to
    /// become it: the source fingerprint this digest computes is a published
    /// identity — `verify_binding` compares it against a fingerprint recorded by
    /// an earlier run, and the geo determinism goldens pin it — so its byte order
    /// is frozen with it.
    fn field(&mut self, text: &str) {
        self.bytes(&(text.len() as u64).to_be_bytes());
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

    /// Absorb a term value, tag first, through a triple term's components.
    ///
    /// The terms are absorbed in [`TermValue::visit_terms`]'s pre-order — a triple
    /// term's tag, then its subject's whole nesting, then its predicate's, then its
    /// object's. The walk is unbounded by design and by the same contract every
    /// other term walker in this workspace relies on: a term value is finite, and a
    /// depth cap here would add a refusal that rejects legal RDF 1.2 data while
    /// protecting against nothing this crate can actually receive.
    fn term(&mut self, value: &TermValue) {
        let ControlFlow::Continue(()) = value.visit_terms(|term| -> ControlFlow<Infallible> {
            match term {
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
                TermValue::Triple { .. } => self.tag(TERM_TRIPLE),
            }
            ControlFlow::Continue(())
        });
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

pub use purrdf_core::GraphSelector;

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
/// an a shared owner, because the evaluator property-function adapter receives no dataset and so
/// cannot reach the geometries at query time.
///
/// Everything it holds is **sorted**: the entries by subject, each entry's
/// geometries by their canonical rendering, and each relation's asserted pairs
/// lexicographically. No map's iteration order reaches a result, so two hosts
/// that ingest the same triples in different orders build indexes with the same
/// contents *and* the same [`source_fingerprint`](Self::source_fingerprint).
///
/// An index holding nothing is well formed and needs no special case anywhere: it
/// has no entries, an empty asserted vector for every relation, and a real
/// fingerprint over its configuration and its (empty) contents, so two empty
/// indexes under different configurations stay distinguishable and the value moves
/// the moment the first geometry lands. See
/// [`from_dataset`](Self::from_dataset) for the reachable route to it.
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
    /// # The empty index, and why an absent graph reaches it
    ///
    /// A [`GraphSelector::Named`] graph the dataset has not interned holds
    /// nothing, because a graph IRI is interned only once a quad is in that
    /// graph. So no quad can match the selector, and the projection is the empty
    /// index: no entries, no asserted pair under any relation, and a
    /// [`source_fingerprint`](Self::source_fingerprint) computed by the same call
    /// over the same (empty) contents the populated path uses.
    ///
    /// That is an ordinary operating state rather than a fault — an index standing
    /// ready before the data it will hold arrives is exactly what a host building
    /// one over a graph it is about to load has. It is also the posture this
    /// function already takes one step below for an absent serialization
    /// property, and the two conditions are the same class: a term the corpus has
    /// not got yet. What stays a refusal is a configuration with no subject at all
    /// — an empty serialization list, which [`GeoIndexConfig::new`] rejects,
    /// because this crate mints no vocabulary to guess one.
    ///
    /// # Errors
    ///
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
        Self::project_dataset(dataset, vocab, config, &mut ProjectionWork::plain())
    }

    /// Project the same original dataset with complete parser, owned term,
    /// canonical ordering and fingerprint admission under the worker's policy.
    /// # Errors
    /// Adds operational work/storage/output refusal to [`Self::from_dataset`].
    pub fn from_dataset_in_context<D: DatasetView>(
        dataset: &D,
        vocab: &GeoVocab,
        config: &GeoIndexConfig,
        context: &mut crate::MetricContext,
    ) -> Result<Self, GeoError> {
        Self::project_observed(dataset, vocab, config, context, None)
    }

    /// Project the same exact dataset with bounded work/cancellation callbacks.
    /// # Errors
    /// Adds observer refusal to [`Self::from_dataset_in_context`].
    pub fn from_dataset_metered<D: DatasetView>(
        dataset: &D,
        vocab: &GeoVocab,
        config: &GeoIndexConfig,
        context: &mut crate::MetricContext,
        observer: &mut dyn crate::MetricWorkObserver,
    ) -> Result<Self, GeoError> {
        Self::project_observed(dataset, vocab, config, context, Some(observer))
    }

    fn project_observed<D: DatasetView>(
        dataset: &D,
        vocab: &GeoVocab,
        config: &GeoIndexConfig,
        context: &mut crate::MetricContext,
        observer: Option<&mut dyn crate::MetricWorkObserver>,
    ) -> Result<Self, GeoError> {
        context.begin(1)?;
        let mut work = ProjectionWork::admitted(context, observer);
        work.tick(1)?;
        let result = Self::project_dataset(dataset, vocab, config, &mut work);
        work.finish(result.is_ok())?;
        result
    }

    fn project_dataset<D: DatasetView>(
        dataset: &D,
        vocab: &GeoVocab,
        config: &GeoIndexConfig,
        work: &mut ProjectionWork<'_, '_>,
    ) -> Result<Self, GeoError> {
        work.config(config)?;
        dataset
            .checked_read(|dataset| {
                let Some(graph) = config
                    .graph()
                    .resolve(dataset)
                    .map_err(|error| GeoError::source_read(error.to_string()))?
                else {
                    // The configured named graph is not interned, so the dataset holds no
                    // quad in it and nothing can match. The empty projection is built
                    // through the ordinary steps — an empty entry table, one empty
                    // asserted vector per relation so `asserted` stays in bounds, and the
                    // same `fingerprint` call — rather than a second construction path
                    // that could drift from the first.
                    let entries: Vec<GeoEntry> = Vec::new();
                    work.reserve::<Vec<(TermValue, TermValue)>>(SpatialRelation::ALL.len())?;
                    let asserted: Vec<Vec<(TermValue, TermValue)>> =
                        vec![Vec::new(); SpatialRelation::ALL.len()];
                    let source_fingerprint = fingerprint(config, &entries, &asserted, work)?;
                    return Ok(Self {
                        config: config.clone(),
                        entries,
                        asserted,
                        source_fingerprint,
                    });
                };

                // Step 1 — the geometry nodes, keyed by dataset id so step 2 can join
                // against them without resolving anything twice.
                let mut by_node: BTreeMap<D::Id, Vec<Keyed>> = BTreeMap::new();
                for property in config.serializations() {
                    work.tick(1)?;
                    let Some(predicate) = dataset
                        .term_id_by_value(property)
                        .map_err(|error| GeoError::source_read(error.to_string()))?
                    else {
                        // A conformance class may name `geo:asGeoJSON` over a dataset
                        // that holds only WKT. That is an ordinary empty match, not a
                        // configuration error.
                        continue;
                    };
                    for quad in dataset.quads_for_pattern(None, Some(predicate), None, graph) {
                        work.tick(1)?;
                        let object = work.resolve(dataset, quad.o)?;
                        let literal = parse_serialization(&object, property, vocab, work)?;
                        let keyed = Keyed::of_with(literal, vocab, work)?;
                        let destination = work.map_entry(&mut by_node, quad.s)?;
                        work.output_count(
                            destination
                                .len()
                                .checked_add(1)
                                .ok_or(GeoError::ArithmeticOverflow("geometry count"))?,
                        )?;
                        if destination.len() == destination.capacity() {
                            work.reserve::<Keyed>(1)?;
                            destination.reserve_exact(1);
                        }
                        destination.push(keyed);
                    }
                    work.tick(0)?;
                }

                // Steps 2 and 3 — every geometry node is an entry in its own right, and
                // every default-geometry subject inherits its geometries.
                let mut by_subject = BTreeMap::new();
                for (id, geometries) in &by_node {
                    work.reserve::<Keyed>(geometries.len())?;
                    let mut copied = Vec::with_capacity(geometries.len());
                    for geometry in geometries {
                        copied.push(work.clone_keyed(geometry)?);
                    }
                    *work.map_entry(&mut by_subject, *id)? = copied;
                }
                for term in [GeoTerm::HasDefaultGeometry, GeoTerm::DefaultGeometry] {
                    work.storage(vocab.term(term).len() as u64)?;
                    let iri = TermValue::iri(vocab.term(term));
                    let Some(predicate) = dataset
                        .term_id_by_value(&iri)
                        .map_err(|error| GeoError::source_read(error.to_string()))?
                    else {
                        continue;
                    };
                    for quad in dataset.quads_for_pattern(None, Some(predicate), None, graph) {
                        work.tick(1)?;
                        work.map_lookup(by_node.len())?;
                        let Some(inherited) = by_node.get(&quad.o) else {
                            continue;
                        };
                        work.reserve::<Keyed>(inherited.len())?;
                        let destination = work.map_entry(&mut by_subject, quad.s)?;
                        destination.reserve_exact(inherited.len());
                        for geometry in inherited {
                            destination.push(work.clone_keyed(geometry)?);
                        }
                    }
                    work.tick(0)?;
                }

                let entries = finish_entries(dataset, by_subject, work)?;
                let asserted = collect_asserted(dataset, vocab, graph, work)?;
                let source_fingerprint = fingerprint(config, &entries, &asserted, work)?;
                Ok(Self {
                    config: config.clone(),
                    entries,
                    asserted,
                    source_fingerprint,
                })
            })
            .map_err(|error| GeoError::source_read(error.to_string()))?
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
    /// FNV-1a ([`purrdf_hash::fnv`]) over the configuration, the sorted entries (each
    /// subject and each of its geometries in canonical WKT), and the sorted
    /// asserted pairs. FNV-1a rather than a hasher whose output is a function of
    /// its own version, because the value is compared against one computed by a
    /// different run of this code; see [`verify_binding`] for what the comparison
    /// is for.
    #[must_use]
    pub const fn source_fingerprint(&self) -> u64 {
        self.source_fingerprint
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
    fn holds_relation(
        relation: SpatialRelation,
        left: &GeoEntry,
        right: &GeoEntry,
    ) -> Result<bool, GeoError> {
        let mut undecidable: Option<GeoError> = None;
        for a in &left.geometries {
            for b in &right.geometries {
                if let Err(error) = a.require_same_crs(b) {
                    undecidable.get_or_insert(error);
                    continue;
                }
                let matrix = relate(a.geometry(), b.geometry());
                if relation.holds(
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
    /// [`GeoError::Domain`] when no comparable geometry pairing can decide the relation.
    pub fn relation_pairs(
        &self,
        relation: SpatialRelation,
        subject: Option<&TermValue>,
        object: Option<&TermValue>,
    ) -> Result<Vec<(TermValue, TermValue)>, GeoError> {
        self.relation_pairs_with(relation, subject, object, &mut PlanarPairs(relation))
    }

    fn relation_pairs_with(
        &self,
        relation: SpatialRelation,
        subject: Option<&TermValue>,
        object: Option<&TermValue>,
        worker: &mut impl PairWorker,
    ) -> Result<Vec<(TermValue, TermValue)>, GeoError> {
        worker.bindings(subject, object)?;
        let lefts = self.candidates(subject);
        let rights = self.candidates(object);

        // Reserve for the CROSS PRODUCT's shape, not its size. The product is
        // quadratic and a `(TermValue, TermValue)` is not small, so reserving it
        // up front would commit gigabytes for a large all-free invocation —
        // before a single `relate` has run, and even when the answer is a handful
        // of rows or the caller wanted `LIMIT 1`. The satisfying pairs are a
        // subset of unknown size, so the honest starting point is the larger side
        // and then growth; that keeps the common small case allocation-free
        // without betting the process on the worst case.
        let capacity = lefts.len().max(rights.len());
        worker.begin(capacity)?;
        let mut pairs: Vec<(TermValue, TermValue)> = Vec::with_capacity(capacity);
        for left in lefts {
            for right in rights {
                if worker.holds(left, right)? {
                    worker.row(&left.subject, &right.subject, pairs.len(), pairs.capacity())?;
                    pairs.push((left.subject.clone(), right.subject.clone()));
                }
            }
        }

        // The entailment half: an asserted triple matches whether or not the
        // geometries satisfy the relation, and neither side need be an index
        // entry at all.
        for (asserted_subject, asserted_object) in self.asserted(relation) {
            if subject.is_some_and(|bound| bound != asserted_subject)
                || object.is_some_and(|bound| bound != asserted_object)
            {
                continue;
            }
            worker.row(
                asserted_subject,
                asserted_object,
                pairs.len(),
                pairs.capacity(),
            )?;
            pairs.push((asserted_subject.clone(), asserted_object.clone()));
        }

        worker.finish(&pairs)?;
        purrdf_lex::walk::try_sort_unstable_by(&mut pairs, |left, right| {
            worker.compare(left, right)
        })?;
        purrdf_lex::walk::try_dedup_by(&mut pairs, |left, right| {
            worker.compare(left, right).map(core::cmp::Ordering::is_eq)
        })?;
        Ok(pairs)
    }

    /// The entry for `subject`, by binary search over the sorted entries.
    #[must_use]
    pub fn entry_of(&self, subject: &TermValue) -> Option<&GeoEntry> {
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

trait PairWorker {
    fn bindings(
        &mut self,
        subject: Option<&TermValue>,
        object: Option<&TermValue>,
    ) -> Result<(), GeoError>;
    fn begin(&mut self, capacity: usize) -> Result<(), GeoError>;
    fn holds(&mut self, left: &GeoEntry, right: &GeoEntry) -> Result<bool, GeoError>;
    fn row(
        &mut self,
        subject: &TermValue,
        object: &TermValue,
        len: usize,
        capacity: usize,
    ) -> Result<(), GeoError>;
    fn finish(&mut self, pairs: &[(TermValue, TermValue)]) -> Result<(), GeoError>;
    fn compare(
        &mut self,
        left: &(TermValue, TermValue),
        right: &(TermValue, TermValue),
    ) -> Result<core::cmp::Ordering, GeoError>;
}

struct PlanarPairs(SpatialRelation);
impl PairWorker for PlanarPairs {
    fn bindings(&mut self, _: Option<&TermValue>, _: Option<&TermValue>) -> Result<(), GeoError> {
        Ok(())
    }
    fn begin(&mut self, _: usize) -> Result<(), GeoError> {
        Ok(())
    }
    fn holds(&mut self, left: &GeoEntry, right: &GeoEntry) -> Result<bool, GeoError> {
        GeoIndex::holds_relation(self.0, left, right)
    }
    fn row(&mut self, _: &TermValue, _: &TermValue, _: usize, _: usize) -> Result<(), GeoError> {
        Ok(())
    }
    fn finish(&mut self, _: &[(TermValue, TermValue)]) -> Result<(), GeoError> {
        Ok(())
    }
    fn compare(
        &mut self,
        left: &(TermValue, TermValue),
        right: &(TermValue, TermValue),
    ) -> Result<core::cmp::Ordering, GeoError> {
        Ok(left.cmp(right))
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
    fn of_with(
        literal: GeometryLiteral,
        vocab: &GeoVocab,
        work: &mut ProjectionWork<'_, '_>,
    ) -> Result<Self, GeoError> {
        let mut key = work.write(&literal, vocab.coordinate_scale())?;
        push_exact_ordinates(&mut key, &literal, work)?;
        Ok(Self { key, literal })
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
fn push_exact_ordinates(
    key: &mut String,
    literal: &GeometryLiteral,
    work: &mut ProjectionWork<'_, '_>,
) -> Result<(), GeoError> {
    // The unit separator keeps the exact suffix out of the rendering's alphabet,
    // so the rendering remains the primary sort key.
    work.append(key, "\u{1}")?;
    for coord in literal.geometry().coords() {
        for ordinate in [Some(coord.x()), Some(coord.y()), coord.z(), coord.m()] {
            match ordinate {
                Some(value) => {
                    work.exact_ordinate(key, value)?;
                }
                None => work.append(key, "|_")?,
            }
        }
    }
    Ok(())
}

/// The lossless identity of one geometry literal, independent of any vocabulary
/// setting.
///
/// The structural half is rendered at scale zero so that no vocabulary knob can
/// change the digest of an index whose rows did not change; the exact half then
/// restores every ordinate value, so the identity is complete rather than a
/// rounding of one.
fn exact_identity(
    literal: &GeometryLiteral,
    work: &mut ProjectionWork<'_, '_>,
) -> Result<String, GeoError> {
    let mut key = work.write(literal, 0)?;
    push_exact_ordinates(&mut key, literal, work)?;
    Ok(key)
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
    vocab: &GeoVocab,
    work: &mut ProjectionWork<'_, '_>,
) -> Result<GeometryLiteral, GeoError> {
    if !matches!(object, TermValue::Literal { .. }) {
        return Err(GeoError::literal(format!(
            "the object of the configured serialization property {property:?} is {object:?}, \
             which is not a literal; only a literal can carry a geometry serialization, so this \
             row names no geometry and passing over it would drop an answer the configuration \
             asked for"
        )));
    }
    work.geometry_arg(vocab, object)
}

/// Resolve a dataset-local id to its dataset-independent [`TermValue`] through
/// [`DatasetView::term_value`].
///
/// # Errors
///
/// [`GeoError::Config`] when the view hands back an id that is not its own — a
/// literal whose datatype does not resolve to an IRI. The view is the host's
/// wiring, so the refusal names it rather than indexing a term with an invented
/// datatype.
fn resolve_value<D: DatasetView>(dataset: &D, id: D::Id) -> Result<TermValue, GeoError> {
    dataset.term_value(id).map_err(|error| match error {
        foreign @ purrdf_core::TermLookupError::ForeignId => {
            GeoError::config(format!("the dataset view is inconsistent: {foreign}"))
        }
        purrdf_core::TermLookupError::Read(error) => GeoError::source_read(error.to_string()),
    })
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
    work: &mut ProjectionWork<'_, '_>,
) -> Result<Vec<GeoEntry>, GeoError> {
    work.output_count(by_subject.len())?;
    work.reserve::<(TermValue, Vec<Keyed>)>(by_subject.len())?;
    let mut rows = Vec::with_capacity(by_subject.len());
    for (id, geometries) in by_subject {
        rows.push((work.resolve(dataset, id)?, geometries));
    }
    purrdf_lex::walk::try_sort_unstable_by(&mut rows, |left, right| {
        work.compare_terms(&left.0, &right.0)
    })?;

    work.reserve::<GeoEntry>(rows.len())?;
    let mut entries: Vec<GeoEntry> = Vec::with_capacity(rows.len());
    let mut current: Option<(TermValue, Vec<Keyed>)> = None;
    for (subject, geometries) in rows {
        match current.take() {
            // Two ids resolving to one value cannot happen in an interned
            // dataset, but coalescing costs one comparison and keeps the
            // binary-search precondition true by construction rather than by
            // assumption.
            Some((held, mut collected)) if work.compare_terms(&held, &subject)?.is_eq() => {
                work.reserve::<Keyed>(geometries.len())?;
                collected.reserve_exact(geometries.len());
                collected.extend(geometries);
                current = Some((held, collected));
            }
            Some(previous) => {
                push_entry(&mut entries, previous, work)?;
                current = Some((subject, geometries));
            }
            None => current = Some((subject, geometries)),
        }
    }
    if let Some(last) = current {
        push_entry(&mut entries, last, work)?;
    }
    Ok(entries)
}

/// Canonicalize one subject's geometries and push it, unless it has none.
///
/// A spatial object with no geometry satisfies no relation's existential over
/// pairings, so it can contribute no computed row; keeping it would only inflate
/// the declared row bound.
fn push_entry(
    entries: &mut Vec<GeoEntry>,
    row: (TermValue, Vec<Keyed>),
    work: &mut ProjectionWork<'_, '_>,
) -> Result<(), GeoError> {
    let (subject, geometries) = row;
    let geometries = canonical(geometries, work)?;
    if geometries.is_empty() {
        return Ok(());
    }
    entries.push(GeoEntry {
        subject,
        geometries,
    });
    Ok(())
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
fn canonical(
    mut geometries: Vec<Keyed>,
    work: &mut ProjectionWork<'_, '_>,
) -> Result<Vec<GeometryLiteral>, GeoError> {
    purrdf_lex::walk::try_sort_unstable_by(&mut geometries, |left, right| {
        work.compare_keys(left, right)
    })?;
    purrdf_lex::walk::try_dedup_by(&mut geometries, |left, right| {
        work.compare_keys(left, right)
            .map(core::cmp::Ordering::is_eq)
    })?;
    work.reserve::<GeometryLiteral>(geometries.len())?;
    Ok(geometries.into_iter().map(|keyed| keyed.literal).collect())
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
    work: &mut ProjectionWork<'_, '_>,
) -> Result<Vec<Vec<(TermValue, TermValue)>>, GeoError> {
    work.reserve::<Vec<(TermValue, TermValue)>>(SpatialRelation::ALL.len())?;
    let mut out: Vec<Vec<(TermValue, TermValue)>> = Vec::with_capacity(SpatialRelation::ALL.len());
    for relation in SpatialRelation::ALL {
        work.tick(1)?;
        work.storage(
            (vocab.core_namespace().len() as u64)
                .checked_add(relation.local_name().len() as u64)
                .ok_or(GeoError::ArithmeticOverflow("asserted relation IRI"))?,
        )?;
        let iri = TermValue::iri(format!(
            "{}{}",
            vocab.core_namespace(),
            relation.local_name()
        ));
        let mut pairs: Vec<(TermValue, TermValue)> = Vec::new();
        if let Some(predicate) = dataset
            .term_id_by_value(&iri)
            .map_err(|error| GeoError::source_read(error.to_string()))?
        {
            for quad in dataset.quads_for_pattern(None, Some(predicate), None, graph) {
                work.tick(1)?;
                work.output_count(
                    pairs
                        .len()
                        .checked_add(1)
                        .ok_or(GeoError::ArithmeticOverflow("asserted pair count"))?,
                )?;
                if pairs.len() == pairs.capacity() {
                    work.reserve::<(TermValue, TermValue)>(1)?;
                    pairs.reserve_exact(1);
                }
                pairs.push((
                    work.resolve(dataset, quad.s)?,
                    work.resolve(dataset, quad.o)?,
                ));
            }
        }
        work.tick(0)?;
        purrdf_lex::walk::try_sort_unstable_by(&mut pairs, |a, b| compare_pair(a, b, work))?;
        purrdf_lex::walk::try_dedup_by(&mut pairs, |a, b| {
            compare_pair(a, b, work).map(core::cmp::Ordering::is_eq)
        })?;
        out.push(pairs);
    }
    Ok(out)
}

fn compare_pair(
    a: &(TermValue, TermValue),
    b: &(TermValue, TermValue),
    work: &mut ProjectionWork<'_, '_>,
) -> Result<core::cmp::Ordering, GeoError> {
    let first = work.compare_terms(&a.0, &b.0)?;
    if first.is_eq() {
        work.compare_terms(&a.1, &b.1)
    } else {
        Ok(first)
    }
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
    work: &mut ProjectionWork<'_, '_>,
) -> Result<u64, GeoError> {
    let mut digest = Digest::new();
    for property in config.serializations() {
        work.scan_term(property)?;
    }
    if let GraphSelector::Named(term) = config.graph() {
        work.scan_term(term)?;
    }
    config.absorb(&mut digest);
    digest.count(entries.len());
    for entry in entries {
        work.scan_term(&entry.subject)?;
        digest.term(&entry.subject);
        digest.count(entry.geometries.len());
        for geometry in &entry.geometries {
            let identity = exact_identity(geometry, work)?;
            work.tick((identity.len() as u64).div_ceil(16))?;
            digest.field(&identity);
        }
    }
    for (at, pairs) in asserted.iter().enumerate() {
        digest.count(at);
        digest.count(pairs.len());
        for (subject, object) in pairs {
            work.scan_term(subject)?;
            work.scan_term(object)?;
            digest.term(subject);
            digest.term(object);
        }
    }
    Ok(digest.finish())
}

/// Prove an index and a dataset are the pairing the caller intends.
///
/// # The channel this closes
///
/// the evaluator property-function adapter receives no dataset. The engine validates *registry*
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

#[cfg(test)]
mod term_walk_tests {
    //! The term digest and the value bridge against their recursive references, and the
    //! digest at a hundred thousand levels on a 128 KiB thread.

    use purrdf_core::backend::TermFactory as _;
    use purrdf_core::{
        RdfDataset, RdfDatasetBuilder, RdfTextDirection, TermBox, TermId, TermRef, TermValue,
    };

    use super::{Digest, TERM_BLANK, TERM_IRI, TERM_LITERAL, TERM_TRIPLE, resolve_value};

    /// An empty literal retains its source spelling through the value bridge.
    #[test]
    fn a_present_empty_literal_resolves_to_the_empty_string() {
        let mut builder = RdfDatasetBuilder::new();
        let holder = builder.intern_iri("http://example.org/holder");
        let datatype = "http://example.org/geo#wktLiteral";
        let value = builder.intern_value(&TermValue::typed_literal("", datatype));
        builder.push_quad(holder, holder, value, None);
        let dataset = builder.freeze().expect("one literal freezes");
        let object = dataset.quads().next().expect("one quad").o;
        assert_eq!(
            resolve_value(&*dataset, object),
            Ok(TermValue::typed_literal("", datatype))
        );
    }

    fn reference_feed(digest: &mut Digest, value: &TermValue) {
        match value {
            TermValue::Iri(iri) => {
                digest.tag(TERM_IRI);
                digest.field(iri);
            }
            TermValue::Blank { label, scope } => {
                digest.tag(TERM_BLANK);
                digest.field(label);
                digest.scope(*scope);
            }
            TermValue::Literal {
                lexical_form,
                datatype,
                language,
                direction,
            } => {
                digest.tag(TERM_LITERAL);
                digest.field(lexical_form);
                digest.field(datatype);
                digest.optional(language.as_deref());
                digest.optional(direction.map(RdfTextDirection::as_str));
            }
            TermValue::Triple { s, p, o } => {
                digest.tag(TERM_TRIPLE);
                reference_feed(digest, s);
                reference_feed(digest, p);
                reference_feed(digest, o);
            }
        }
    }

    fn reference_value(ds: &RdfDataset, id: TermId) -> TermValue {
        match ds.resolve(id) {
            TermRef::Triple { s, p, o } => TermValue::Triple {
                s: TermBox::new(reference_value(ds, s)),
                p: TermBox::new(reference_value(ds, p)),
                o: TermBox::new(reference_value(ds, o)),
            },
            _ => ds.term_value(id),
        }
    }

    /// The digest absorbs, and the bridge resolves, every generated term exactly as the
    /// recursive references do.
    #[test]
    fn the_digest_and_the_bridge_agree_with_their_recursive_references() {
        for seed in 0..300_u64 {
            let mut state = seed;
            let mut budget = 8;
            let value = purrdf_core::term_fixture::term_value(
                &mut state,
                purrdf_testkit::rng::splitmix64_next,
                &mut budget,
                purrdf_core::term_fixture::TermShape::WellFormed,
            );
            let (mut found, mut expected) = (Digest::new(), Digest::new());
            found.term(&value);
            reference_feed(&mut expected, &value);
            assert_eq!(found.finish(), expected.finish(), "seed {seed}");
            let mut builder = RdfDatasetBuilder::new();
            let object = builder.intern_value(&value);
            let holder = builder.intern_iri("http://example.org/holder");
            builder.push_quad(holder, holder, object, None);
            let ds = builder.freeze().expect("a generated term freezes");
            let object = ds.quads().next().expect("one quad").o;
            assert_eq!(
                resolve_value(&*ds, object).unwrap(),
                reference_value(&ds, object),
                "seed {seed}"
            );
        }
    }

    /// A triple term a hundred thousand levels deep is absorbed on a thread whose whole
    /// stack is 128 KiB, to a digest that tells it from one a level shallower.
    #[test]
    fn a_hundred_thousand_level_term_is_digested_on_a_128_kib_thread() {
        const LEVELS: usize = 100_000;
        purrdf_stack::on_stack(128 * 1024, || {
            let (mut deep, mut shallower) = (Digest::new(), Digest::new());
            deep.term(&purrdf_core::term_fixture::triple_chain(LEVELS));
            shallower.term(&purrdf_core::term_fixture::triple_chain(LEVELS - 1));
            assert_ne!(deep.finish(), shallower.finish());
        })
        .expect("the thread starts");
    }
}

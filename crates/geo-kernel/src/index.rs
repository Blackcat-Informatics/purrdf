// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Frozen dataset geometry projection and exact spatial-relation queries.

mod geographic;
mod projection;
pub use geographic::{GeographicGeoIndex, GeographicRelationReceipt};
use projection::ProjectionWork;

use purrdf_core::FastMap;
use purrdf_hash::{Domain, fnv};

use crate::topology::relate;
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
    #[cfg(test)]
    fn term(&mut self, value: &TermValue) {
        self.term_with(value, &mut ProjectionWork::plain())
            .expect("resident term digest");
    }
    fn term_with(
        &mut self,
        value: &TermValue,
        work: &mut ProjectionWork<'_, '_>,
    ) -> Result<(), GeoError> {
        work.absorb_term(value, |term| self.term_node(term))
    }
    fn term_node(&mut self, term: &TermValue) {
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
    fn absorb(
        &self,
        digest: &mut Digest,
        work: &mut ProjectionWork<'_, '_>,
    ) -> Result<(), GeoError> {
        digest.count(self.serializations.len());
        for property in &self.serializations {
            digest.term_with(property, work)?;
        }
        match &self.graph {
            GraphSelector::Any => digest.tag(SELECTOR_ANY),
            GraphSelector::Default => digest.tag(SELECTOR_DEFAULT),
            GraphSelector::Named(name) => {
                digest.tag(SELECTOR_NAMED);
                digest.term_with(name, work)?;
            }
        }
        Ok(())
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
    data: crate::context::Retained<GeoIndexData>,
}

#[derive(Debug)]
struct GeoIndexData {
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
        let mut work = ProjectionWork::plain();
        let value = Self::project_dataset(dataset, vocab, config, &mut work)?;
        Ok(Self {
            data: work.publish(value)?,
        })
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
        let result = Self::project_dataset(dataset, vocab, config, &mut work)
            .and_then(|value| work.publish(value))
            .map(|data| Self { data });
        match result {
            Ok(value) => match work.finish(true) {
                Ok(()) => Ok(value),
                Err(original) => {
                    drop(value);
                    let original = work.fail(original);
                    let _ = work.finish(false);
                    Err(original)
                }
            },
            Err(original) => {
                let original = work.fail(original);
                let _ = work.finish(false);
                Err(original)
            }
        }
    }

    fn project_dataset<D: DatasetView>(
        dataset: &D,
        vocab: &GeoVocab,
        config: &GeoIndexConfig,
        work: &mut ProjectionWork<'_, '_>,
    ) -> Result<GeoIndexData, GeoError> {
        let owned_config = work.config(config)?;
        dataset
            .checked_read(|dataset| {
                let Some(graph) = config
                    .graph()
                    .resolve(dataset)
                    .map_err(|error| work.source_error(error))?
                else {
                    // The configured named graph is not interned, so the dataset holds no
                    // quad in it and nothing can match. The empty projection is built
                    // through the ordinary steps — an empty entry table, one empty
                    // asserted vector per relation so `asserted` stays in bounds, and the
                    // same `fingerprint` call — rather than a second construction path
                    // that could drift from the first.
                    let entries: Vec<GeoEntry> = Vec::new();
                    let mut asserted: Vec<Vec<(TermValue, TermValue)>> = Vec::new();
                    work.reserve(&mut asserted, SpatialRelation::ALL.len())?;
                    for _ in SpatialRelation::ALL {
                        work.push(&mut asserted, Vec::new())?;
                    }
                    let source_fingerprint = fingerprint(config, &entries, &asserted, work)?;
                    return Ok(GeoIndexData {
                        config: owned_config,
                        entries,
                        asserted,
                        source_fingerprint,
                    });
                };

                // Step 1 — the geometry nodes, keyed by dataset id so step 2 can join
                // against them without resolving anything twice.
                let mut by_node: FastMap<D::Id, Vec<Keyed>> = FastMap::default();
                for property in config.serializations() {
                    work.tick(1)?;
                    let Some(predicate) = dataset
                        .term_id_by_value(property)
                        .map_err(|error| work.source_error(error))?
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
                        work.release_term(object)?;
                        let keyed = Keyed::of_with(literal, vocab, work)?;
                        let destination = work.map_entry(&mut by_node, quad.s)?;
                        work.output_count(
                            destination
                                .len()
                                .checked_add(1)
                                .ok_or(GeoError::ArithmeticOverflow("geometry count"))?,
                        )?;
                        work.push(destination, keyed)?;
                    }
                    work.tick(0)?;
                }

                // Steps 2 and 3 — every geometry node is an entry in its own right, and
                // every default-geometry subject inherits its geometries.
                let mut by_subject = FastMap::default();
                for term in [GeoTerm::HasDefaultGeometry, GeoTerm::DefaultGeometry] {
                    let iri = work.iri(format_args!(
                        "{}{}",
                        vocab.core_namespace(),
                        term.local_name()
                    ))?;
                    let predicate = dataset
                        .term_id_by_value(&iri)
                        .map_err(|error| work.source_error(error))?;
                    work.release_term(iri)?;
                    let Some(predicate) = predicate else {
                        continue;
                    };
                    for quad in dataset.quads_for_pattern(None, Some(predicate), None, graph) {
                        work.tick(1)?;
                        work.map_lookup(by_node.len())?;
                        let Some(inherited) = by_node.get(&quad.o) else {
                            continue;
                        };
                        let destination = work.map_entry(&mut by_subject, quad.s)?;
                        let required = destination.len().checked_add(inherited.len()).ok_or(
                            GeoError::Storage(purrdf_lex::allocation::StorageError::SizeOverflow),
                        )?;
                        work.reserve(destination, required)?;
                        for geometry in inherited {
                            let copy = work.clone_keyed(geometry)?;
                            work.push(destination, copy)?;
                        }
                    }
                    work.tick(0)?;
                }

                for (id, mut geometries) in by_node.drain() {
                    let destination = work.map_entry(&mut by_subject, id)?;
                    if destination.is_empty() {
                        work.release_vec(core::mem::replace(destination, geometries))?;
                    } else {
                        let required = destination.len().checked_add(geometries.len()).ok_or(
                            GeoError::Storage(purrdf_lex::allocation::StorageError::SizeOverflow),
                        )?;
                        work.reserve(destination, required)?;
                        destination.append(&mut geometries);
                        work.release_vec(geometries)?;
                    }
                }
                work.release_map(by_node)?;
                let entries = finish_entries(dataset, by_subject, work)?;
                let asserted = collect_asserted(dataset, vocab, graph, work)?;
                let source_fingerprint = fingerprint(config, &entries, &asserted, work)?;
                Ok(GeoIndexData {
                    config: owned_config,
                    entries,
                    asserted,
                    source_fingerprint,
                })
            })
            .map_err(|error| work.source_error(error))?
    }

    /// The configuration this index was built under.
    #[must_use]
    pub fn config(&self) -> &GeoIndexConfig {
        &self.data.config
    }

    /// Every indexed spatial object, sorted, with its parsed geometries.
    #[must_use]
    pub fn entries(&self) -> &[GeoEntry] {
        &self.data.entries
    }

    /// The number of indexed spatial objects.
    #[must_use]
    pub fn len(&self) -> usize {
        self.data.entries.len()
    }

    /// Whether this index holds no spatial objects.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.data.entries.is_empty()
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
        &self.data.asserted[relation_position(relation)]
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
    pub fn source_fingerprint(&self) -> u64 {
        self.data.source_fingerprint
    }

    fn holds_relation_with(
        relation: SpatialRelation,
        left: &GeoEntry,
        right: &GeoEntry,
        work: &mut impl PlanarMetric,
    ) -> Result<bool, GeoError> {
        let mut undecidable = None;
        for a in &left.geometries {
            for b in &right.geometries {
                if a.crs() != b.crs() {
                    undecidable.get_or_insert((a, b));
                    continue;
                }
                let matrix = work.matrix(a.geometry(), b.geometry())?;
                if relation.holds(
                    &matrix,
                    work.dimension(a.geometry())?,
                    work.dimension(b.geometry())?,
                ) {
                    return Ok(true);
                }
            }
        }
        // No witness. Only now does an undecidable pairing matter: without one,
        // `false` would be indistinguishable from a pairing that was never tried.
        match undecidable {
            Some((a, b)) => Err(work.crs_failure(a, b)),
            None => Ok(false),
        }
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
        let pairs = self.relation_pair_references(relation, subject, object, None)?;
        self.materialize_pairs(relation, &pairs)
    }

    /// Produce the complete sorted answer as immutable source ordinals. The
    /// original working array is admitted before birth and retained after return;
    /// source terms are borrowed rather than copied into a second query table.
    ///
    /// # Errors
    /// Preserves relation, comparison, physical and observer refusal.
    pub fn relation_pair_references(
        &self,
        relation: SpatialRelation,
        subject: Option<&TermValue>,
        object: Option<&TermValue>,
        observer: Option<&mut dyn crate::MetricWorkObserver>,
    ) -> Result<crate::PreparedValues<RelationPair>, GeoError> {
        let mut worker = PlanarPairs {
            index: self,
            relation,
            progress: crate::context::WorkProgress::integer(observer),
            context: None,
            metadata: 0,
        };
        self.relation_pairs_with(relation, subject, object, &mut worker)
    }

    /// Produce the same complete planar answer under the invoking caller's
    /// execution policy and physical allocation owner. The row ceiling does not
    /// shorten matrix work, sorting, or duplicate removal.
    ///
    /// # Errors
    /// Preserves the first exact-topology, policy, source comparison, allocation
    /// or observer refusal. No skipped CRS pairing can be reported as `false`.
    pub fn relation_pair_references_metered(
        &self,
        relation: SpatialRelation,
        subject: Option<&TermValue>,
        object: Option<&TermValue>,
        context: &mut crate::MetricContext,
        observer: &mut dyn crate::MetricWorkObserver,
    ) -> Result<crate::PreparedValues<RelationPair>, GeoError> {
        context.begin(1)?;
        let mut worker = PlanarPairs {
            index: self,
            relation,
            progress: crate::context::WorkProgress::integer(Some(observer)),
            context: Some(context),
            metadata: 0,
        };
        self.relation_pairs_with(relation, subject, object, &mut worker)
    }

    /// Borrow the original terms of a pair produced over this immutable index.
    /// Returns `None` when the source ordinal belongs to a different index.
    #[must_use]
    pub fn pair_terms(
        &self,
        relation: SpatialRelation,
        pair: RelationPair,
    ) -> Option<(&TermValue, &TermValue)> {
        match pair.source {
            PairSource::Computed { left, right } => Some((
                &self.data.entries.get(left)?.subject,
                &self.data.entries.get(right)?.subject,
            )),
            PairSource::Asserted(position) => self
                .asserted(relation)
                .get(position)
                .map(|(left, right)| (left, right)),
        }
    }

    fn materialize_pairs(
        &self,
        relation: SpatialRelation,
        pairs: &[RelationPair],
    ) -> Result<Vec<(TermValue, TermValue)>, GeoError> {
        let mut resident = purrdf_lex::allocation::Resident;
        let mut memory = purrdf_lex::allocation::Memory::new(&mut resident);
        memory.try_map(pairs.iter(), |pair, _| {
            let (left, right) = self
                .pair_terms(relation, *pair)
                .expect("pair was produced by this immutable source");
            let left = left.try_clone().map_err(|_| {
                GeoError::Storage(purrdf_lex::allocation::StorageError::AllocationFailed)
            })?;
            let right = right.try_clone().map_err(|_| {
                GeoError::Storage(purrdf_lex::allocation::StorageError::AllocationFailed)
            })?;
            Ok((left, right))
        })
    }

    fn relation_pairs_with(
        &self,
        relation: SpatialRelation,
        subject: Option<&TermValue>,
        object: Option<&TermValue>,
        worker: &mut impl PairWorker,
    ) -> Result<crate::PreparedValues<RelationPair>, GeoError> {
        use crate::context::RetainedBuilder;
        worker.bindings(subject, object)?;
        let lefts = self.candidate_positions(subject, worker)?;
        let rights = self.candidate_positions(object, worker)?;
        let mut pairs = RetainedBuilder::default();
        worker.reserve(&mut pairs, lefts.len().max(rights.len()))?;
        for left in lefts {
            for right in rights.clone() {
                if worker.holds(left, right)? {
                    worker.row(
                        &self.data.entries[left].subject,
                        &self.data.entries[right].subject,
                        pairs.len(),
                    )?;
                    let required = pairs.len().checked_add(1).ok_or(GeoError::Storage(
                        purrdf_lex::allocation::StorageError::SizeOverflow,
                    ))?;
                    worker.reserve(&mut pairs, required)?;
                    pairs.push_reserved(RelationPair {
                        source: PairSource::Computed { left, right },
                    });
                }
            }
        }
        for (position, (asserted_subject, asserted_object)) in
            self.asserted(relation).iter().enumerate()
        {
            if let Some(bound) = subject
                && !worker.compare_terms(bound, asserted_subject)?.is_eq()
            {
                continue;
            }
            if let Some(bound) = object
                && !worker.compare_terms(bound, asserted_object)?.is_eq()
            {
                continue;
            }
            worker.row(asserted_subject, asserted_object, pairs.len())?;
            let required = pairs.len().checked_add(1).ok_or(GeoError::Storage(
                purrdf_lex::allocation::StorageError::SizeOverflow,
            ))?;
            worker.reserve(&mut pairs, required)?;
            pairs.push_reserved(RelationPair {
                source: PairSource::Asserted(position),
            });
        }
        worker.finish(pairs.len())?;
        let mut compare = |left: &RelationPair, right: &RelationPair| {
            let (la, lb) = self
                .pair_terms(relation, *left)
                .expect("working pair belongs to its immutable source");
            let (ra, rb) = self
                .pair_terms(relation, *right)
                .expect("working pair belongs to its immutable source");
            let order = worker.compare_terms(la, ra)?;
            if order.is_eq() {
                worker.compare_terms(lb, rb)
            } else {
                Ok(order)
            }
        };
        purrdf_lex::walk::try_sort_unstable_by(&mut pairs, &mut compare)?;
        pairs.dedup_by(|left, right| compare(left, right).map(core::cmp::Ordering::is_eq))?;
        Ok(crate::PreparedValues::from_admitted(pairs))
    }

    fn candidate_positions(
        &self,
        bound: Option<&TermValue>,
        worker: &mut impl PairWorker,
    ) -> Result<core::ops::Range<usize>, GeoError> {
        let Some(bound) = bound else {
            return Ok(0..self.data.entries.len());
        };
        let mut failure = None;
        let found = self.data.entries.binary_search_by(|entry| {
            if failure.is_some() {
                return core::cmp::Ordering::Equal;
            }
            match worker.compare_terms(&entry.subject, bound) {
                Ok(order) => order,
                Err(error) => {
                    failure = Some(error);
                    core::cmp::Ordering::Equal
                }
            }
        });
        if let Some(error) = failure {
            return Err(error);
        }
        Ok(found.map_or(0..0, |position| position..position + 1))
    }

    /// The entry for `subject`, by binary search over the sorted entries.
    #[must_use]
    pub fn entry_of(&self, subject: &TermValue) -> Option<&GeoEntry> {
        self.data
            .entries
            .binary_search_by(|entry| entry.subject.cmp(subject))
            .ok()
            .map(|at| &self.data.entries[at])
    }
}

/// One computed or asserted answer in its original immutable source.
/// Its fields are private; no dictionary term is copied during materialization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RelationPair {
    source: PairSource,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PairSource {
    Computed { left: usize, right: usize },
    Asserted(usize),
}

trait PairWorker {
    fn bindings(
        &mut self,
        subject: Option<&TermValue>,
        object: Option<&TermValue>,
    ) -> Result<(), GeoError>;
    fn reserve(
        &mut self,
        pairs: &mut crate::context::RetainedBuilder<RelationPair>,
        required: usize,
    ) -> Result<(), GeoError>;
    fn holds(&mut self, left: usize, right: usize) -> Result<bool, GeoError>;
    fn row(&mut self, subject: &TermValue, object: &TermValue, len: usize) -> Result<(), GeoError>;
    fn finish(&mut self, count: usize) -> Result<(), GeoError>;
    fn compare_terms(
        &mut self,
        left: &TermValue,
        right: &TermValue,
    ) -> Result<core::cmp::Ordering, GeoError>;
}

trait PlanarMetric {
    fn matrix(
        &mut self,
        left: &crate::Geometry,
        right: &crate::Geometry,
    ) -> Result<crate::IntersectionMatrix, GeoError>;
    fn dimension(&mut self, value: &crate::Geometry) -> Result<i32, GeoError>;
    fn crs_failure(&mut self, left: &GeometryLiteral, right: &GeometryLiteral) -> GeoError;
}

struct PlanarPairs<'index, 'context, 'observer> {
    index: &'index GeoIndex,
    relation: SpatialRelation,
    progress: crate::context::WorkProgress<'observer>,
    context: Option<&'context mut crate::MetricContext>,
    metadata: u64,
}
impl PlanarMetric for PlanarPairs<'_, '_, '_> {
    fn matrix(
        &mut self,
        left: &crate::Geometry,
        right: &crate::Geometry,
    ) -> Result<crate::IntersectionMatrix, GeoError> {
        match self.context.as_deref_mut() {
            Some(context) => {
                crate::topology::relate_admitted(left, right, context, &mut self.progress)
            }
            None => Ok(relate(left, right)),
        }
    }
    fn dimension(&mut self, value: &crate::Geometry) -> Result<i32, GeoError> {
        self.progress.with_storage(0, |memory| {
            crate::topology::topological_dimension_with_memory(value, memory)
                .map_err(GeoError::Storage)
        })
    }
    fn crs_failure(&mut self, left: &GeometryLiteral, right: &GeometryLiteral) -> GeoError {
        left.require_same_crs_with_progress(right, &mut self.progress)
            .expect_err("the original pair's coordinate systems differ")
    }
}

impl PairWorker for PlanarPairs<'_, '_, '_> {
    fn bindings(&mut self, _: Option<&TermValue>, _: Option<&TermValue>) -> Result<(), GeoError> {
        Ok(())
    }
    fn reserve(
        &mut self,
        pairs: &mut crate::context::RetainedBuilder<RelationPair>,
        required: usize,
    ) -> Result<(), GeoError> {
        match self.context.as_deref_mut() {
            Some(context) => {
                pairs.reserve_admitted(required, context, &mut self.progress, &mut self.metadata)
            }
            None => pairs.reserve(required, &mut self.progress),
        }
    }
    fn holds(&mut self, left: usize, right: usize) -> Result<bool, GeoError> {
        let index = self.index;
        GeoIndex::holds_relation_with(
            self.relation,
            &index.data.entries[left],
            &index.data.entries[right],
            self,
        )
    }
    fn row(&mut self, _: &TermValue, _: &TermValue, len: usize) -> Result<(), GeoError> {
        if let Some(context) = self.context.as_deref_mut() {
            let limit = context.policy().limits().max_output_elements;
            if len as u64 >= limit {
                return Err(GeoError::OutputExhausted { limit });
            }
            context.charge_work(1)?;
            self.progress.context_poll(context)?;
        }
        Ok(())
    }
    fn finish(&mut self, _: usize) -> Result<(), GeoError> {
        Ok(())
    }
    fn compare_terms(
        &mut self,
        left: &TermValue,
        right: &TermValue,
    ) -> Result<core::cmp::Ordering, GeoError> {
        self.progress.with_storage(0, |memory| {
            left.try_cmp_with_memory(right, memory)
                .map_err(GeoError::Storage)
        })
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
        return Err(work.invalid_serialization(property, object));
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
#[cfg(test)]
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
    mut by_subject: FastMap<D::Id, Vec<Keyed>>,
    work: &mut ProjectionWork<'_, '_>,
) -> Result<Vec<GeoEntry>, GeoError> {
    work.output_count(by_subject.len())?;
    let mut rows = Vec::new();
    work.reserve(&mut rows, by_subject.len())?;
    for (id, geometries) in by_subject.drain() {
        let subject = work.resolve(dataset, id)?;
        work.push(&mut rows, (subject, geometries))?;
    }
    work.release_map(by_subject)?;
    purrdf_lex::walk::try_sort_unstable_by(&mut rows, |left, right| {
        work.compare_terms(&left.0, &right.0)
    })?;

    let mut entries: Vec<GeoEntry> = Vec::new();
    work.reserve(&mut entries, rows.len())?;
    let row_capacity = rows.capacity();
    let mut current: Option<(TermValue, Vec<Keyed>)> = None;
    #[expect(
        clippy::iter_with_drain,
        reason = "The original row buffer remains covered until the native release_vec call after all rows have moved or died"
    )]
    for (subject, mut geometries) in rows.drain(..) {
        match current.take() {
            // Two ids resolving to one value cannot happen in an interned
            // dataset, but coalescing costs one comparison and keeps the
            // binary-search precondition true by construction rather than by
            // assumption.
            Some((held, mut collected)) if work.compare_terms(&held, &subject)?.is_eq() => {
                let needed =
                    collected
                        .len()
                        .checked_add(geometries.len())
                        .ok_or(GeoError::Storage(
                            purrdf_lex::allocation::StorageError::SizeOverflow,
                        ))?;
                work.reserve(&mut collected, needed)?;
                collected.append(&mut geometries);
                work.release_vec(geometries)?;
                work.release_term(subject)?;
                current = Some((held, collected));
            }
            Some(previous) => {
                push_entry(&mut entries, previous, work)?;
                current = Some((subject, geometries));
            }
            None => current = Some((subject, geometries)),
        }
    }
    drop(rows);
    work.release_vector_layout::<(TermValue, Vec<Keyed>)>(row_capacity)?;
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
    work.push(
        entries,
        GeoEntry {
            subject,
            geometries,
        },
    )
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
    let kept = purrdf_lex::walk::try_partition_unique_by(&mut geometries, |left, right| {
        work.compare_keys(left, right)
            .map(core::cmp::Ordering::is_eq)
    })?;
    while geometries.len() > kept {
        work.release_keyed(geometries.pop().expect("a duplicate geometry"))?;
    }
    let mut output = Vec::new();
    work.reserve(&mut output, geometries.len())?;
    #[expect(
        clippy::iter_with_drain,
        reason = "The original keyed buffer remains covered until release_vec destroys it after cell publication"
    )]
    for keyed in geometries.drain(..) {
        work.release_string(keyed.key)?;
        work.push(&mut output, keyed.literal)?;
    }
    work.release_vec(geometries)?;
    Ok(output)
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
    let mut out: Vec<Vec<(TermValue, TermValue)>> = Vec::new();
    work.reserve(&mut out, SpatialRelation::ALL.len())?;
    for relation in SpatialRelation::ALL {
        work.tick(1)?;
        let iri = work.relation_iri(vocab, relation)?;
        let mut pairs: Vec<(TermValue, TermValue)> = Vec::new();
        if let Some(predicate) = dataset
            .term_id_by_value(&iri)
            .map_err(|error| work.source_error(error))?
        {
            for quad in dataset.quads_for_pattern(None, Some(predicate), None, graph) {
                work.tick(1)?;
                work.output_count(
                    pairs
                        .len()
                        .checked_add(1)
                        .ok_or(GeoError::ArithmeticOverflow("asserted pair count"))?,
                )?;
                let subject = work.resolve(dataset, quad.s)?;
                let object = work.resolve(dataset, quad.o)?;
                work.push(&mut pairs, (subject, object))?;
            }
        }
        work.release_term(iri)?;
        work.tick(0)?;
        purrdf_lex::walk::try_sort_unstable_by(&mut pairs, |a, b| compare_pair(a, b, work))?;
        let kept = purrdf_lex::walk::try_partition_unique_by(&mut pairs, |a, b| {
            compare_pair(a, b, work).map(core::cmp::Ordering::is_eq)
        })?;
        while pairs.len() > kept {
            let (subject, object) = pairs.pop().expect("a duplicate asserted pair");
            work.release_term(subject)?;
            work.release_term(object)?;
        }
        work.push(&mut out, pairs)?;
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
    config.absorb(&mut digest, work)?;
    digest.count(entries.len());
    for entry in entries {
        digest.term_with(&entry.subject, work)?;
        digest.count(entry.geometries.len());
        for geometry in &entry.geometries {
            let identity = exact_identity(geometry, work)?;
            work.tick((identity.len() as u64).div_ceil(16))?;
            digest.field(&identity);
            work.release_string(identity)?;
        }
    }
    for (at, pairs) in asserted.iter().enumerate() {
        digest.count(at);
        digest.count(pairs.len());
        for (subject, object) in pairs {
            digest.term_with(subject, work)?;
            digest.term_with(object, work)?;
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
        let holder = builder.intern_iri("http://example.org/holder").unwrap();
        let datatype = "http://example.org/geo#wktLiteral";
        let value = builder
            .intern_value(&TermValue::typed_literal("", datatype))
            .unwrap();
        builder.push_quad(holder, holder, value, None).unwrap();
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
            let object = builder.intern_value(&value).unwrap();
            let holder = builder.intern_iri("http://example.org/holder").unwrap();
            builder.push_quad(holder, holder, object, None).unwrap();
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

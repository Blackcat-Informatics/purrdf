// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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

use crate::{
    EvalError, PfArgs, PfArity, PfCursor, PfRow, PropertyFunction, PropertyFunctionRegistry,
    Volatility,
};
use purrdf_core::TermValue;
use purrdf_core::binding_pattern::BindingPattern;
pub use purrdf_geo_kernel::index::{
    GeoEntry, GeoIndex, GeoIndexConfig, GeographicGeoIndex, GraphSelector, verify_binding,
};
use purrdf_geo_kernel::{
    ExecutionPolicy, GeoError, GeoVocab, MetricContext, MetricWorkContinuation, MetricWorkObserver,
    RelationFamily, SpatialRelation,
};
use std::sync::Arc;

/// The subject-side flattened position of a Query Rewrite call.
const SUBJECT: usize = 0;
/// The object-side flattened position of a Query Rewrite call.
const OBJECT: usize = 1;
/// The single access pattern a Query Rewrite relation declares.
const ALL_FREE_MODE: &str = "ff";
/// The declared arity of every Query Rewrite relation.
const REWRITE_ARITY: PfArity = PfArity::new(1, 1);

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
    index: RelationIndex,
    generation: Option<Arc<str>>,
    /// Which of the twenty-four relations this is.
    relation: SpatialRelation,
    /// The single declared mode, materialized once so [`PropertyFunction::modes`]
    /// can hand out a slice.
    modes: [BindingPattern; 1],
    /// The row maxima, measured once at construction.
    bounds: RowBounds,
}

#[derive(Clone, Debug)]
enum RelationIndex {
    Planar(Arc<GeoIndex>),
    Geographic(Arc<GeographicGeoIndex>, ExecutionPolicy),
}
impl RelationIndex {
    fn source(&self) -> &GeoIndex {
        match self {
            Self::Planar(index) => index,
            Self::Geographic(index, _) => index.source(),
        }
    }
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
            index: RelationIndex::Planar(index),
            generation: None,
            relation,
            modes: [BindingPattern::from_code(ALL_FREE_MODE)],
            bounds,
        }
    }

    /// Query the original prepared geographic laws on their actual target surface.
    /// Admission is explicit and does not change the immutable index identity.
    #[must_use]
    pub fn new_geographic(
        index: Arc<GeographicGeoIndex>,
        relation: SpatialRelation,
        policy: ExecutionPolicy,
    ) -> Self {
        let bounds = RowBounds::of(index.source(), relation);
        let generation = Some(Arc::from(index.id().to_string()));
        Self {
            index: RelationIndex::Geographic(index, policy),
            generation,
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
        self.index.source()
    }

    /// Adapt the kernel's complete sorted relation answer to evaluator rows.
    fn rows(
        &self,
        args: &PfArgs<'_>,
        meter: Option<&crate::NativeFnContext<'_>>,
    ) -> Result<(Vec<PfRow>, u64), GeoError> {
        match &self.index {
            RelationIndex::Planar(index) => Ok((
                pair_rows(
                    index.relation_pairs(self.relation, args.get(SUBJECT), args.get(OBJECT))?,
                    |_| Ok(()),
                )?,
                0,
            )),
            RelationIndex::Geographic(index, policy) => {
                let mut observer = meter.map(super::functions::GeoWork);
                let mut context = match observer.as_mut() {
                    Some(observer) => {
                        MetricContext::from_reference_metered(index.reference(), *policy, observer)?
                    }
                    None => MetricContext::from_reference(index.reference(), *policy)?,
                };
                let (pairs, receipt) = match observer.as_mut() {
                    Some(observer) => {
                        let mut continuation = MetricWorkContinuation::new(
                            observer,
                            context.work_items(),
                            context.workspace_peak(),
                        );
                        index.relation_pairs_with_receipt_metered(
                            self.relation,
                            args.get(SUBJECT),
                            args.get(OBJECT),
                            &mut context,
                            &mut continuation,
                        )?
                    }
                    None => index.relation_pairs_with_receipt(
                        self.relation,
                        args.get(SUBJECT),
                        args.get(OBJECT),
                        &mut context,
                    )?,
                };
                let bytes = pairs
                    .len()
                    .checked_mul(size_of::<PfRow>() + 2 * size_of::<TermValue>())
                    .and_then(|bytes| u64::try_from(bytes).ok())
                    .ok_or(GeoError::ArithmeticOverflow("geographic evaluator rows"))?;
                let complete_peak = receipt.workspace_peak.checked_add(bytes).ok_or(
                    GeoError::ArithmeticOverflow("geographic evaluator workspace"),
                )?;
                context.admit_workspace(
                    complete_peak.saturating_sub(context.retained_workspace_bytes()),
                )?;
                let mut growth = bytes;
                let rows = pair_rows(pairs, |count| {
                    context.charge_work(count)?;
                    if let Some(observer) = observer.as_mut() {
                        observer.charge_chunk(count, core::mem::take(&mut growth))?;
                    }
                    context.checkpoint()
                })?;
                Ok((rows, context.work_items()))
            }
        }
    }

    fn open_inner(
        &self,
        args: &PfArgs<'_>,
        ceiling: Option<u64>,
        meter: Option<&crate::NativeFnContext<'_>>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        let supplied = args.arity();
        if supplied != REWRITE_ARITY {
            return Err(EvalError::function(format!(
                "the GeoSPARQL Query Rewrite relation geo:{} expects {REWRITE_ARITY} argument(s), got {supplied}",
                self.relation.local_name()
            )));
        }
        let (rows, work) = self.rows(args, meter)?;
        Ok(Box::new(GeoCursor {
            rows,
            at: 0,
            bound: args.flattened().map(<Option<&TermValue>>::cloned).collect(),
            remaining: ceiling,
            generation: self.generation.clone(),
            pending_work: if meter.is_some() { 0 } else { work },
        }))
    }
}

fn pair_rows(
    pairs: Vec<(TermValue, TermValue)>,
    mut before_chunk: impl FnMut(u64) -> Result<(), GeoError>,
) -> Result<Vec<PfRow>, GeoError> {
    before_chunk(0)?;
    let mut rows = Vec::with_capacity(pairs.len());
    let mut pairs = pairs.into_iter();
    while pairs.len() != 0 {
        let count = pairs.len().min(64);
        before_chunk(count as u64)?;
        for (left, right) in pairs.by_ref().take(count) {
            rows.push(vec![left, right]);
        }
    }
    Ok(rows)
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
        self.open_inner(args, ceiling, None)
    }

    fn open_metered(
        &self,
        args: &PfArgs<'_>,
        ceiling: Option<u64>,
        context: &crate::NativeFnContext<'_>,
    ) -> Result<Box<dyn PfCursor>, EvalError> {
        self.open_inner(args, ceiling, Some(context))
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
    generation: Option<Arc<str>>,
    pending_work: u64,
}

impl PfCursor for GeoCursor {
    fn generation(&self) -> crate::IndexGeneration {
        self.generation
            .as_ref()
            .map_or(crate::IndexGeneration::Undeclared, |generation| {
                crate::IndexGeneration::declared(Arc::clone(generation))
            })
    }
    fn take_work(&mut self) -> u64 {
        core::mem::take(&mut self.pending_work)
    }
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
    register_with(registry, vocab, families, |relation| {
        GeoRelation::new(Arc::clone(index), relation)
    })
}

/// Register the same relation families over original prepared geographic sets.
/// # Errors
/// Refuses an empty family selection as [`register`] does.
pub fn register_geographic(
    registry: &mut PropertyFunctionRegistry,
    vocab: &GeoVocab,
    index: &Arc<GeographicGeoIndex>,
    policy: ExecutionPolicy,
    families: &[RelationFamily],
) -> Result<(), GeoError> {
    register_with(registry, vocab, families, |relation| {
        GeoRelation::new_geographic(Arc::clone(index), relation, policy)
    })
}

fn register_with(
    registry: &mut PropertyFunctionRegistry,
    vocab: &GeoVocab,
    families: &[RelationFamily],
    mut create: impl FnMut(SpatialRelation) -> GeoRelation,
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
        registry.register(iri, Arc::new(create(relation)));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{EvalError, PfArgs, PfRow, PropertyFunction, PropertyFunctionRegistry, Volatility};
    use purrdf_core::binding_pattern::BindingPattern;
    use purrdf_core::{RdfDataset, RdfDatasetBuilder, RdfLiteral, TermValue};

    use super::{
        GeoEntry, GeoIndex, GeoIndexConfig, GeoRelation, GraphSelector, register, verify_binding,
    };
    use purrdf_geo_kernel::GeoError;
    use purrdf_geo_kernel::geom::Crs;
    use purrdf_geo_kernel::relations::{RelationFamily, SpatialRelation};
    use purrdf_geo_kernel::vocab::{GeoVocab, GeoVocabBuilder};

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

    /// The same serialization set, scoped to the named graph [`GRAPH`].
    fn named_config() -> GeoIndexConfig {
        GeoIndexConfig::new(
            vec![TermValue::iri(geo("asWKT"))],
            GraphSelector::Named(TermValue::iri(GRAPH)),
        )
        .expect("an IRI selector")
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

    /// A view whose geometry literals name a non-IRI datatype — ids foreign to it —
    /// refuses the build; the same dataset through a view that answers its own
    /// datatypes builds.
    #[test]
    fn a_view_handing_back_a_foreign_id_refuses_the_build() {
        let dataset = dataset_of(&four_branch_rows());
        let not_an_iri = dataset
            .quads()
            .map(|quad| quad.o)
            .find(|&id| matches!(dataset.resolve(id), purrdf_core::TermRef::Literal { .. }))
            .expect("the fixture holds a literal");
        let view = |foreign| purrdf_core::term_fixture::ForeignDatatypeView {
            inner: Arc::clone(&dataset),
            datatype: not_an_iri,
            foreign,
        };
        let refused = GeoIndex::from_dataset(&view(true), &vocab(), &config());
        assert!(
            matches!(refused, Err(GeoError::Config(ref message)) if message.contains("does not name a term")),
            "{refused:?}"
        );
        let built = GeoIndex::from_dataset(&view(false), &vocab(), &config())
            .expect("the view answers its own ids");
        assert_eq!(built.len(), index_of(&four_branch_rows()).len());
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

    /// The source fingerprint is frozen over a fixed dataset: `verify_binding` compares
    /// it across runs and releases, so a moved value reports an identical dataset as a
    /// different one.
    #[test]
    fn the_source_fingerprint_is_frozen() {
        let index = index_of(&four_branch_rows());
        assert_eq!(
            format!("{:#018x}", index.source_fingerprint()),
            "0x21e852458fc234f7"
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

    /// A named graph selector must hold an IRI. The neighbouring valid case — the
    /// same list under an IRI selector, over a dataset that holds the graph — is
    /// exercised alongside it.
    #[test]
    fn a_named_graph_selector_must_hold_an_iri_and_an_iri_selector_is_accepted() {
        assert!(matches!(
            GeoIndexConfig::new(
                vec![TermValue::iri(geo("asWKT"))],
                GraphSelector::Named(TermValue::simple_literal("g1"))
            ),
            Err(GeoError::Config(_))
        ));

        let in_graph = dataset_in(&four_branch_rows(), Some(GRAPH));
        let index = GeoIndex::from_dataset(&*in_graph, &vocab(), &named_config())
            .expect("the graph exists");
        assert_eq!(index.len(), 4, "and it indexes the graph's spatial objects");
    }

    /// A graph the dataset has not interned yields the **empty index** rather than
    /// a refusal. A graph IRI is interned only once a quad is in that graph, so
    /// refusing here would mean an index could never stand ready before the data
    /// it will hold arrives — and the same function already reads an absent
    /// serialization property as an ordinary empty match.
    ///
    /// The empty index is a real index: it answers every relation with no asserted
    /// pairs, and it carries a genuine fingerprint rather than a placeholder.
    #[test]
    fn a_graph_the_dataset_has_not_interned_builds_an_empty_index() {
        let default_graph = dataset_of(&four_branch_rows());
        let index = GeoIndex::from_dataset(&*default_graph, &vocab(), &named_config())
            .expect("an absent graph is a corpus state, not a wiring fault");

        assert_eq!(index.len(), 0, "nothing is in a graph that is not there");
        assert!(index.is_empty());
        assert_ne!(
            index.source_fingerprint(),
            0,
            "an empty index still attests a generation"
        );
        for relation in SpatialRelation::ALL {
            assert!(
                index.asserted(relation).is_empty(),
                "{relation:?} must answer with no asserted pairs, in bounds"
            );
        }
    }

    /// Two empty indexes under **different** configurations have different
    /// fingerprints, so the digest still distinguishes configurations rather than
    /// collapsing to one constant for every empty index.
    #[test]
    fn two_empty_indexes_under_different_configurations_disagree_on_their_fingerprint() {
        let dataset = dataset_of(&four_branch_rows());
        let other = GeoIndexConfig::new(
            vec![TermValue::iri(geo("asWKT"))],
            GraphSelector::Named(iri("g2")),
        )
        .expect("an IRI selector");

        let one = GeoIndex::from_dataset(&*dataset, &vocab(), &named_config()).expect("empty");
        let two = GeoIndex::from_dataset(&*dataset, &vocab(), &other).expect("empty");

        assert!(one.is_empty() && two.is_empty(), "both hold nothing");
        assert_ne!(
            one.source_fingerprint(),
            two.source_fingerprint(),
            "the configuration is digested before any content"
        );
    }

    /// The same configuration, once the data lands in that graph, builds non-empty,
    /// retrieves its geometries, and has a fingerprint different from the empty
    /// one. This is the direction the old refusal hid: the empty index is a stage
    /// on the way to this one, not a wrong answer standing in for it.
    #[test]
    fn the_same_configuration_builds_non_empty_once_the_data_lands_in_that_graph() {
        let before =
            GeoIndex::from_dataset(&*dataset_of(&four_branch_rows()), &vocab(), &named_config())
                .expect("empty before the data lands");

        let after = GeoIndex::from_dataset(
            &*dataset_in(&four_branch_rows(), Some(GRAPH)),
            &vocab(),
            &named_config(),
        )
        .expect("populated once it has");

        assert_eq!(after.len(), 4, "the graph's four spatial objects");
        let point = after
            .entry_of(&iri("gp"))
            .expect("the point is an entry in its own right");
        assert_eq!(
            point.geometries().len(),
            1,
            "and its geometry came back with it"
        );
        assert_ne!(
            before.source_fingerprint(),
            after.source_fingerprint(),
            "the fingerprint moves the moment the first geometry lands"
        );
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

    #[test]
    fn geographic_property_calls_share_physical_rows_and_governed_refusals() {
        use purrdf_geo_kernel::{GeoProfile, MetricContext};
        use purrdf_iri::vocab::ogc;
        let dataset = dataset_of(&[
            row(
                "a",
                ogc::geo::AS_WKT.to_owned(),
                Obj::Lit("POINT(90 90)".to_owned(), ogc::geo::WKT_LITERAL.to_owned()),
            ),
            row(
                "b",
                ogc::geo::AS_WKT.to_owned(),
                Obj::Lit("POINT(-90 90)".to_owned(), ogc::geo::WKT_LITERAL.to_owned()),
            ),
        ]);
        let config =
            GeoIndexConfig::new(vec![TermValue::iri(ogc::geo::AS_WKT)], GraphSelector::Any)
                .unwrap();
        let source = Arc::new(
            GeoIndex::from_dataset(&*dataset, purrdf_geo_kernel::standard_vocabulary(), &config)
                .unwrap(),
        );
        let profile = GeoProfile::standard();
        let mut context = MetricContext::wgs84().unwrap();
        let index = Arc::new(
            super::GeographicGeoIndex::prepare(source, &profile, crs(ogc::CRS84), &mut context)
                .unwrap(),
        );
        let relation = GeoRelation::new_geographic(
            Arc::clone(&index),
            SpatialRelation::SfEquals,
            profile.policy(),
        );
        assert_eq!(
            pairs(&relation),
            vec![
                pair("a", "a"),
                pair("a", "b"),
                pair("b", "a"),
                pair("b", "b")
            ]
        );
        let free = [None];
        let mut cursor = relation.open(&PfArgs::new(&free, &free), None).unwrap();
        assert_eq!(
            cursor.generation(),
            crate::IndexGeneration::declared(index.id().to_string())
        );
        assert!(cursor.take_work() > 0);
        assert_eq!(cursor.take_work(), 0);
        let mut registry = PropertyFunctionRegistry::new();
        super::register_geographic(
            &mut registry,
            purrdf_geo_kernel::standard_vocabulary(),
            &index,
            profile.policy(),
            &[RelationFamily::SimpleFeatures],
        )
        .unwrap();
        let env = crate::ExtensionEnv::over_relations(registry).unwrap();
        let engine = crate::NativeSparqlEngine::new();
        let query = format!("SELECT ?a ?b WHERE {{ ?a <{}> ?b }}", ogc::geo::SF_EQUALS);
        let run = |fuel| {
            let governors = crate::QueryGovernors::UNBOUNDED.with_fuel(fuel);
            let state = Arc::new(crate::governor::GovernorState::new(&governors));
            engine
                .query_governed_in_operation(
                    &*dataset,
                    purrdf_core::SparqlRequest {
                        query: &query,
                        base_iri: None,
                        substitutions: &[],
                    },
                    crate::QueryOptions {
                        env: &env,
                        ..crate::QueryOptions::EMPTY
                    },
                    &state,
                )
                .unwrap()
        };
        let complete = run(4_000_000);
        assert!(complete.tripped().is_none());
        let crate::GovernedOutcome::Complete {
            result: purrdf_core::SparqlResult::Solutions { rows, .. },
            ..
        } = complete
        else {
            panic!("complete geographic property rows")
        };
        assert_eq!(rows.len(), 4);
        assert!(matches!(
            run(1_000),
            crate::GovernedOutcome::BudgetExhausted(_)
        ));
    }
}

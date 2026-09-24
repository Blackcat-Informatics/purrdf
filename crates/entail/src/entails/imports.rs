// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! `owl:imports`: the documents a premise says it is not all of.
//!
//! # Why this is a HARD failure and not a boundary
//!
//! `owl:imports` is not a hint. OWL 2 defines the imports closure of an ontology to BE the
//! ontology for every semantic purpose: an axiom in an imported document constrains the
//! importing one exactly as if it had been written there. So a reasoner handed a premise
//! that imports a document it does not have is not reasoning over a slightly smaller
//! premise — it is reasoning over a DIFFERENT premise, and every answer it gives is about
//! that different one.
//!
//! Two answers are then possible, and only one of them is honest:
//!
//! * "not entailed", because the missing axioms were the ones that would have derived the
//!   conclusion. This is a false negative that no report line can undo — the caller asked a
//!   question about their ontology and got an answer about a truncation of it.
//! * a refusal, naming the document that is missing.
//!
//! This module refuses. [`EntailError::UnresolvedImport`] carries the IRI, so the caller
//! learns what to hand over rather than that "something" was incomplete. The same rule is
//! why a resolved import is MERGED into the premise before anything else happens rather
//! than consulted afterwards: an imported axiom has to be able to participate in a rule
//! body beside an importing one, which it can only do if the chase sees one graph.
//!
//! # An ontology already in the graph is not missing
//!
//! A premise that ALREADY holds the ontology it imports — its `owl:Ontology` header, or an
//! ontology whose `owl:versionIRI` is the imported IRI — has that document's axioms in hand,
//! and refusing it would refuse the closure the import asked for. So does a premise that
//! imports the very document it was read from ([`ImportMap::declare_loaded`]). [`unresolved_imports`]
//! states the rule once, and every host that reads `owl:imports` (this crate's `entails`,
//! the CLI's shapes lanes, the prepared-shapes-product packer) takes its verdict from it.
//!
//! # The closure is transitive, because the specification's is
//!
//! An imported document may import further documents, and OWL 2's imports closure is the
//! transitive one. The resolution below is therefore a work-list to a fixpoint over the
//! import graph, visiting each document once — which also makes a cyclic import (`A`
//! imports `B` imports `A`, which OWL 2 explicitly permits) terminate rather than loop.
//!
//! # Blank nodes are standardized apart, and the premise's are not moved
//!
//! Merging two RDF documents is an RDF MERGE: `_:b` in one and `_:b` in the other are
//! different nodes, and conflating them would invent identities the author never asserted.
//! Each imported document is therefore copied under a scope of its own (`purrdf` C0.2).
//!
//! The premise keeps its ORIGINAL scopes, which is not symmetry-breaking for its own sake:
//! an [`EntailmentWarrant`](super::warrant::EntailmentWarrant) is re-checked against the
//! premise the caller passed, and a check that had to know which fresh scope the premise's
//! blank nodes were moved to would be re-deriving the merge instead of reading the premise.
//! Imported scopes are allocated strictly above every scope the premise uses, so the
//! standardize-apart property holds in both directions.
//!
//! # ONE import concept for the crate
//!
//! This crate already had a caller-owns-the-I/O import discipline before this module:
//! [`resolve_rif_imports`](crate::resolve_rif_imports) takes a
//! [`crate::RifImport`]'s location and a resolver CALLBACK, and the library
//! fetches nothing. [`ImportMap`] is the same discipline in table form, and
//! [`ImportMap::rif_resolver`] is the bridge: one map of caller-supplied documents serves
//! both, so a caller that already declared what its ontology IRIs denote does not declare it
//! twice.
//!
//! What the bridge does NOT do is pretend the two resolutions are one operation, because
//! they are not. A RIF import names an entailment PROFILE and contributes the FACTS of the
//! closure computed under it; an `owl:imports` names an ontology and contributes its AXIOMS
//! verbatim, with the closure computed after the merge. Collapsing those would change what
//! one of them means, so what is shared is the configuration and the no-I/O rule, and the
//! two consumers stay separate.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::sync::Arc;

use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder, TermId, TermValue};

use crate::EntailError;
use crate::rif_xml::RifImport;
use crate::vocab::{OWL_IMPORTS, OWL_ONTOLOGY, OWL_VERSIONIRI, RDF_TYPE};

/// The documents an `owl:imports` resolves to.
///
/// PurRDF mints no vocabulary and fetches nothing: it has no notion of what an ontology IRI
/// dereferences to, and inventing one would make an entailment depend on the network. So
/// the import closure is **caller-supplied configuration**, exactly like every other
/// vocabulary this library reads, and a premise that imports a document the caller did not
/// supply — and does not already contain, see [`unresolved_imports`] — is a hard error
/// rather than a silently truncated premise.
///
/// ```
/// use purrdf_core::RdfDatasetBuilder;
/// use purrdf_entail::{EntailError, ImportMap, Regime, entails};
///
/// let mut b = RdfDatasetBuilder::new();
/// let ontology = b.intern_iri("http://example.org/o");
/// let imports = b.intern_iri("http://www.w3.org/2002/07/owl#imports");
/// let other = b.intern_iri("http://example.org/other");
/// b.push_quad(ontology, imports, other, None);
/// let premise = b.freeze().expect("freeze");
/// let conclusion = RdfDatasetBuilder::new().freeze().expect("freeze");
///
/// // An import nobody supplied is a refusal that NAMES the document.
/// let error = entails(&premise, &conclusion, Regime::OwlRl, &ImportMap::new()).unwrap_err();
/// assert!(matches!(error, EntailError::UnresolvedImport(ref iri) if iri == "http://example.org/other"));
/// ```
#[derive(Debug, Clone, Default)]
pub struct ImportMap {
    /// Ontology IRI → the document it names.
    documents: BTreeMap<String, Arc<RdfDataset>>,
    /// The IRIs of the documents the PREMISE itself was read from — its retrieval IRI or
    /// base. An import of one of these names a document already in hand.
    loaded: BTreeSet<String>,
}

impl ImportMap {
    /// An import map that resolves nothing.
    ///
    /// The right value for the overwhelmingly common premise that imports nothing, and the
    /// wrong one for a premise that imports something — which is why the difference is an
    /// error rather than a default.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Declare that `iri` names `document`, returning whatever it named before.
    pub fn insert(
        &mut self,
        iri: impl Into<String>,
        document: Arc<RdfDataset>,
    ) -> Option<Arc<RdfDataset>> {
        self.documents.insert(iri.into(), document)
    }

    /// The document `iri` names, if this map has one.
    #[must_use]
    pub fn get(&self, iri: &str) -> Option<&Arc<RdfDataset>> {
        self.documents.get(iri)
    }

    /// How many documents this map resolves.
    #[must_use]
    pub fn len(&self) -> usize {
        self.documents.len()
    }

    /// This map as a resolver for [`resolve_rif_imports`](crate::resolve_rif_imports).
    ///
    /// A [`RifImport`]'s `location` is looked up exactly as an `owl:imports` object is, and an
    /// unresolved one refuses by name through the SAME error. See the [module docs](self) for
    /// why the two resolutions share their configuration and not their semantics.
    ///
    /// ```
    /// use purrdf_core::RdfDatasetBuilder;
    /// use purrdf_entail::{EntailError, ImportMap, RifImport};
    ///
    /// let mut b = RdfDatasetBuilder::new();
    /// let s = b.intern_iri("http://example.org/s");
    /// let p = b.intern_iri("http://example.org/p");
    /// let o = b.intern_iri("http://example.org/o");
    /// b.push_quad(s, p, o, None);
    /// let document = b.freeze().expect("freeze");
    ///
    /// let mut map = ImportMap::new();
    /// map.insert("http://example.org/lib", document);
    /// let mut resolve = map.rif_resolver();
    ///
    /// let known = RifImport { location: "http://example.org/lib".to_owned(), profile: None };
    /// assert!(resolve(&known).is_ok());
    /// let unknown = RifImport { location: "http://example.org/other".to_owned(), profile: None };
    /// assert!(matches!(
    ///     resolve(&unknown),
    ///     Err(EntailError::UnresolvedImport(ref iri)) if iri == "http://example.org/other"
    /// ));
    /// ```
    pub fn rif_resolver(&self) -> impl FnMut(&RifImport) -> Result<Arc<RdfDataset>, EntailError> {
        move |import: &RifImport| {
            self.get(&import.location)
                .map(Arc::clone)
                .ok_or_else(|| EntailError::UnresolvedImport(import.location.clone()))
        }
    }

    /// Whether this map resolves no document at all.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    /// Declare that the premise was itself read from the document at `iri` — its retrieval
    /// IRI, or the base it was parsed under — returning whether the IRI was new.
    ///
    /// An `owl:imports <iri>` then names a document that is already loaded, so it is
    /// resolved without a merge. The case is not exotic: SHACL collects `sh:declare`
    /// prefixes along `sh:prefixes/owl:imports*`, and a document routinely points that path
    /// at its own IRI. A caller that does not know where the premise came from declares
    /// nothing, and an import of that document then refuses by name exactly as before.
    pub fn declare_loaded(&mut self, iri: impl Into<String>) -> bool {
        self.loaded.insert(iri.into())
    }

    /// Every ontology IRI in `premise`'s transitive `owl:imports` closure that neither this
    /// map nor the closure itself resolves, deduplicated, in the order the closure walk
    /// first meets them.
    ///
    /// An import is resolved when this map supplies its document, when it names a document
    /// the premise was read from (see [`ImportMap::declare_loaded`]), or when the closure
    /// already CONTAINS the ontology it names — see [`unresolved_imports`] for that rule and
    /// why it is the right one. A document this map supplies contributes its own imports
    /// and its own ontology declarations to the closure, so the walk is to a fixpoint and a
    /// cycle terminates.
    ///
    /// An empty answer is exactly the condition under which [`entails`](crate::entails())
    /// accepts the premise's imports; the first element of a non-empty one is the IRI its
    /// [`EntailError::UnresolvedImport`] names.
    ///
    /// ```
    /// use purrdf_core::RdfDatasetBuilder;
    /// use purrdf_entail::ImportMap;
    ///
    /// let mut b = RdfDatasetBuilder::new();
    /// let ontology = b.intern_iri("http://example.org/o");
    /// let imports = b.intern_iri("http://www.w3.org/2002/07/owl#imports");
    /// let other = b.intern_iri("http://example.org/other");
    /// b.push_quad(ontology, imports, other, None);
    /// let premise = b.freeze().expect("freeze");
    ///
    /// assert_eq!(
    ///     ImportMap::new().unresolved_imports(&premise),
    ///     vec!["http://example.org/other".to_owned()]
    /// );
    /// ```
    #[must_use]
    pub fn unresolved_imports(&self, premise: &RdfDataset) -> Vec<String> {
        walk_closure(premise, self).unresolved
    }
}

/// Every ontology IRI in `ds`'s `owl:imports` closure that `ds` does not itself contain,
/// deduplicated, in the order the closure walk first meets them.
///
/// `loaded` is the IRIs of the documents `ds` was read from: each one's retrieval IRI or
/// the base it was parsed under. Pass an empty slice when that is unknown.
///
/// # The rule
///
/// An `owl:imports <X>` is RESOLVED when the document or ontology `X` names is already in
/// hand:
///
/// * `X` is in `loaded` — the import names a document `ds` was read from. SHACL collects
///   `sh:declare` prefixes along `sh:prefixes/owl:imports*`, and a document routinely points
///   that path at its OWN IRI from a node that is not an `owl:Ontology` at all;
/// * `X rdf:type owl:Ontology` — the ontology header OWL 2's RDF mapping reads an ontology's
///   IRI from — or
/// * some `O owl:versionIRI X` — an ontology whose VERSION IRI is `X`. OWL 2 §3.2 makes a
///   version IRI a name the ontology may be imported by, and `owl:versionIRI`'s domain is
///   `owl:Ontology`, so its subject is an ontology whether or not it is also typed as one.
///
/// Everything else is unresolved: PurRDF fetches nothing, so an import whose ontology is not
/// in hand is a document the caller has to supply (an [`ImportMap`] entry, or the CLI's
/// `--import IRI=FILE`).
///
/// # Why presence counts
///
/// Merging a vocabulary INTO the graph that imports it is how an `owl:imports` is resolved by
/// hand, and it is what a caller who read the W3C SHACL 1.2 vocabularies together has done:
/// `shnex.ttl` imports `sh:`, and `shacl.ttl` is the document whose header declares it.
/// Refusing that graph as incomplete would refuse the very closure the import asked for.
/// The closure is transitive by construction: a resolved ontology's own imports are triples
/// of the same graph, so they are among the imports this function checks.
///
/// Only IRIs count on both sides, for the reason [`imported_iris`] gives.
#[must_use]
pub fn unresolved_imports(ds: &RdfDataset, loaded: &[&str]) -> Vec<String> {
    let mut map = ImportMap::new();
    for iri in loaded {
        map.declare_loaded(*iri);
    }
    map.unresolved_imports(ds)
}

/// One walk of an import closure: the documents a map supplied, in walk order, and the
/// imports nothing resolved.
struct ClosureWalk {
    /// Each document the map supplied, once, in the order the walk reached it.
    documents: Vec<Arc<RdfDataset>>,
    /// Each import neither the map nor any graph of the closure resolves, once, in the
    /// order the walk first met it.
    unresolved: Vec<String>,
}

/// Walk `premise`'s `owl:imports` closure against `map`, to a fixpoint.
///
/// Breadth-first, each IRI visited once, so a CYCLE (`A` imports `B` imports `A`, which OWL 2
/// §3.4 permits) terminates rather than looping. A map-supplied document wins over an
/// in-graph declaration of the same ontology: the caller named that document, and merging it
/// is what the caller asked for. Whether an unsupplied import is declared in the closure is
/// decided only once the walk is complete, because a document reached LATER may be the one
/// that declares it.
fn walk_closure(premise: &RdfDataset, map: &ImportMap) -> ClosureWalk {
    let mut declared: BTreeSet<String> = map.loaded.clone();
    declared_ontologies(premise, &mut declared);
    let mut queue: VecDeque<String> = imported_iris(premise).into_iter().collect();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut documents: Vec<Arc<RdfDataset>> = Vec::new();
    let mut unsupplied: Vec<String> = Vec::new();
    while let Some(iri) = queue.pop_front() {
        if !seen.insert(iri.clone()) {
            continue;
        }
        let Some(document) = map.get(&iri) else {
            unsupplied.push(iri);
            continue;
        };
        declared_ontologies(document, &mut declared);
        queue.extend(imported_iris(document));
        documents.push(Arc::clone(document));
    }
    unsupplied.retain(|iri| !declared.contains(iri));
    ClosureWalk {
        documents,
        unresolved: unsupplied,
    }
}

/// Add every ontology IRI `ds` declares to `into`: each IRI typed `owl:Ontology`, and each
/// IRI some ontology names as its `owl:versionIRI`.
fn declared_ontologies(ds: &RdfDataset, into: &mut BTreeSet<String>) {
    let rdf_type = ds.term_id_by_iri(RDF_TYPE);
    let ontology = ds.term_id_by_iri(OWL_ONTOLOGY);
    let version_iri = ds.term_id_by_iri(OWL_VERSIONIRI);
    if version_iri.is_none() && (rdf_type.is_none() || ontology.is_none()) {
        return;
    }
    for quad in ds.quads() {
        let named = if Some(quad.p) == rdf_type && Some(quad.o) == ontology {
            quad.s
        } else if Some(quad.p) == version_iri {
            quad.o
        } else {
            continue;
        };
        if let TermValue::Iri(iri) = ds.term_value(named) {
            into.insert(iri);
        }
    }
}

/// Every ontology IRI `ds` imports, in the dataset's own frozen quad order.
///
/// Only IRI objects: `owl:imports` is defined to relate an ontology to an ontology IRI, and
/// a blank node or literal object is not one — such a triple names no document and cannot
/// make one missing.
///
/// `owl:imports` is not an entailment construct — it is an RDF-level directive that any
/// consumer of a document has to honour — so this is `pub`: the CLI's SHACL lane reads an
/// imported document's imports through it rather than re-deriving "which objects count",
/// which is exactly the judgement the paragraph above records. Whether an import still NEEDS
/// a document is a different question, answered by [`unresolved_imports`].
#[must_use]
pub fn imported_iris(ds: &RdfDataset) -> Vec<String> {
    let Some(imports) = ds.term_id_by_iri(OWL_IMPORTS) else {
        return Vec::new();
    };
    ds.quads()
        .filter(|quad| quad.p == imports)
        .filter_map(|quad| match ds.term_value(quad.o) {
            TermValue::Iri(iri) => Some(iri),
            _ => None,
        })
        .collect()
}

/// The highest blank-node scope `ds` uses, so imported documents can be placed above it.
///
/// The survey is over [`term_positions`](crate::engine::term_positions) — every position
/// the merge below WRITES — and not over `quads()` alone. A premise may carry a blank node
/// that occurs only as a reifier, only inside an annotation, or only as a declared named
/// graph; a scope survey blind to those three would report a maximum BELOW a scope the
/// premise actually uses, and the very next imported document would be rescoped on top of
/// it. Standardize-apart (see the [module docs](self)) is then false in the direction that
/// matters: two documents' blank nodes would be the same term.
fn max_scope(ds: &RdfDataset) -> u32 {
    crate::engine::term_positions(ds)
        .map(|id| scope_of(&ds.term_value(id)))
        .max()
        .unwrap_or(0)
}

/// The highest blank-node scope a term mentions, recursing into triple terms.
fn scope_of(term: &TermValue) -> u32 {
    match term {
        TermValue::Blank { scope, .. } => scope.ordinal(),
        TermValue::Triple { s, p, o } => scope_of(s).max(scope_of(p)).max(scope_of(o)),
        TermValue::Iri(_) | TermValue::Literal { .. } => 0,
    }
}

/// Intern `value` into `b`, rewriting every blank-node scope through `rescope`.
fn intern_rescoped(
    b: &mut RdfDatasetBuilder,
    value: &TermValue,
    rescope: &mut ScopeMap<'_>,
) -> TermId {
    match value {
        TermValue::Blank { label, scope } => b.intern_blank(label, rescope.map(*scope)),
        TermValue::Triple { s, p, o } => {
            let s = intern_rescoped(b, s, rescope);
            let p = intern_rescoped(b, p, rescope);
            let o = intern_rescoped(b, o, rescope);
            b.intern_triple(s, p, o)
        }
        other => crate::interner::intern_into(b, other),
    }
}

/// An injective renumbering of ONE document's blank-node scopes into fresh ones.
///
/// Injective rather than "add a constant", because a document may already carry several
/// scopes of its own and collapsing two of them would merge nodes the document keeps apart.
struct ScopeMap<'a> {
    /// The document's own scope → the scope it was given here.
    assigned: BTreeMap<u32, BlankScope>,
    /// The next unused scope, shared across every document of one merge.
    next: &'a mut u32,
}

impl ScopeMap<'_> {
    /// The fresh scope this document's `scope` was given, allocating one on first sight.
    fn map(&mut self, scope: BlankScope) -> BlankScope {
        if let Some(&assigned) = self.assigned.get(&scope.ordinal()) {
            return assigned;
        }
        let assigned = BlankScope(*self.next);
        *self.next = self
            .next
            .checked_add(1)
            .expect("blank-node scope counter exceeded u32::MAX");
        self.assigned.insert(scope.ordinal(), assigned);
        assigned
    }
}

/// The premise together with its whole `owl:imports` closure, or the premise unchanged.
///
/// `Ok(None)` means there is no document to merge — the premise imports nothing, or every
/// ontology it imports is already in it (see [`unresolved_imports`]) — so there is no copy
/// to pay for and the caller reasons over the dataset it already has.
///
/// # Errors
///
/// [`EntailError::UnresolvedImport`] naming the first ontology IRI, in import order, that
/// neither `map` nor the closure resolves; [`EntailError::Build`] if the merged dataset
/// cannot be frozen.
pub(crate) fn resolve(
    premise: &RdfDataset,
    map: &ImportMap,
) -> Result<Option<Arc<RdfDataset>>, EntailError> {
    // Breadth-first over the import graph to a FIXPOINT, each document visited once. Two
    // properties follow, and both matter:
    //
    // * an imported document's OWN imports are followed, so a resolver that stopped at depth
    //   one — reasoning over a partial premise, which is the exact failure this module exists
    //   to prevent — is not what runs here. `an_imported_document_is_itself_checked_for_imports`
    //   is the falsifiable form.
    // * a CYCLE terminates rather than looping, and it does so without refusing: OWL 2 §3.4
    //   defines the imports closure as the transitive one and explicitly permits `A` to
    //   import `B` to import `A`. Hard-failing a cycle would refuse an ontology the
    //   specification allows, so the visited set is the answer and not a hedge.
    let ClosureWalk {
        documents,
        unresolved,
    } = walk_closure(premise, map);
    if let Some(iri) = unresolved.into_iter().next() {
        return Err(EntailError::UnresolvedImport(iri));
    }
    if documents.is_empty() {
        return Ok(None);
    }

    let mut b = RdfDatasetBuilder::new();
    crate::engine::copy_into(&mut b, premise);
    let mut next = max_scope(premise)
        .checked_add(1)
        .expect("blank-node scope counter exceeded u32::MAX");
    for document in &documents {
        let mut rescope = ScopeMap {
            assigned: BTreeMap::new(),
            next: &mut next,
        };
        for quad in document.quads() {
            let s = intern_rescoped(&mut b, &document.term_value(quad.s), &mut rescope);
            let p = intern_rescoped(&mut b, &document.term_value(quad.p), &mut rescope);
            let o = intern_rescoped(&mut b, &document.term_value(quad.o), &mut rescope);
            let g = quad
                .g
                .map(|g| intern_rescoped(&mut b, &document.term_value(g), &mut rescope));
            b.push_quad(s, p, o, g);
        }
        for (reifier, triple, graph) in document.reifiers_with_graph() {
            let reifier = intern_rescoped(&mut b, &document.term_value(reifier), &mut rescope);
            let triple = intern_rescoped(&mut b, &document.term_value(triple), &mut rescope);
            let graph =
                graph.map(|g| intern_rescoped(&mut b, &document.term_value(g), &mut rescope));
            b.push_reifier_in_graph(reifier, triple, graph);
        }
        for (reifier, predicate, object, graph) in document.annotations_with_graph() {
            let reifier = intern_rescoped(&mut b, &document.term_value(reifier), &mut rescope);
            let predicate = intern_rescoped(&mut b, &document.term_value(predicate), &mut rescope);
            let object = intern_rescoped(&mut b, &document.term_value(object), &mut rescope);
            let graph =
                graph.map(|g| intern_rescoped(&mut b, &document.term_value(g), &mut rescope));
            b.push_annotation_in_graph(reifier, predicate, object, graph);
        }
        for graph in document.named_graphs() {
            let g = intern_rescoped(&mut b, &document.term_value(graph), &mut rescope);
            b.declare_named_graph(g);
        }
    }
    b.freeze()
        .map(Some)
        .map_err(|e| EntailError::Build(e.to_string()))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use purrdf_core::{BlankScope, RdfDataset, RdfDatasetBuilder, TermValue};

    use super::{ImportMap, imported_iris, resolve, unresolved_imports};
    use crate::EntailError;
    use crate::vocab::{OWL_IMPORTS, OWL_ONTOLOGY, OWL_VERSIONIRI, RDF_TYPE};

    const P: &str = "http://example.org/p";

    /// A one-triple document `_:b p <o>`, plus optional `owl:imports` targets.
    fn document(label: &str, object: &str, imports: &[&str]) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_blank(label, BlankScope::DEFAULT);
        let p = b.intern_iri(P);
        let o = b.intern_iri(object);
        b.push_quad(s, p, o, None);
        for target in imports {
            let ontology = b.intern_iri("http://example.org/self");
            let imports = b.intern_iri(OWL_IMPORTS);
            let target = b.intern_iri(target);
            b.push_quad(ontology, imports, target, None);
        }
        b.freeze().expect("freeze")
    }

    #[test]
    fn a_premise_that_imports_nothing_is_not_copied() {
        let premise = document("b", "http://example.org/o", &[]);
        assert!(
            resolve(&premise, &ImportMap::new())
                .expect("no import to resolve")
                .is_none()
        );
    }

    #[test]
    fn an_unresolvable_import_names_the_document() {
        let premise = document("b", "http://example.org/o", &["http://example.org/a"]);
        let Err(EntailError::UnresolvedImport(iri)) = resolve(&premise, &ImportMap::new()) else {
            panic!("an import nobody supplied must refuse");
        };
        assert_eq!(iri, "http://example.org/a");
    }

    #[test]
    fn the_import_closure_is_transitive_and_cycle_safe() {
        // a imports b imports a: OWL 2 permits the cycle, so the merge must terminate and
        // must carry BOTH documents.
        let premise = document("b", "http://example.org/o", &["http://example.org/a"]);
        let mut map = ImportMap::new();
        map.insert(
            "http://example.org/a",
            document("b", "http://example.org/a-said", &["http://example.org/c"]),
        );
        map.insert(
            "http://example.org/c",
            document("b", "http://example.org/c-said", &["http://example.org/a"]),
        );
        let merged = resolve(&premise, &map)
            .expect("every import resolves")
            .expect("the premise imports something");
        let objects: Vec<String> = merged
            .quads()
            .filter_map(|quad| match merged.term_value(quad.o) {
                TermValue::Iri(iri) => Some(iri),
                _ => None,
            })
            .collect();
        for said in [
            "http://example.org/o",
            "http://example.org/a-said",
            "http://example.org/c-said",
        ] {
            assert!(objects.iter().any(|o| o == said), "{said} is missing");
        }
    }

    /// AN IMPORTED DOCUMENT IS ITSELF CHECKED FOR IMPORTS. A resolver that stopped at depth
    /// one would reason over a partial premise, which is the exact failure this module
    /// exists to prevent — so the depth-2 document's own content has to arrive, and the
    /// depth-2 import has to be REFUSED by name when nobody supplied it.
    #[test]
    fn an_imported_document_is_itself_checked_for_imports() {
        let premise = document("b", "http://example.org/o", &["http://example.org/a"]);
        let mut map = ImportMap::new();
        map.insert(
            "http://example.org/a",
            document(
                "b",
                "http://example.org/a-said",
                &["http://example.org/deep"],
            ),
        );
        // Depth 2 is unresolved, so the whole merge refuses NAMING it — a resolver that
        // stopped at depth 1 would have succeeded here with a premise missing an axiom.
        let Err(EntailError::UnresolvedImport(iri)) = resolve(&premise, &map) else {
            panic!("the imported document's own import must be followed");
        };
        assert_eq!(iri, "http://example.org/deep");

        // …and supplying it lets the merge through, carrying all three documents.
        map.insert(
            "http://example.org/deep",
            document("b", "http://example.org/deep-said", &[]),
        );
        let merged = resolve(&premise, &map)
            .expect("every import resolves")
            .expect("the premise imports something");
        let objects: Vec<String> = merged
            .quads()
            .filter_map(|quad| match merged.term_value(quad.o) {
                TermValue::Iri(iri) => Some(iri),
                _ => None,
            })
            .collect();
        assert!(objects.iter().any(|o| o == "http://example.org/deep-said"));
    }

    /// The map serves the RIF lane too, so a caller declares its documents ONCE.
    #[test]
    fn the_map_resolves_a_rif_import_the_same_way() {
        let mut map = ImportMap::new();
        map.insert(
            "http://example.org/lib",
            document("b", "http://example.org/o", &[]),
        );
        let mut resolve = map.rif_resolver();
        assert!(
            resolve(&crate::RifImport {
                location: "http://example.org/lib".to_owned(),
                profile: None,
            })
            .is_ok()
        );
        let Err(EntailError::UnresolvedImport(iri)) = resolve(&crate::RifImport {
            location: "http://example.org/missing".to_owned(),
            profile: None,
        }) else {
            panic!("an unsupplied RIF import refuses by name, exactly as an owl:imports does");
        };
        assert_eq!(iri, "http://example.org/missing");
    }

    /// A RESOLVED IMPORT IS NOT REPORTED AS AXIOMS THE RUN DID NOT HAVE.
    ///
    /// One boundary token used to mean both things: the report of an [`entails`] run whose
    /// whole import closure had been merged in carried the same
    /// `owl:imports … premises this run did not have` line as a materialization that had
    /// resolved nothing, so no consumer could tell the two apart. They are two constructs
    /// now, and this is the falsifiable form of the difference — the same premise, the same
    /// regime, the two paths, the two tokens.
    ///
    /// It also pins the completeness half: resolving the imports must not NARROW the report.
    /// A chase lane always meets [`Construct::DatatypeValueSpace`], so `exact-within-boundaries`
    /// is what an `OWL-RL` run says with or without an import — and the assertion is against
    /// the import-FREE run of the same regime rather than against a spelling, so a later
    /// change that made the import boundary the deciding one would fail here.
    #[test]
    fn a_resolved_import_is_a_different_boundary_from_an_unresolved_one() {
        use purrdf_core::RdfDatasetBuilder;

        use crate::report::Construct;
        use crate::{Materialization, Regime, entails, materialize};

        const SUB_CLASS_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
        const TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";

        /// `ex:tom a ex:Cat`, plus `ex:o owl:imports ex:schema` when `imports` is set.
        fn premise_graph(imports: bool) -> Arc<RdfDataset> {
            let mut b = RdfDatasetBuilder::new();
            let tom = b.intern_iri("http://example.org/tom");
            let ty = b.intern_iri(TYPE);
            let cat = b.intern_iri("http://example.org/Cat");
            b.push_quad(tom, ty, cat, None);
            if imports {
                let ontology = b.intern_iri("http://example.org/o");
                let predicate = b.intern_iri(OWL_IMPORTS);
                let schema = b.intern_iri("http://example.org/schema");
                b.push_quad(ontology, predicate, schema, None);
            }
            b.freeze().expect("freeze")
        }

        let premise = premise_graph(true);
        let mut b = RdfDatasetBuilder::new();
        let cat = b.intern_iri("http://example.org/Cat");
        let sub = b.intern_iri(SUB_CLASS_OF);
        let animal = b.intern_iri("http://example.org/Animal");
        b.push_quad(cat, sub, animal, None);
        let schema = b.freeze().expect("freeze");

        let mut map = ImportMap::new();
        map.insert("http://example.org/schema", schema);

        // `ex:tom a ex:Animal` — reachable ONLY through the imported schema, so an entailed
        // verdict is itself the proof that the run had the imported axioms.
        let mut b = RdfDatasetBuilder::new();
        let tom = b.intern_iri("http://example.org/tom");
        let ty = b.intern_iri(TYPE);
        let animal = b.intern_iri("http://example.org/Animal");
        b.push_quad(tom, ty, animal, None);
        let conclusion = b.freeze().expect("freeze");

        let certificate = entails(&premise, &conclusion, Regime::OwlRl, &map)
            .expect("every import resolves, and the premise is consistent");
        let constructs: Vec<Construct> = certificate
            .report()
            .boundaries()
            .iter()
            .map(|boundary| boundary.construct())
            .collect();
        assert!(
            constructs.contains(&Construct::ResolvedOntologyImport),
            "a run whose imports were merged names the RESOLVED construct: {constructs:?}"
        );
        assert!(
            !constructs.contains(&Construct::UnresolvedOntologyImport),
            "…and never the one that says the axioms were missing: {constructs:?}"
        );

        // The same premise through `materialize`, which takes no map at all: the honest
        // "this run did not have them" signal is exactly what must survive there.
        let (_, report) = materialize(&premise, Materialization::OwlRl).expect("a closure");
        let materialized: Vec<Construct> = report
            .boundaries()
            .iter()
            .map(|boundary| boundary.construct())
            .collect();
        assert!(
            materialized.contains(&Construct::UnresolvedOntologyImport),
            "a materialization resolved nothing and must say so: {materialized:?}"
        );
        assert!(
            !materialized.contains(&Construct::ResolvedOntologyImport),
            "…and must not claim a merge it never made: {materialized:?}"
        );

        // Resolving the imports does not narrow the report: the completeness of the merged
        // run is the completeness of a run of the same regime that imports nothing.
        let (_, plain) = materialize(&premise_graph(false), Materialization::OwlRl)
            .expect("a closure over an import-free premise");
        assert_eq!(certificate.report().completeness(), plain.completeness());
    }

    /// A PREMISE BLANK NODE IN NO QUAD IS STILL A PREMISE BLANK NODE.
    ///
    /// The premise's every quad is all-IRI; its only blank node is a REIFIER, at
    /// `BlankScope(1)`. A scope survey that read `quads()` alone therefore reported a
    /// maximum of 0, allocated the first imported document `BlankScope(1)`, and rescoped the
    /// imported `_:r` onto the premise's reifier — one term where the two documents named
    /// two, which is precisely the standardize-apart guarantee the module docs state.
    ///
    /// The assertion is over the merged dataset's whole term space rather than its quads,
    /// for the same reason the bug existed: the colliding term is in a side table.
    #[test]
    fn a_premise_blank_node_that_occurs_only_as_a_reifier_is_not_overwritten() {
        // The premise: two all-IRI quads, `_:r` reifying one of them at `BlankScope(1)`,
        // and an `owl:imports`.
        let premise = {
            let mut b = RdfDatasetBuilder::new();
            let s = b.intern_iri("http://example.org/s");
            let p = b.intern_iri("http://example.org/p");
            let o = b.intern_iri("http://example.org/o");
            b.push_quad(s, p, o, None);
            let ontology = b.intern_iri("http://example.org/self");
            let imports = b.intern_iri(OWL_IMPORTS);
            let target = b.intern_iri("http://example.org/a");
            b.push_quad(ontology, imports, target, None);
            let triple = b.intern_triple(s, p, o);
            let reifier = b.intern_blank("r", BlankScope(1));
            b.push_reifier_in_graph(reifier, triple, None);
            b.freeze().expect("freeze")
        };

        // The imported document names its own `_:r`, in the DEFAULT scope.
        let mut map = ImportMap::new();
        map.insert(
            "http://example.org/a",
            document("r", "http://example.org/a-said", &[]),
        );

        let merged = resolve(&premise, &map)
            .expect("every import resolves")
            .expect("the premise imports something");
        let mut scopes: Vec<u32> = crate::engine::term_positions(&merged)
            .filter_map(|id| match merged.term_value(id) {
                TermValue::Blank { label, scope } if label == "r" => Some(scope.ordinal()),
                _ => None,
            })
            .collect();
        scopes.sort_unstable();
        scopes.dedup();
        assert_eq!(
            scopes.len(),
            2,
            "the premise's reifier and the imported document's `_:r` are two nodes, not one: {scopes:?}"
        );
        assert!(
            scopes.contains(&1),
            "the premise's own scope must survive the merge unmoved: {scopes:?}"
        );
    }

    #[test]
    fn merged_documents_are_standardized_apart_and_the_premise_is_not_moved() {
        // Every document calls its blank node `_:b`. They are three different nodes, and the
        // premise's keeps the scope the caller gave it.
        let premise = document("b", "http://example.org/o", &["http://example.org/a"]);
        let mut map = ImportMap::new();
        map.insert(
            "http://example.org/a",
            document("b", "http://example.org/a-said", &["http://example.org/c"]),
        );
        map.insert(
            "http://example.org/c",
            document("b", "http://example.org/c-said", &[]),
        );
        let merged = resolve(&premise, &map)
            .expect("every import resolves")
            .expect("the premise imports something");
        let mut scopes: Vec<u32> = merged
            .quads()
            .filter_map(|quad| match merged.term_value(quad.s) {
                TermValue::Blank { label, scope } if label == "b" => Some(scope.ordinal()),
                _ => None,
            })
            .collect();
        scopes.sort_unstable();
        scopes.dedup();
        assert_eq!(
            scopes.len(),
            3,
            "three documents named their blank node `_:b`; they are three nodes"
        );
        assert!(
            scopes.contains(&BlankScope::DEFAULT.ordinal()),
            "the premise's own scope must survive the merge unmoved"
        );
    }

    // ── The in-graph resolution rule ────────────────────────────────────────────────

    /// The W3C SHACL 1.2 core vocabulary document, vendored beside the shapes engine.
    const SHACL_TTL: &str = include_str!("../../../shapes/spec/shacl.ttl");
    /// The W3C SHACL 1.2 node-expression vocabulary, which `owl:imports <sh:>`.
    const SHNEX_TTL: &str = include_str!("../../../shapes/spec/shnex.ttl");
    /// The ontology IRI `shnex.ttl` imports and `shacl.ttl` declares.
    const SH: &str = "http://www.w3.org/ns/shacl#";

    /// Parse one Turtle document with the native codec.
    fn turtle(text: &str) -> Arc<RdfDataset> {
        purrdf_rdf::parse_dataset(text.as_bytes(), "text/turtle", None).expect("turtle parses")
    }

    /// A dataset of the given IRI triples.
    fn triples(rows: &[(&str, &str, &str)]) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        for (s, p, o) in rows {
            let s = b.intern_iri(s);
            let p = b.intern_iri(p);
            let o = b.intern_iri(o);
            b.push_quad(s, p, o, None);
        }
        b.freeze().expect("freeze")
    }

    /// The merged SHACL 1.2 vocabularies: `shnex.ttl` imports `sh:`, and `shacl.ttl` is in
    /// the same graph declaring `sh: a owl:Ontology`, so nothing is missing.
    #[test]
    fn import_present_in_graph_is_resolved() {
        let merged = RdfDataset::union(&[&turtle(SHNEX_TTL), &turtle(SHACL_TTL)]);
        // The oracle observes the import: the graph DOES import `sh:`, so an empty answer
        // is the rule resolving it, not a graph with nothing to resolve.
        assert!(imported_iris(&merged).iter().any(|iri| iri == SH));
        assert_eq!(unresolved_imports(&merged, &[]), Vec::<String>::new());
        // …and `entails` takes the same verdict, with no import map at all.
        resolve(&merged, &ImportMap::new()).expect("an in-graph ontology resolves its import");
    }

    /// The neighbour of the test above: `shnex.ttl` without `shacl.ttl` imports an ontology
    /// the graph does not contain, and that import alone is named.
    #[test]
    fn absent_import_is_unresolved() {
        let shnex = turtle(SHNEX_TTL);
        assert_eq!(unresolved_imports(&shnex, &[]), vec![SH.to_owned()]);
        let Err(EntailError::UnresolvedImport(iri)) = resolve(&shnex, &ImportMap::new()) else {
            panic!("an import of an absent ontology must refuse");
        };
        assert_eq!(iri, SH);
    }

    /// A document that imports its OWN IRI — the SHACL `sh:prefixes/owl:imports*` idiom,
    /// from a node that is no `owl:Ontology` — imports nothing missing once the caller says
    /// the graph was read from that IRI. The neighbours: the same graph with no loaded IRI,
    /// or with a different one, still refuses the import by name.
    #[test]
    fn self_import_is_resolved() {
        const DOC: &str = "http://example.org/shapes/doc.ttl";
        let graph = purrdf_rdf::parse_dataset(
            b"@prefix owl: <http://www.w3.org/2002/07/owl#> .\n\
              @prefix sh: <http://www.w3.org/ns/shacl#> .\n\
              <> sh:declare [ sh:prefix \"ex\" ; sh:namespace \"http://example.org/ns#\" ] .\n\
              <http://example.org/shapes/doc.ttl#Prefixes> owl:imports <> .\n",
            "text/turtle",
            Some(DOC),
        )
        .expect("turtle parses");
        assert_eq!(imported_iris(&graph), vec![DOC.to_owned()]);
        assert_eq!(unresolved_imports(&graph, &[DOC]), Vec::<String>::new());
        assert_eq!(unresolved_imports(&graph, &[]), vec![DOC.to_owned()]);
        assert_eq!(
            unresolved_imports(&graph, &["http://example.org/shapes/other.ttl"]),
            vec![DOC.to_owned()]
        );

        // `entails` takes the same verdict once the map knows where the premise came from.
        let mut map = ImportMap::new();
        assert!(resolve(&graph, &map).is_err());
        map.declare_loaded(DOC);
        assert!(
            resolve(&graph, &map)
                .expect("a self-import resolves")
                .is_none()
        );
    }

    /// An ontology whose `owl:versionIRI` is the imported IRI resolves it; the neighbour
    /// whose version IRI is a different one does not.
    #[test]
    fn version_iri_resolves() {
        const IMPORTER: &str = "http://example.org/importer";
        const LIB: &str = "http://example.org/lib";
        const V1: &str = "http://example.org/lib/1.0";
        const V2: &str = "http://example.org/lib/2.0";

        let with = |version: &str| {
            triples(&[
                (IMPORTER, RDF_TYPE, OWL_ONTOLOGY),
                (IMPORTER, OWL_IMPORTS, V1),
                (LIB, RDF_TYPE, OWL_ONTOLOGY),
                (LIB, OWL_VERSIONIRI, version),
            ])
        };
        assert_eq!(unresolved_imports(&with(V1), &[]), Vec::<String>::new());
        assert_eq!(unresolved_imports(&with(V2), &[]), vec![V1.to_owned()]);
    }

    /// The closure is transitive both ways a document can arrive: an in-graph ontology's
    /// own imports are checked, and a map-supplied document's declarations resolve an
    /// import the walk met BEFORE reaching that document. Each unresolved IRI is named once,
    /// in walk order.
    #[test]
    fn the_resolution_rule_is_transitive_over_the_import_closure() {
        const ROOT: &str = "http://example.org/root";
        const INNER: &str = "http://example.org/inner";
        const DEEP: &str = "http://example.org/deep";
        const EARLY: &str = "http://example.org/early";
        const SUPPLIED: &str = "http://example.org/supplied";
        const MISSING: &str = "http://example.org/missing";

        // In-graph: `inner` is declared here, so it resolves — and ITS import of `deep`,
        // declared nowhere, is what stays unresolved. `deep` is imported twice and named
        // once.
        let premise = triples(&[
            (ROOT, RDF_TYPE, OWL_ONTOLOGY),
            (ROOT, OWL_IMPORTS, INNER),
            (INNER, RDF_TYPE, OWL_ONTOLOGY),
            (INNER, OWL_IMPORTS, DEEP),
            (ROOT, OWL_IMPORTS, DEEP),
        ]);
        assert_eq!(unresolved_imports(&premise, &[]), vec![DEEP.to_owned()]);

        // Through a map: `early` is met first and supplied by no one, but the document the
        // map supplies for `supplied` declares it; that document's own import of `missing`
        // is followed and named.
        let premise = triples(&[(ROOT, OWL_IMPORTS, EARLY), (ROOT, OWL_IMPORTS, SUPPLIED)]);
        let mut map = ImportMap::new();
        map.insert(
            SUPPLIED,
            triples(&[
                (EARLY, RDF_TYPE, OWL_ONTOLOGY),
                (SUPPLIED, OWL_IMPORTS, MISSING),
            ]),
        );
        assert_eq!(map.unresolved_imports(&premise), vec![MISSING.to_owned()]);
        // With no map, both of the premise's imports are missing, in import order.
        assert_eq!(
            unresolved_imports(&premise, &[]),
            vec![EARLY.to_owned(), SUPPLIED.to_owned()]
        );
        // Supplying the last document closes the walk.
        map.insert(MISSING, triples(&[]));
        assert_eq!(map.unresolved_imports(&premise), Vec::<String>::new());
    }

    /// `entails` over a premise whose import is already in it runs with no map, and its
    /// report says the closure was HAD — not that the imported axioms were missing.
    #[test]
    fn an_import_present_in_the_premise_needs_no_map_and_reports_resolved() {
        use crate::report::Construct;
        use crate::{Regime, entails};

        const SUB_CLASS_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
        let premise = triples(&[
            (
                "http://example.org/o",
                OWL_IMPORTS,
                "http://example.org/schema",
            ),
            ("http://example.org/schema", RDF_TYPE, OWL_ONTOLOGY),
            (
                "http://example.org/Cat",
                SUB_CLASS_OF,
                "http://example.org/Animal",
            ),
            ("http://example.org/tom", RDF_TYPE, "http://example.org/Cat"),
        ]);
        let conclusion = triples(&[(
            "http://example.org/tom",
            RDF_TYPE,
            "http://example.org/Animal",
        )]);
        let certificate = entails(&premise, &conclusion, Regime::OwlRl, &ImportMap::new())
            .expect("the import is resolved in place");
        let constructs: Vec<Construct> = certificate
            .report()
            .boundaries()
            .iter()
            .map(|boundary| boundary.construct())
            .collect();
        assert!(
            constructs.contains(&Construct::ResolvedOntologyImport),
            "{constructs:?}"
        );
        assert!(
            !constructs.contains(&Construct::UnresolvedOntologyImport),
            "{constructs:?}"
        );
    }
}

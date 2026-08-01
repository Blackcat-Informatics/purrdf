// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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
use crate::vocab::OWL_IMPORTS;

/// The documents an `owl:imports` resolves to.
///
/// PurRDF mints no vocabulary and fetches nothing: it has no notion of what an ontology IRI
/// dereferences to, and inventing one would make an entailment depend on the network. So
/// the import closure is **caller-supplied configuration**, exactly like every other
/// vocabulary this library reads, and a premise that imports a document the caller did not
/// supply is a hard error rather than a silently truncated premise.
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
}

/// Every ontology IRI `ds` imports, in the dataset's own frozen quad order.
///
/// Only IRI objects: `owl:imports` is defined to relate an ontology to an ontology IRI, and
/// a blank node or literal object is not one — such a triple names no document and cannot
/// make one missing.
fn imported_iris(ds: &RdfDataset) -> Vec<String> {
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
fn max_scope(ds: &RdfDataset) -> u32 {
    ds.quads()
        .flat_map(|quad| {
            [Some(quad.s), Some(quad.p), Some(quad.o), quad.g]
                .into_iter()
                .flatten()
        })
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
/// `Ok(None)` means the premise imports nothing, so there is no merge to do and no copy to
/// pay for — the caller reasons over the dataset it already has.
///
/// # Errors
///
/// [`EntailError::UnresolvedImport`] naming the first ontology IRI, in import order, that
/// `map` does not resolve; [`EntailError::Build`] if the merged dataset cannot be frozen.
pub(crate) fn resolve(
    premise: &RdfDataset,
    map: &ImportMap,
) -> Result<Option<Arc<RdfDataset>>, EntailError> {
    let direct = imported_iris(premise);
    if direct.is_empty() {
        return Ok(None);
    }

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
    let mut queue: VecDeque<String> = direct.into_iter().collect();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut documents: Vec<Arc<RdfDataset>> = Vec::new();
    while let Some(iri) = queue.pop_front() {
        if !seen.insert(iri.clone()) {
            continue;
        }
        let Some(document) = map.get(&iri) else {
            return Err(EntailError::UnresolvedImport(iri));
        };
        for onward in imported_iris(document) {
            queue.push_back(onward);
        }
        documents.push(Arc::clone(document));
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

    use super::{ImportMap, resolve};
    use crate::EntailError;
    use crate::vocab::OWL_IMPORTS;

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
}

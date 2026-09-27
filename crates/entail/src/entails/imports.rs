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
//! # Which `owl:imports` is an import
//!
//! OWL 2 reads a document's imports off its ontology header — *Mapping to RDF Graphs*
//! §3.1.2: "The set Imp(G) of the IRIs of ontology documents that are directly imported into
//! G contains exactly all *:z1, ..., *:zk that are matched in the pattern" of Table 4,
//! `*:x rdf:type owl:Ontology . *:x owl:imports *:z1 ...` (or `_:x` for an anonymous
//! ontology). So an `owl:imports` on a node of the premise that is not its ontology header,
//! not an IRI the premise was loaded under ([`ImportMap::declare_loaded`]), and not a node
//! naming either as its `owl:versionIRI` imports nothing: it is an ordinary premise triple,
//! reasoned over like any other, and no document is looked for. The rule is
//! [`purrdf_core::imports`]'s (see there for the full citation), and it is the same rule
//! the SHACL engine applies to a shapes graph.
//!
//! # An ontology already in the graph is not missing
//!
//! A premise that ALREADY holds the ontology it imports — its `owl:Ontology` header, or an
//! ontology whose `owl:versionIRI` is the imported IRI — has that document's axioms in hand,
//! and refusing it would refuse the closure the import asked for. So does a premise that
//! imports the very document it was read from ([`ImportMap::declare_loaded`]).
//!
//! That rule is not this crate's: it is [`purrdf_core::imports`], re-exported here, and the
//! SHACL engine takes its verdict about a shapes graph's `owl:imports` from the same code.
//! Entailment and SHACL do not depend on each other, so the kernel both sit on is the one
//! place a single rule can serve them both.
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
//! [`rif_resolver`] is the bridge: one map of caller-supplied documents serves
//! both, so a caller that already declared what its ontology IRIs denote does not declare it
//! twice.
//!
//! What the bridge does NOT do is pretend the two resolutions are one operation, because
//! they are not. A RIF import names an entailment PROFILE and contributes the FACTS of the
//! closure computed under it; an `owl:imports` names an ontology and contributes its AXIOMS
//! verbatim, with the closure computed after the merge. Collapsing those would change what
//! one of them means, so what is shared is the configuration and the no-I/O rule, and the
//! two consumers stay separate.

use std::sync::Arc;

use purrdf_core::RdfDataset;

use crate::EntailError;
use crate::rif_xml::RifImport;

pub use purrdf_core::imports::{ImportClosure, ImportMap, imported_iris, unresolved_imports};

/// `map` as a resolver for [`resolve_rif_imports`](crate::resolve_rif_imports).
///
/// A [`RifImport`]'s `location` is looked up exactly as an `owl:imports` object is, and an
/// unresolved one refuses by name through the SAME error. See the [module docs](self) for
/// why the two resolutions share their configuration and not their semantics.
///
/// ```
/// use purrdf_core::RdfDatasetBuilder;
/// use purrdf_entail::{EntailError, ImportMap, RifImport, rif_resolver};
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
/// let mut resolve = rif_resolver(&map);
///
/// let known = RifImport { location: "http://example.org/lib".to_owned(), profile: None };
/// assert!(resolve(&known).is_ok());
/// let unknown = RifImport { location: "http://example.org/other".to_owned(), profile: None };
/// assert!(matches!(
///     resolve(&unknown),
///     Err(EntailError::UnresolvedImport(ref iri)) if iri == "http://example.org/other"
/// ));
/// ```
pub fn rif_resolver(
    map: &ImportMap,
) -> impl FnMut(&RifImport) -> Result<Arc<RdfDataset>, EntailError> + '_ {
    move |import: &RifImport| {
        map.get(&import.location)
            .map(Arc::clone)
            .ok_or_else(|| EntailError::UnresolvedImport(import.location.clone()))
    }
}

/// The premise together with its whole `owl:imports` closure, or the premise unchanged.
///
/// `Ok(None)` means there is no document to merge — the premise imports nothing, or every
/// ontology it imports is already in it (see [`unresolved_imports`]) — so there is no copy
/// to pay for and the caller reasons over the dataset it already has.
///
/// The walk and the merge are [`ImportMap::closure`] and [`ImportClosure::merge`]: the
/// closure is followed to a fixpoint, so an imported document's OWN imports are checked
/// (`an_imported_document_is_itself_checked_for_imports` is the falsifiable form), and a
/// cycle terminates without refusing, because OWL 2 §3.4 permits one.
///
/// A supplied document the closure never reaches is NOT refused here: an entailment
/// [`ImportMap`] is a table of what the caller's ontology IRIs denote, shared with the RIF
/// lane ([`rif_resolver`]), and a document one lane never reaches may be the one the other
/// does.
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
    let closure = map.closure(premise);
    if let Some(iri) = closure.unresolved().first() {
        return Err(EntailError::UnresolvedImport(iri.clone()));
    }
    closure
        .merge(premise)
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

    /// A one-triple document `_:b p <o>`, plus an ontology header `ex:self a owl:Ontology`
    /// importing each target.
    fn document(label: &str, object: &str, imports: &[&str]) -> Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let s = b.intern_blank(label, BlankScope::DEFAULT);
        let p = b.intern_iri(P);
        let o = b.intern_iri(object);
        b.push_quad(s, p, o, None);
        let ontology = b.intern_iri("http://example.org/self");
        let rdf_type = b.intern_iri(RDF_TYPE);
        let owl_ontology = b.intern_iri(OWL_ONTOLOGY);
        b.push_quad(ontology, rdf_type, owl_ontology, None);
        for target in imports {
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
        let mut resolve = super::rif_resolver(&map);
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

        /// `ex:tom a ex:Cat`, plus `ex:o a owl:Ontology ; owl:imports ex:schema` when
        /// `imports` is set.
        fn premise_graph(imports: bool) -> Arc<RdfDataset> {
            let mut b = RdfDatasetBuilder::new();
            let tom = b.intern_iri("http://example.org/tom");
            let ty = b.intern_iri(TYPE);
            let cat = b.intern_iri("http://example.org/Cat");
            b.push_quad(tom, ty, cat, None);
            if imports {
                let ontology = b.intern_iri("http://example.org/o");
                let owl_ontology = b.intern_iri(OWL_ONTOLOGY);
                b.push_quad(ontology, ty, owl_ontology, None);
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
        // The premise: all-IRI quads, `_:r` reifying one of them at `BlankScope(1)`, and an
        // ontology header's `owl:imports`.
        let premise = {
            let mut b = RdfDatasetBuilder::new();
            let s = b.intern_iri("http://example.org/s");
            let p = b.intern_iri("http://example.org/p");
            let o = b.intern_iri("http://example.org/o");
            b.push_quad(s, p, o, None);
            let ontology = b.intern_iri("http://example.org/self");
            let rdf_type = b.intern_iri(RDF_TYPE);
            let owl_ontology = b.intern_iri(OWL_ONTOLOGY);
            b.push_quad(ontology, rdf_type, owl_ontology, None);
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
    //
    // The same rule over the vendored W3C SHACL 1.2 vocabulary files themselves is held
    // by `purrdf-shapes` (`imports::tests`), which reads Turtle; these are the headers
    // those files carry, so this crate needs no parser to test the rule.

    /// The ontology IRI `shnex.ttl` imports and `shacl.ttl` declares.
    const SH: &str = "http://www.w3.org/ns/shacl#";
    /// The ontology IRI `shnex.ttl` declares.
    const SHNEX: &str = "http://www.w3.org/ns/shacl-node-expr#";

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

    /// `shnex.ttl`'s header: it declares its own ontology and imports `sh:`.
    fn shnex_header() -> Vec<(&'static str, &'static str, &'static str)> {
        vec![(SHNEX, RDF_TYPE, OWL_ONTOLOGY), (SHNEX, OWL_IMPORTS, SH)]
    }

    /// The merged SHACL 1.2 vocabularies: `shnex.ttl` imports `sh:`, and `shacl.ttl` is in
    /// the same graph declaring `sh: a owl:Ontology`, so nothing is missing.
    #[test]
    fn import_present_in_graph_is_resolved() {
        let mut rows = shnex_header();
        rows.push((SH, RDF_TYPE, OWL_ONTOLOGY));
        let merged = triples(&rows);
        // The oracle observes the import: the graph DOES import `sh:`, so an empty answer
        // is the rule resolving it, not a graph with nothing to resolve.
        assert!(
            imported_iris(merged.as_ref(), &[])
                .iter()
                .any(|iri| iri == SH)
        );
        assert_eq!(unresolved_imports(&merged, &[]), Vec::<String>::new());
        // …and `entails` takes the same verdict, with no import map at all.
        resolve(&merged, &ImportMap::new()).expect("an in-graph ontology resolves its import");
    }

    /// The neighbour of the test above: `shnex.ttl` without `shacl.ttl` imports an ontology
    /// the graph does not contain, and that import alone is named.
    #[test]
    fn absent_import_is_unresolved() {
        let shnex = triples(&shnex_header());
        assert_eq!(unresolved_imports(&shnex, &[]), vec![SH.to_owned()]);
        let Err(EntailError::UnresolvedImport(iri)) = resolve(&shnex, &ImportMap::new()) else {
            panic!("an import of an absent ontology must refuse");
        };
        assert_eq!(iri, SH);
    }

    /// A document whose header imports the document's OWN retrieval IRI imports nothing
    /// missing once the caller says the graph was read from that IRI. The neighbours: the
    /// same graph with no loaded IRI, or with a different one, still refuses the import by
    /// name.
    #[test]
    fn self_import_is_resolved() {
        const DOC: &str = "http://example.org/shapes/doc.ttl";
        const HEADER: &str = "http://example.org/shapes/doc.ttl#ontology";
        let graph = triples(&[(HEADER, RDF_TYPE, OWL_ONTOLOGY), (HEADER, OWL_IMPORTS, DOC)]);
        assert_eq!(imported_iris(graph.as_ref(), &[]), vec![DOC.to_owned()]);
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
        let premise = triples(&[
            (ROOT, RDF_TYPE, OWL_ONTOLOGY),
            (ROOT, OWL_IMPORTS, EARLY),
            (ROOT, OWL_IMPORTS, SUPPLIED),
        ]);
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
            ("http://example.org/o", RDF_TYPE, OWL_ONTOLOGY),
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

    // ── Which `owl:imports` is an import ────────────────────────────────────────────

    const SUB_CLASS_OF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
    const TOM: &str = "http://example.org/tom";
    const CAT: &str = "http://example.org/Cat";
    const ANIMAL: &str = "http://example.org/Animal";
    const LIB: &str = "http://example.org/lib";
    const DOC: &str = "http://example.org/premise.ttl";
    const OTHER_NODE: &str = "http://example.org/other-node";

    /// `ex:tom a ex:Cat` plus `rows`.
    fn premise_with(rows: &[(&str, &str, &str)]) -> Arc<RdfDataset> {
        let mut all = vec![(TOM, RDF_TYPE, CAT)];
        all.extend_from_slice(rows);
        triples(&all)
    }

    /// The map supplying `ex:Cat rdfs:subClassOf ex:Animal` as [`LIB`].
    fn schema_map() -> ImportMap {
        let mut map = ImportMap::new();
        map.insert(LIB, triples(&[(CAT, SUB_CLASS_OF, ANIMAL)]));
        map
    }

    /// The outcome of `entails(premise, conclusion)` under OWL 2 RL, and the constructs its
    /// report names.
    fn run(
        premise: &RdfDataset,
        conclusion: &[(&str, &str, &str)],
        map: &ImportMap,
    ) -> Result<(bool, Vec<crate::report::Construct>), EntailError> {
        let certificate = crate::entails(premise, &triples(conclusion), crate::Regime::OwlRl, map)?;
        let entailed = matches!(certificate.outcome(), crate::EntailmentOutcome::Entailed(_));
        let constructs = certificate
            .report()
            .boundaries()
            .iter()
            .map(|boundary| boundary.construct())
            .collect();
        Ok((entailed, constructs))
    }

    /// An `owl:imports` on a node that is neither the premise's header nor its loaded IRI is
    /// a premise triple, not an import: the run is not refused, the triple is itself entailed
    /// (it is in the premise), and the report names no import boundary. The neighbour types
    /// the same node `owl:Ontology`, and its unsupplied import is refused by name.
    #[test]
    fn a_non_anchor_owl_imports_is_a_premise_triple_and_a_header_one_is_an_import() {
        use crate::report::Construct;

        let data = premise_with(&[(OTHER_NODE, OWL_IMPORTS, LIB)]);
        let (entailed, constructs) =
            run(&data, &[(OTHER_NODE, OWL_IMPORTS, LIB)], &ImportMap::new())
                .expect("a non-anchor owl:imports is not refused");
        assert!(entailed, "the triple is still in the premise, as data");
        assert!(
            !constructs.contains(&Construct::UnresolvedOntologyImport)
                && !constructs.contains(&Construct::ResolvedOntologyImport),
            "no document was imported, so no import boundary: {constructs:?}"
        );

        let header = premise_with(&[
            (OTHER_NODE, RDF_TYPE, OWL_ONTOLOGY),
            (OTHER_NODE, OWL_IMPORTS, LIB),
        ]);
        let Err(EntailError::UnresolvedImport(iri)) =
            run(&header, &[(TOM, RDF_TYPE, CAT)], &ImportMap::new())
        else {
            panic!("an ontology header's unsupplied import is refused");
        };
        assert_eq!(iri, LIB);
        let (entailed, constructs) =
            run(&header, &[(TOM, RDF_TYPE, ANIMAL)], &schema_map()).expect("supplied");
        assert!(entailed, "the imported schema's axiom took part");
        assert!(constructs.contains(&Construct::ResolvedOntologyImport));
    }

    /// An `owl:imports` on the IRI the premise was loaded under is an import: unsupplied it
    /// is refused; supplied, the conclusion only the imported schema licenses is entailed and
    /// the report says the closure was resolved. The neighbour declares no loaded IRI, so the
    /// very same triple is data: nothing is refused, the supplied schema is never merged, and
    /// the same conclusion is NOT entailed.
    #[test]
    fn an_import_on_the_loaded_iri_counts_and_without_it_the_triple_is_data() {
        use crate::report::Construct;

        let premise = premise_with(&[(DOC, OWL_IMPORTS, LIB)]);
        let mut loaded = ImportMap::new();
        loaded.declare_loaded(DOC);
        let Err(EntailError::UnresolvedImport(iri)) =
            run(&premise, &[(TOM, RDF_TYPE, CAT)], &loaded)
        else {
            panic!("an unsupplied import on the loaded IRI is refused");
        };
        assert_eq!(iri, LIB);

        let mut supplied = schema_map();
        supplied.declare_loaded(DOC);
        let (entailed, constructs) =
            run(&premise, &[(TOM, RDF_TYPE, ANIMAL)], &supplied).expect("supplied");
        assert!(entailed);
        assert!(
            constructs.contains(&Construct::ResolvedOntologyImport)
                && !constructs.contains(&Construct::UnresolvedOntologyImport),
            "{constructs:?}"
        );

        let (entailed, constructs) = run(&premise, &[(TOM, RDF_TYPE, ANIMAL)], &schema_map())
            .expect("with no loaded IRI the triple is data");
        assert!(
            !entailed,
            "the schema was not imported, so its axiom took no part"
        );
        assert!(!constructs.contains(&Construct::ResolvedOntologyImport));
        let (entailed, _) = run(&premise, &[(DOC, OWL_IMPORTS, LIB)], &schema_map())
            .expect("with no loaded IRI the triple is data");
        assert!(entailed, "the triple is still in the premise");
    }

    /// An anonymous ontology header imports, and so does a node naming the loaded IRI as its
    /// version IRI. Their neighbours — an untyped blank node, a node versioning another IRI —
    /// import nothing.
    #[test]
    fn a_blank_header_and_a_versioning_node_import_and_their_neighbours_do_not() {
        let blank = |class: &str| {
            let mut b = RdfDatasetBuilder::new();
            let header = b.intern_blank("h", BlankScope::DEFAULT);
            let rdf_type = b.intern_iri(RDF_TYPE);
            let class = b.intern_iri(class);
            let imports = b.intern_iri(OWL_IMPORTS);
            let lib = b.intern_iri(LIB);
            b.push_quad(header, rdf_type, class, None);
            b.push_quad(header, imports, lib, None);
            b.freeze().expect("freeze")
        };
        let Err(EntailError::UnresolvedImport(iri)) =
            resolve(&blank(OWL_ONTOLOGY), &ImportMap::new())
        else {
            panic!("an anonymous ontology's import is refused unsupplied");
        };
        assert_eq!(iri, LIB);
        assert!(
            resolve(&blank("http://example.org/Thing"), &ImportMap::new())
                .expect("an untyped blank node imports nothing")
                .is_none()
        );

        const SERIES: &str = "http://example.org/series";
        let versioned = |version: &str| {
            triples(&[
                (SERIES, OWL_VERSIONIRI, version),
                (SERIES, OWL_IMPORTS, LIB),
            ])
        };
        let mut map = ImportMap::new();
        map.declare_loaded(DOC);
        let Err(EntailError::UnresolvedImport(iri)) = resolve(&versioned(DOC), &map) else {
            panic!("a node versioning the loaded IRI stands for the premise");
        };
        assert_eq!(iri, LIB);
        assert!(
            resolve(&versioned("http://example.org/elsewhere"), &map)
                .expect("a node versioning another IRI imports nothing")
                .is_none()
        );
    }

    /// A materialization is handed no loaded IRI, so its import boundary is raised by an
    /// ontology header's import and by nothing else: a non-anchor `owl:imports` raises none.
    #[test]
    fn materialize_raises_the_import_boundary_for_a_header_import_only() {
        use crate::report::Construct;
        use crate::{Materialization, materialize};

        let raised = |premise: &RdfDataset| {
            let (_, report) = materialize(premise, Materialization::OwlRl).expect("a closure");
            report
                .boundaries()
                .iter()
                .any(|boundary| boundary.construct() == Construct::UnresolvedOntologyImport)
        };
        assert!(raised(&premise_with(&[
            (OTHER_NODE, RDF_TYPE, OWL_ONTOLOGY),
            (OTHER_NODE, OWL_IMPORTS, LIB),
        ])));
        assert!(!raised(&premise_with(&[(OTHER_NODE, OWL_IMPORTS, LIB)])));
    }
}

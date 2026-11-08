// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The **consuming** GTS `Graph` importer (C2.b).
//!
//! Where [`super::import_sink`] is the authoritative, scope-preserving *streaming*
//! path, this is the convenience path: it takes an already-folded
//! [`purrdf_gts::model::Graph`] **by value** and MOVES its owned term strings into
//! the interner (`std::mem::take` on `Term.value` / `Term.lang`), rather than
//! cloning them.
//!
//! Because `purrdf_gts::reader::read()` folds every segment into one append-order
//! term table, per-segment blank-node scope is ALREADY gone by the time a `Graph`
//! exists. This importer therefore interns every blank node under a single
//! flattened scope ([`BlankScope::DEFAULT`]) and that flattening IS the recorded
//! [`crate::loss::gts_to_rdf_loss_ledger`] `bnode-scope-flatten` intentional loss.
//! **Callers that need blank-node scope correctness MUST use
//! [`crate::import_gts_events`] (the event-sink path) instead.**
//!
//! Per the no-optionality / hard-fail doctrine, malformed structure (a dangling
//! term id, a non-IRI predicate/datatype, an unbound or cyclic quoted-triple term)
//! is an `Err`, never a silent skip.

use purrdf_core::cdt_blank::BlankBinding;
use purrdf_gts::model::{Graph, Term, TermKind};

use crate::{
    BlankScope, GtsBundle, RdfDatasetBuilder, RdfDiagnostic, RdfEnvelope, RdfLiteral, RdfLocation,
    TermId, TermRef,
};
use purrdf_lex::allocation::{Memory, Resident, StorageError};
use purrdf_lex::walk::WorkList;

/// The single, flattened blank-node scope every folded blank is interned under.
///
/// The folded `Graph` has already collapsed per-segment blank scope, so there is no
/// segment identity left to preserve here — every blank lands in
/// [`BlankScope::DEFAULT`]. This is the `bnode-scope-flatten` loss made concrete.
const FLATTENED_BLANK_SCOPE: BlankScope = BlankScope::DEFAULT;

/// The moved-out interning state: the term table (with owned strings progressively
/// `take`n out), the reifier bindings, and the GTS-id → [`TermId`] remap built so
/// far. Keeping these together lets a quoted-triple term resolve its components by
/// id WITHOUT re-borrowing the consumed [`Graph`].
struct GraphInterner {
    builder: RdfDatasetBuilder,
    /// Term table; leaf strings are MOVED out via `std::mem::take` as they intern.
    terms: Vec<Term>,
    /// Reifier-id → `(s, p, o)` component term ids.
    reifier_bindings: crate::FastMap<usize, (usize, usize, usize)>,
    /// GTS term id → interned [`TermId`]. A term interns at most once.
    remap: crate::FastMap<usize, Option<TermId>>,
    remap_bytes: usize,
}

#[derive(Clone, Copy)]
enum InternStep {
    Visit(usize),
    Finish(usize),
}

impl GraphInterner {
    /// Intern the term at `gts_id` (once), returning its [`TermId`]. Leaf kinds MOVE
    /// their owned strings out of `self.terms[gts_id]`; a quoted-triple term resolves
    /// its `(s, p, o)` over admitted continuations. Idempotent through the remap;
    /// a vacant remap answer marks an active node and preserves cycle refusal.
    fn intern(
        &mut self,
        gts_id: usize,
        memory: &mut Memory<'_, Resident>,
    ) -> Result<TermId, RdfDiagnostic> {
        if let Some(&Some(id)) = self.remap.get(&gts_id) {
            return Ok(id);
        }
        let mut pending: WorkList<InternStep, 32> = WorkList::new();
        let result = (|| {
            pending.try_push_admitted(InternStep::Visit(gts_id), memory)?;
            while let Some(step) = pending.pop() {
                match step {
                    InternStep::Visit(id) => {
                        match self.remap.get(&id) {
                            Some(Some(_)) => continue,
                            Some(None) => return Err(RdfDiagnostic::error(
                                "gts-self-reaching-term",
                                format!("GTS term {id} resolves through itself, so no walk of its components can terminate"),
                            ).with_location(self.location(id))),
                            None => {},
                        }
                        let term = self.terms.get(id).ok_or_else(|| {
                            RdfDiagnostic::error(
                                "gts-term-out-of-range",
                                format!("GTS term id {id} is out of range"),
                            )
                            .with_location(self.location(id))
                        })?;
                        let kind = term.kind;
                        let datatype = term.datatype;
                        let components = if kind == TermKind::Triple {
                            Some(self.triple_components(id)?)
                        } else {
                            None
                        };
                        let required = self
                            .remap
                            .len()
                            .checked_add(1)
                            .ok_or(StorageError::SizeOverflow)?;
                        purrdf_core::hash::reserve_map_with_retained_memory(
                            &mut self.remap,
                            &mut self.remap_bytes,
                            required,
                            memory,
                        )?;
                        self.remap.insert(id, None);
                        pending.try_push_admitted(InternStep::Finish(id), memory)?;
                        if let Some((s, p, o)) = components {
                            pending.try_push_admitted(InternStep::Visit(o), memory)?;
                            pending.try_push_admitted(InternStep::Visit(p), memory)?;
                            pending.try_push_admitted(InternStep::Visit(s), memory)?;
                        } else if kind == TermKind::Literal
                            && let Some(datatype) = datatype
                        {
                            pending.try_push_admitted(InternStep::Visit(datatype), memory)?;
                        }
                    }
                    InternStep::Finish(id) => {
                        let resolved = match self.terms[id].kind {
                            TermKind::Iri => self.intern_iri(id)?,
                            TermKind::Bnode => self.intern_blank(id)?,
                            TermKind::Literal => self.intern_literal(id)?,
                            TermKind::Triple => self.intern_triple(id)?,
                        };
                        self.remap.insert(id, Some(resolved));
                    }
                }
            }
            // Every scheduled root has completed before the work list empties.
            Ok(self.remap[&gts_id].expect("root completed its interning continuation"))
        })();
        let released = pending
            .release_admitted(memory)
            .map_err(RdfDiagnostic::from);
        let id = result?;
        released?;
        Ok(id)
    }

    fn intern_iri(&mut self, gts_id: usize) -> Result<TermId, RdfDiagnostic> {
        // MOVE the IRI string out of the term rather than cloning it.
        let value = std::mem::take(&mut self.terms[gts_id].value);
        let Some(iri) = value.filter(|value| !value.is_empty()) else {
            return Err(RdfDiagnostic::error(
                "gts-iri-missing-value",
                "GTS IRI term requires a non-empty value",
            )
            .with_location(self.location(gts_id)));
        };
        Ok(self.builder.intern_iri(&iri)?)
    }

    fn intern_blank(&mut self, gts_id: usize) -> Result<TermId, RdfDiagnostic> {
        // MOVE the blank label out; all folded blanks share the flattened scope.
        let label = std::mem::take(&mut self.terms[gts_id].value)
            .unwrap_or_else(|| format!("gts_bnode_{gts_id}"));
        Ok(self.builder.intern_blank(&label, FLATTENED_BLANK_SCOPE)?)
    }

    fn intern_literal(&mut self, gts_id: usize) -> Result<TermId, RdfDiagnostic> {
        // The datatype is a term id; resolve (and intern) it first, then read its
        // interned IRI string. This applies the SAME datatype-must-be-IRI rule the
        // shared resolver enforces.
        let datatype = match self.terms[gts_id].datatype {
            Some(dt_id) => Some(self.resolve_datatype_iri(dt_id)?),
            None => None,
        };
        // MOVE the lexical form and language tag out.
        let lexical_form = std::mem::take(&mut self.terms[gts_id].value).unwrap_or_default();
        let language = std::mem::take(&mut self.terms[gts_id].lang);
        let direction = crate::gts_resolve::parse_gts_direction(
            self.terms[gts_id].direction.as_deref(),
            language.as_deref(),
        )?;
        // A composite literal's embedded blank labels bind into the SAME
        // flattened scope `intern_blank` puts this reader's bare labels in, so
        // both spellings of one node stay one node across a GTS round trip.
        Ok(self.builder.intern_literal_bound(
            RdfLiteral {
                lexical_form,
                datatype,
                language,
                direction,
            },
            BlankBinding::Ambient(FLATTENED_BLANK_SCOPE),
        )?)
    }

    /// Resolve a quoted-triple term's components.
    ///
    /// A self-describing term (wire `"tt"`) states them itself. Only a term
    /// written in the older indirect spelling has to go through a reifier id —
    /// and since `rdf:reifies` is not functional, that id may bind several
    /// triples, of which such a term can only mean the first (the GTS reader
    /// raises `ConflictingReifier` for exactly that shape).
    fn triple_components(&self, gts_id: usize) -> Result<(usize, usize, usize), RdfDiagnostic> {
        match (self.terms[gts_id].triple, self.terms[gts_id].reifier) {
            (Some(components), _) => Ok(components),
            (None, Some(reifier_id)) => {
                let Some(&components) = self.reifier_bindings.get(&reifier_id) else {
                    return Err(RdfDiagnostic::error(
                        "gts-missing-reifier-binding",
                        format!("GTS triple term references missing reifier {reifier_id}"),
                    )
                    .with_location(self.location(gts_id).with_gts_reifier(reifier_id as u64)));
                };
                Ok(components)
            }
            (None, None) => Err(RdfDiagnostic::error(
                "gts-unbound-triple-term",
                "GTS triple term names neither its own components nor a reifier",
            )
            .with_location(self.location(gts_id))),
        }
    }

    fn intern_triple(&mut self, gts_id: usize) -> Result<TermId, RdfDiagnostic> {
        let (s, p, o) = self.triple_components(gts_id)?;
        // The Visit continuation completed all three dependencies first.
        let s = self.remap[&s].expect("subject dependency completed");
        let p = self.remap[&p].expect("predicate dependency completed");
        let o = self.remap[&o].expect("object dependency completed");
        Ok(self.builder.intern_triple(s, p, o)?)
    }

    /// Intern a literal datatype term (which MUST resolve to an IRI) and return its
    /// IRI string for the literal interner.
    fn resolve_datatype_iri(&self, dt_id: usize) -> Result<String, RdfDiagnostic> {
        // The literal's Visit continuation completed its datatype first.
        let id = self.remap[&dt_id].expect("datatype dependency completed");
        match self.builder.resolve(id) {
            TermRef::Iri(iri) => Ok(iri.to_string()),
            other => Err(RdfDiagnostic::error(
                "gts-literal-datatype-not-iri",
                format!("GTS literal datatype must resolve to an IRI, got {other:?}"),
            )
            .with_location(self.location(dt_id))),
        }
    }

    /// Resolve an already-(or just-)interned term id, hard-failing on a dangling
    /// reference from a quad / reifier / annotation row.
    fn resolve_row_term(
        &mut self,
        gts_id: usize,
        role: &str,
        location: &RdfLocation,
        memory: &mut Memory<'_, Resident>,
    ) -> Result<TermId, RdfDiagnostic> {
        if gts_id >= self.terms.len() {
            return Err(RdfDiagnostic::error(
                "rdf-ir-dangling-term-ref",
                format!("GTS {role} references term id {gts_id}, which no term introduced"),
            )
            .with_location(location.clone().with_gts_term(gts_id as u64)));
        }
        self.intern(gts_id, memory)
    }

    fn location(&self, gts_id: usize) -> RdfLocation {
        RdfLocation::logical("gts:graph").with_gts_term(gts_id as u64)
    }
}

/// Consume a folded GTS [`Graph`] by value, MOVING owned term strings into the
/// interner, and return the frozen [`GtsBundle`].
///
/// Because `reader::read()` folds all segments, per-segment blank-node scope is
/// already lost; this importer records the `bnode-scope-flatten` intentional loss
/// (see [`crate::loss::gts_to_rdf_loss_ledger`]). Callers needing scope correctness
/// MUST use [`crate::import_gts_events`] (`import_sink.rs`) instead.
///
/// Malformed structure (dangling ids, non-IRI predicate/datatype, unbound or cyclic
/// quoted-triple terms) hard-fails as `Err`.
pub fn import_gts_graph(graph: Graph) -> Result<GtsBundle, RdfDiagnostic> {
    Memory::new(&mut Resident).try_scope(|memory| import_graph_with_memory(graph, memory))
}

fn import_graph_with_memory(
    graph: Graph,
    memory: &mut Memory<'_, Resident>,
) -> Result<GtsBundle, RdfDiagnostic> {
    // Read the envelope/lookaside from the SAME graph fields `gts.rs` reads, BEFORE
    // moving term strings out (it touches only meta/segment/blob/etc., never the
    // term `value`/`lang` we move).
    let lookaside = crate::gts_core::lookaside_from_graph(&graph);

    let Graph {
        terms,
        quads,
        reifiers,
        annotations,
        ..
    } = graph;

    // Reifier rows are `(reifier_id, (s,p,o), graph?)`. This map serves ONE purpose —
    // resolving a LEGACY GTS triple TERM, one whose `Term.reifier` slot names a reifier
    // id instead of stating its own components. That is a shape question, not a graph
    // question, so the graph slot is not part of this lookup's key. Nothing is dropped
    // from the IR: the statement layer is keyed per graph, and the binding loop below
    // carries EVERY row (including several rows for one reifier id, which
    // `rdf:reifies` being non-functional makes legal) through
    // `push_reifier_in_graph` / `push_annotation_in_graph`. Only this legacy lookup is
    // single-valued, and it keeps the FIRST binding to match `Graph::reifier`.
    let mut reifier_bindings: crate::FastMap<usize, (usize, usize, usize)> =
        crate::FastMap::default();
    for &(reifier_id, triple, _graph) in &reifiers {
        reifier_bindings.entry(reifier_id).or_insert(triple);
    }

    let mut interner = GraphInterner {
        builder: RdfDatasetBuilder::new(),
        terms,
        reifier_bindings,
        remap: crate::FastMap::default(),
        remap_bytes: 0,
    };

    // Intern every term up front (idempotent through the remap), so leaf strings are
    // moved out in one forward sweep and triple terms resolve their components.
    for gts_id in 0..interner.terms.len() {
        interner.intern(gts_id, memory)?;
    }

    // Quads, resolved through the remap; a dangling id hard-fails.
    for (index, (s, p, o, g)) in quads.iter().copied().enumerate() {
        let location = RdfLocation::logical("gts:quad").with_gts_quad(index as u64);
        let s = interner.resolve_row_term(s, "quad subject", &location, memory)?;
        let p = interner.resolve_row_term(p, "quad predicate", &location, memory)?;
        let o = interner.resolve_row_term(o, "quad object", &location, memory)?;
        let g = match g {
            Some(g) => Some(interner.resolve_row_term(g, "quad graph name", &location, memory)?),
            None => None,
        };
        let handle = interner.builder.push_quad_with_handle(s, p, o, g)?;
        interner.builder.attach_location(handle, location);
    }

    // Reifier bindings: bind the reifier resource to the interned triple term, carrying
    // the reifier declaration's own named graph (`None` = default graph).
    for (reifier_id, (s, p, o), graph) in reifiers.iter().copied() {
        let location = RdfLocation::logical("gts:reifier").with_gts_reifier(reifier_id as u64);
        let reifier = interner.resolve_row_term(reifier_id, "reifier", &location, memory)?;
        let s = interner.resolve_row_term(s, "reified subject", &location, memory)?;
        let p = interner.resolve_row_term(p, "reified predicate", &location, memory)?;
        let o = interner.resolve_row_term(o, "reified object", &location, memory)?;
        let g = graph
            .map(|g| interner.resolve_row_term(g, "reifier graph", &location, memory))
            .transpose()?;
        let triple = interner.builder.intern_triple(s, p, o)?;
        interner.builder.push_reifier_in_graph(reifier, triple, g)?;
    }

    // Annotations `(reifier, predicate, value, graph?)`.
    for (r, p, v, graph) in annotations.iter().copied() {
        let location = RdfLocation::logical("gts:annotation").with_gts_reifier(r as u64);
        let r = interner.resolve_row_term(r, "annotation reifier", &location, memory)?;
        let p = interner.resolve_row_term(p, "annotation predicate", &location, memory)?;
        let v = interner.resolve_row_term(v, "annotation object", &location, memory)?;
        let g = graph
            .map(|g| interner.resolve_row_term(g, "annotation graph", &location, memory))
            .transpose()?;
        interner.builder.push_annotation_in_graph(r, p, v, g)?;
    }

    let GraphInterner {
        builder,
        remap,
        remap_bytes,
        ..
    } = interner;
    drop(remap);
    memory.release_bytes(remap_bytes)?;
    let dataset = builder.freeze()?;
    Ok(GtsBundle::new(dataset, RdfEnvelope::new(lookaside)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::TermRef;
    use purrdf_gts::model::{Term as GtsTerm, TermKind as GtsKind};

    fn iri_term(value: &str) -> GtsTerm {
        GtsTerm {
            kind: GtsKind::Iri,
            value: Some(value.to_owned()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
            triple: None,
        }
    }

    fn blank_term(label: &str) -> GtsTerm {
        GtsTerm {
            kind: GtsKind::Bnode,
            value: Some(label.to_owned()),
            datatype: None,
            lang: None,
            direction: None,
            reifier: None,
            triple: None,
        }
    }

    /// A simple single-quad graph imports, with the IRI value preserved.
    #[test]
    fn imports_simple_graph() {
        let mut graph = Graph::default();
        graph.terms.push(iri_term("http://example.org/s"));
        graph.terms.push(iri_term("http://example.org/p"));
        graph.terms.push(iri_term("http://example.org/o"));
        graph.quads.push((0, 1, 2, None));

        let bundle = import_gts_graph(graph).expect("import");
        assert_eq!(bundle.dataset.quad_count(), 1);
        let q = bundle.dataset.quad_refs().next().expect("one quad");
        match q.s {
            TermRef::Iri(s) => assert_eq!(s, "http://example.org/s"),
            other => panic!("expected iri, got {other:?}"),
        }
    }

    #[test]
    fn duplicate_quads_keep_locations_on_their_actual_rows() {
        let mut graph = Graph::default();
        graph.terms.push(iri_term("http://example.org/p"));
        graph.terms.push(iri_term("http://example.org/o"));
        for index in 0..64 {
            graph
                .terms
                .push(iri_term(&format!("http://example.org/s{index}")));
        }
        // Reverse push order forces location handles through the frozen sort;
        // duplicates cannot lend their source index to the next distinct row.
        for index in (2..66).rev() {
            graph.quads.extend([(index, 0, 1, None); 3]);
        }
        let original_subjects: Vec<_> = graph
            .quads
            .iter()
            .map(|quad| graph.terms[quad.0].value.clone().expect("IRI value"))
            .collect();
        let bundle = import_gts_graph(graph).expect("duplicate rows are valid RDF");
        assert_eq!(bundle.dataset.quad_count(), 64);
        for (index, quad) in bundle.dataset.quads().enumerate() {
            let handle = crate::ir::QuadHandle::from_index(u32::try_from(index).expect("index"));
            let original = bundle
                .dataset
                .location_of(handle)
                .expect("every imported row has a location")
                .gts_quad_index
                .expect("the location identifies a GTS row");
            assert_eq!(
                bundle.dataset.resolve(quad.s),
                TermRef::Iri(
                    &original_subjects
                        [usize::try_from(original).expect("the source row fits its input vector")]
                ),
                "a source location must identify a matching input statement"
            );
        }
    }

    /// All folded blanks land in the single flattened scope (the recorded loss).
    #[test]
    fn folded_blanks_flatten_to_default_scope() {
        let mut graph = Graph::default();
        graph.terms.push(iri_term("http://example.org/s"));
        graph.terms.push(iri_term("http://example.org/p"));
        graph.terms.push(blank_term("b1"));
        graph.quads.push((0, 1, 2, None));

        let bundle = import_gts_graph(graph).expect("import");
        let q = bundle.dataset.quad_refs().next().expect("one quad");
        match q.o {
            TermRef::Blank { label, scope } => {
                assert_eq!(label, "b1");
                assert_eq!(
                    scope,
                    BlankScope::DEFAULT,
                    "folded blank flattens to scope 0"
                );
            }
            other => panic!("expected blank, got {other:?}"),
        }
    }

    /// A dangling quad object id (no introducing term) is a hard `Err`.
    #[test]
    fn dangling_object_is_err() {
        let mut graph = Graph::default();
        graph.terms.push(iri_term("http://example.org/s"));
        graph.terms.push(iri_term("http://example.org/p"));
        // Object id 9 was never introduced.
        graph.quads.push((0, 1, 9, None));
        let err = import_gts_graph(graph).expect_err("dangling id must fail");
        assert_eq!(err.code, "rdf-ir-dangling-term-ref");
    }

    /// An empty-valued IRI term hard-fails.
    #[test]
    fn empty_iri_is_err() {
        let mut graph = Graph::default();
        graph.terms.push(iri_term(""));
        let err = import_gts_graph(graph).expect_err("empty IRI must fail");
        assert_eq!(err.code, "gts-iri-missing-value");
    }

    /// A nested quoted-triple term survives the consuming path: the outer triple's
    /// object IS the inner triple term.
    #[test]
    fn nested_triple_term_survives_graph_path() {
        let mut graph = Graph::default();
        graph.terms.push(iri_term("http://example.org/a")); // 0
        graph.terms.push(iri_term("http://example.org/p")); // 1
        graph.terms.push(iri_term("http://example.org/b")); // 2
        graph.terms.push(iri_term("http://example.org/r0")); // 3 reifier resource
        graph.reifiers.push((3, (0, 1, 2), None));
        graph.terms.push(GtsTerm {
            kind: GtsKind::Triple,
            value: None,
            datatype: None,
            lang: None,
            direction: None,
            reifier: Some(3),
            triple: None,
        }); // 4 inner triple term
        graph.terms.push(iri_term("http://example.org/asserts")); // 5
        graph.terms.push(iri_term("http://example.org/r1")); // 6 reifier resource
        graph.reifiers.push((6, (0, 5, 4), None));
        graph.terms.push(GtsTerm {
            kind: GtsKind::Triple,
            value: None,
            datatype: None,
            lang: None,
            direction: None,
            reifier: Some(6),
            triple: None,
        }); // 7 outer triple term
        graph.quads.push((0, 5, 7, None));

        let bundle = import_gts_graph(graph).expect("nested import");
        let q = bundle.dataset.quad_refs().next().expect("one quad");
        match q.o {
            TermRef::Triple { .. } => {}
            other => panic!("expected outer triple term object, got {other:?}"),
        }
    }

    /// A cyclic quoted-triple term remains a hard failure during iterative resolution.
    #[test]
    fn cyclic_triple_term_retains_explicit_cycle_refusal() {
        let mut graph = Graph::default();
        // Triple term 0 bound to reifier 0, whose object is term 0 itself.
        graph.terms.push(GtsTerm {
            kind: GtsKind::Triple,
            value: None,
            datatype: None,
            lang: None,
            direction: None,
            reifier: Some(0),
            triple: None,
        }); // 0
        graph.terms.push(iri_term("http://example.org/p")); // 1
        graph.reifiers.push((0, (1, 1, 0), None));
        let err = import_gts_graph(graph).expect_err("cycle must fail");
        assert_eq!(err.code, "gts-self-reaching-term");
    }

    /// A literal carries its lexical form and (lowercased) language through the move.
    #[test]
    fn literal_lexical_and_lang_survive() {
        let mut graph = Graph::default();
        graph.terms.push(iri_term("http://example.org/s"));
        graph.terms.push(iri_term("http://example.org/p"));
        graph.terms.push(GtsTerm {
            kind: GtsKind::Literal,
            value: Some("Bonjour".to_owned()),
            datatype: None,
            lang: Some("FR".to_owned()),
            direction: None,
            reifier: None,
            triple: None,
        });
        graph.quads.push((0, 1, 2, None));

        let bundle = import_gts_graph(graph).expect("import");
        let q = bundle.dataset.quad_refs().next().expect("one quad");
        match q.o {
            TermRef::Literal {
                lexical, language, ..
            } => {
                assert_eq!(lexical, "Bonjour");
                assert_eq!(language, Some("fr"), "language lowercased per C0.1");
            }
            other => panic!("expected literal, got {other:?}"),
        }
    }

    /// The loss ledger documents the `bnode-scope-flatten` loss this path incurs.
    #[test]
    fn loss_ledger_documents_bnode_scope_flatten() {
        let ledger = crate::loss::gts_to_rdf_loss_ledger();
        assert!(
            ledger
                .entries()
                .iter()
                .any(|e| e.code == "bnode-scope-flatten"),
            "import_gts_graph flattens blank scope; the ledger MUST document it"
        );
        // `intentional` is derived as membership in `profile_for(from, to)` (see
        // `LossLedger::render`'s `intentional` binding), so proving this is a
        // *declared* (in-profile), not merely present, loss requires checking the
        // gts -> rdf-1.2-dataset profile directly.
        assert!(
            crate::loss::profile_for("gts", "rdf-1.2-dataset").contains("bnode-scope-flatten"),
            "gts->rdf-1.2-dataset profile must declare bnode-scope-flatten as an intentional \
             (in-profile) loss"
        );
    }
}

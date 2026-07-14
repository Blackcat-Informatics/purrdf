// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

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

use std::collections::HashMap;

use purrdf_gts::model::{Graph, Term, TermKind};

use crate::gts_resolve::MAX_GTS_TERM_NESTING_DEPTH;
use crate::{
    BlankScope, GtsBundle, RdfDatasetBuilder, RdfDiagnostic, RdfEnvelope, RdfLiteral, RdfLocation,
    TermId, TermRef,
};

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
    reifier_bindings: HashMap<usize, (usize, usize, usize)>,
    /// GTS term id → interned [`TermId`]. A term interns at most once.
    remap: HashMap<usize, TermId>,
}

impl GraphInterner {
    /// Intern the term at `gts_id` (once), returning its [`TermId`]. Leaf kinds MOVE
    /// their owned strings out of `self.terms[gts_id]`; a quoted-triple term resolves
    /// its `(s, p, o)` recursively, depth-bounded. Idempotent through the remap.
    fn intern(&mut self, gts_id: usize, depth: usize) -> Result<TermId, RdfDiagnostic> {
        if let Some(&id) = self.remap.get(&gts_id) {
            return Ok(id);
        }
        if depth > MAX_GTS_TERM_NESTING_DEPTH {
            return Err(RdfDiagnostic::error(
                "gts-term-nesting-limit",
                "GTS term nesting depth limit exceeded",
            )
            .with_location(self.location(gts_id)));
        }
        let kind = self
            .terms
            .get(gts_id)
            .ok_or_else(|| {
                RdfDiagnostic::error(
                    "gts-term-out-of-range",
                    format!("GTS term id {gts_id} is out of range"),
                )
                .with_location(self.location(gts_id))
            })?
            .kind;

        let id = match kind {
            TermKind::Iri => self.intern_iri(gts_id)?,
            TermKind::Bnode => self.intern_blank(gts_id),
            TermKind::Literal => self.intern_literal(gts_id, depth)?,
            TermKind::Triple => self.intern_triple(gts_id, depth)?,
        };
        self.remap.insert(gts_id, id);
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
        Ok(self.builder.intern_iri(&iri))
    }

    fn intern_blank(&mut self, gts_id: usize) -> TermId {
        // MOVE the blank label out; all folded blanks share the flattened scope.
        let label = std::mem::take(&mut self.terms[gts_id].value)
            .unwrap_or_else(|| format!("gts_bnode_{gts_id}"));
        self.builder.intern_blank(&label, FLATTENED_BLANK_SCOPE)
    }

    fn intern_literal(&mut self, gts_id: usize, depth: usize) -> Result<TermId, RdfDiagnostic> {
        // The datatype is a term id; resolve (and intern) it first, then read its
        // interned IRI string. This applies the SAME datatype-must-be-IRI rule the
        // shared resolver enforces.
        let datatype = match self.terms[gts_id].datatype {
            Some(dt_id) => Some(self.resolve_datatype_iri(dt_id, depth + 1)?),
            None => None,
        };
        // MOVE the lexical form and language tag out.
        let lexical_form = std::mem::take(&mut self.terms[gts_id].value).unwrap_or_default();
        let language = std::mem::take(&mut self.terms[gts_id].lang);
        let direction = crate::gts_resolve::parse_gts_direction(
            self.terms[gts_id].direction.as_deref(),
            language.as_deref(),
        )?;
        Ok(self.builder.intern_literal(RdfLiteral {
            lexical_form,
            datatype,
            language,
            direction,
        }))
    }

    fn intern_triple(&mut self, gts_id: usize, depth: usize) -> Result<TermId, RdfDiagnostic> {
        let Some(reifier_id) = self.terms[gts_id].reifier else {
            return Err(RdfDiagnostic::error(
                "gts-unbound-triple-term",
                "GTS triple term has no reifier binding",
            )
            .with_location(self.location(gts_id)));
        };
        let Some(&(s, p, o)) = self.reifier_bindings.get(&reifier_id) else {
            return Err(RdfDiagnostic::error(
                "gts-missing-reifier-binding",
                format!("GTS triple term references missing reifier {reifier_id}"),
            )
            .with_location(self.location(gts_id).with_gts_reifier(reifier_id)));
        };
        let s = self.intern(s, depth + 1)?;
        let p = self.intern(p, depth + 1)?;
        let o = self.intern(o, depth + 1)?;
        Ok(self.builder.intern_triple(s, p, o))
    }

    /// Intern a literal datatype term (which MUST resolve to an IRI) and return its
    /// IRI string for the literal interner.
    fn resolve_datatype_iri(
        &mut self,
        dt_id: usize,
        depth: usize,
    ) -> Result<String, RdfDiagnostic> {
        let id = self.intern(dt_id, depth)?;
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
    ) -> Result<TermId, RdfDiagnostic> {
        if gts_id >= self.terms.len() {
            return Err(RdfDiagnostic::error(
                "rdf-ir-dangling-term-ref",
                format!("GTS {role} references term id {gts_id}, which no term introduced"),
            )
            .with_location(location.clone().with_gts_term(gts_id)));
        }
        self.intern(gts_id, 0)
    }

    fn location(&self, gts_id: usize) -> RdfLocation {
        RdfLocation::logical("gts:graph").with_gts_term(gts_id)
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

    // purrdf-gts 0.9.11 reifier rows are `(reifier_id, (s,p,o), graph?)`. The IR statement
    // layer has no graph dimension (reification is standpoint-scoped), so the binding map
    // drops the graph slot; a non-`None` graph is rejected in the binding loop below.
    let reifier_bindings: HashMap<usize, (usize, usize, usize)> = reifiers
        .iter()
        .map(|&(reifier_id, triple, _graph)| (reifier_id, triple))
        .collect();

    let mut interner = GraphInterner {
        builder: RdfDatasetBuilder::new(),
        terms,
        reifier_bindings,
        remap: HashMap::new(),
    };

    // Intern every term up front (idempotent through the remap), so leaf strings are
    // moved out in one forward sweep and triple terms resolve their components.
    for gts_id in 0..interner.terms.len() {
        interner.intern(gts_id, 0)?;
    }

    // Quads, resolved through the remap; a dangling id hard-fails.
    for (index, (s, p, o, g)) in quads.iter().copied().enumerate() {
        let location = RdfLocation::logical("gts:quad").with_gts_quad(index);
        let s = interner.resolve_row_term(s, "quad subject", &location)?;
        let p = interner.resolve_row_term(p, "quad predicate", &location)?;
        let o = interner.resolve_row_term(o, "quad object", &location)?;
        let g = match g {
            Some(g) => Some(interner.resolve_row_term(g, "quad graph name", &location)?),
            None => None,
        };
        let handle = interner.builder.next_quad_handle();
        interner.builder.push_quad(s, p, o, g);
        interner.builder.attach_location(handle, location);
    }

    // Reifier bindings: bind the reifier resource to the interned triple term, carrying
    // the reifier declaration's own named graph (`None` = default graph).
    for (reifier_id, (s, p, o), graph) in reifiers.iter().copied() {
        let location = RdfLocation::logical("gts:reifier").with_gts_reifier(reifier_id);
        let reifier = interner.resolve_row_term(reifier_id, "reifier", &location)?;
        let s = interner.resolve_row_term(s, "reified subject", &location)?;
        let p = interner.resolve_row_term(p, "reified predicate", &location)?;
        let o = interner.resolve_row_term(o, "reified object", &location)?;
        let g = graph
            .map(|g| interner.resolve_row_term(g, "reifier graph", &location))
            .transpose()?;
        let triple = interner.builder.intern_triple(s, p, o);
        interner.builder.push_reifier_in_graph(reifier, triple, g);
    }

    // Annotations `(reifier, predicate, value, graph?)`.
    for (r, p, v, graph) in annotations.iter().copied() {
        let location = RdfLocation::logical("gts:annotation").with_gts_reifier(r);
        let r = interner.resolve_row_term(r, "annotation reifier", &location)?;
        let p = interner.resolve_row_term(p, "annotation predicate", &location)?;
        let v = interner.resolve_row_term(v, "annotation object", &location)?;
        let g = graph
            .map(|g| interner.resolve_row_term(g, "annotation graph", &location))
            .transpose()?;
        interner.builder.push_annotation_in_graph(r, p, v, g);
    }

    let dataset = interner.builder.freeze()?;
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
        }); // 7 outer triple term
        graph.quads.push((0, 5, 7, None));

        let bundle = import_gts_graph(graph).expect("nested import");
        let q = bundle.dataset.quad_refs().next().expect("one quad");
        match q.o {
            TermRef::Triple { .. } => {}
            other => panic!("expected outer triple term object, got {other:?}"),
        }
    }

    /// A cyclic quoted-triple term (a triple that nests itself) hits the depth bound
    /// and hard-fails rather than recursing forever.
    #[test]
    fn cyclic_triple_term_hits_nesting_limit() {
        let mut graph = Graph::default();
        // Triple term 0 bound to reifier 0, whose object is term 0 itself.
        graph.terms.push(GtsTerm {
            kind: GtsKind::Triple,
            value: None,
            datatype: None,
            lang: None,
            direction: None,
            reifier: Some(0),
        }); // 0
        graph.terms.push(iri_term("http://example.org/p")); // 1
        graph.reifiers.push((0, (1, 1, 0), None));
        let err = import_gts_graph(graph).expect_err("cycle must hit nesting limit");
        assert_eq!(err.code, "gts-term-nesting-limit");
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

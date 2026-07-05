// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The RDF/JS [Sink](https://rdf.js.org/stream-spec/#sink-interface) consumer, over
//! the `purrdf-events` P6 ingestion protocol.
//!
//! [`Sink`] is a streaming dataset builder: JS pushes complete [`Quad`]s, each is
//! emitted into the engine's [`DatasetSink`] as `term` + `quad` events (a fresh
//! protocol-local id per distinct term value), and `finish()` runs the protocol's
//! two-phase resolution — the same seam every parser uses — yielding a [`Dataset`].
//!
//! The asynchronous, `EventEmitter`-based RDF/JS `Stream` and `Sink.import(stream)`
//! surfaces are presented by the TypeScript wrapper over this synchronous push API and
//! over [`Dataset::quads`](crate::Dataset) — wasm is synchronous, so async I/O is a
//! JS-layer concern.

use std::collections::HashMap;

use purrdf::ir::MutableDataset;
use purrdf::{DatasetSink, RdfTerm, RdfTextDirection};
use purrdf_events::{
    EventQuad, EventTerm, EventTermId, EventTriple, RdfEventSink, ScopeId, TextDirection,
};
use wasm_bindgen::prelude::*;

use crate::dataset::Dataset;
use crate::term::{Quad, TermInner, XSD_STRING, canonicalize_literal};

fn to_event_direction(direction: RdfTextDirection) -> TextDirection {
    match direction {
        RdfTextDirection::Ltr => TextDirection::Ltr,
        RdfTextDirection::Rtl => TextDirection::Rtl,
    }
}

/// An RDF/JS `Sink` — a streaming consumer that interns pushed quads through the
/// `purrdf-events` protocol and freezes them at `finish()`.
#[wasm_bindgen]
#[derive(Debug)]
pub struct Sink {
    /// `None` after `finish()` (the protocol sink is consumed to produce the dataset).
    inner: Option<DatasetSink>,
    /// Dedup: a distinct term value is declared once and its protocol id reused.
    ids: HashMap<RdfTerm, EventTermId>,
    /// The next protocol-local [`EventTermId`] to mint (drive-global, monotonic).
    next_id: u32,
}

impl Default for Sink {
    fn default() -> Self {
        Self {
            inner: Some(DatasetSink::new()),
            ids: HashMap::new(),
            next_id: 0,
        }
    }
}

impl Sink {
    fn mint(&mut self) -> EventTermId {
        let id = EventTermId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Declare a term (deduplicated by value) and return its protocol id, emitting any
    /// nested triple-term components first so no forward reference is needed.
    fn emit_term(&mut self, term: &RdfTerm) -> Result<EventTermId, String> {
        if let Some(id) = self.ids.get(term) {
            return Ok(*id);
        }
        let id = match term {
            RdfTerm::Triple(triple) => {
                let s = self.emit_term(&triple.subject)?;
                let p = self.emit_term(&RdfTerm::Iri(triple.predicate.clone()))?;
                let o = self.emit_term(&triple.object)?;
                let id = self.mint();
                // ControlFlow is ignored: DatasetSink is an accumulator that never
                // signals Break (cancellation is a source/driver concern, not a sink).
                let _ = self
                    .sink_mut()?
                    .term(id, EventTerm::Triple(EventTriple { s, p, o }))
                    .map_err(|e| e.to_string())?;
                id
            }
            RdfTerm::Iri(iri) => {
                let id = self.mint();
                let _ = self
                    .sink_mut()?
                    .term(id, EventTerm::Iri(iri))
                    .map_err(|e| e.to_string())?;
                id
            }
            RdfTerm::BlankNode(label) => {
                let id = self.mint();
                let _ = self
                    .sink_mut()?
                    .term(
                        id,
                        EventTerm::Blank {
                            label,
                            scope: ScopeId::DEFAULT,
                        },
                    )
                    .map_err(|e| e.to_string())?;
                id
            }
            RdfTerm::Literal(lit) => {
                let canonical = canonicalize_literal(lit.clone());
                let datatype = canonical.datatype.as_deref().unwrap_or(XSD_STRING);
                let direction = canonical.direction.map(to_event_direction);
                let id = self.mint();
                let _ = self
                    .sink_mut()?
                    .term(
                        id,
                        EventTerm::Literal {
                            lexical: &canonical.lexical_form,
                            datatype,
                            language: canonical.language.as_deref(),
                            direction,
                        },
                    )
                    .map_err(|e| e.to_string())?;
                id
            }
        };
        self.ids.insert(term.clone(), id);
        Ok(id)
    }

    fn sink_mut(&mut self) -> Result<&mut DatasetSink, String> {
        self.inner
            .as_mut()
            .ok_or_else(|| "the sink has already been finished".to_owned())
    }

    fn push_inner(&mut self, quad: &Quad) -> Result<(), String> {
        let subject = quad.subject.to_rdf_term()?;
        let predicate = match &quad.predicate.inner {
            TermInner::Named(iri) => RdfTerm::Iri(iri.clone()),
            _ => return Err("a quad predicate must be a NamedNode".to_owned()),
        };
        let object = quad.object.to_rdf_term()?;
        let graph = match &quad.graph.inner {
            TermInner::DefaultGraph => None,
            TermInner::Named(_) | TermInner::Blank(_) => Some(quad.graph.to_rdf_term()?),
            _ => {
                return Err(
                    "a quad graph must be a NamedNode, BlankNode, or DefaultGraph".to_owned(),
                );
            }
        };

        let s = self.emit_term(&subject)?;
        let p = self.emit_term(&predicate)?;
        let o = self.emit_term(&object)?;
        let g = match &graph {
            Some(g) => Some(self.emit_term(g)?),
            None => None,
        };
        let _ = self
            .sink_mut()?
            .quad(EventQuad { s, p, o, g })
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[wasm_bindgen]
impl Sink {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    /// `push(quad)` — stream one quad into the sink (interned via the event protocol).
    #[wasm_bindgen(js_name = push)]
    pub fn push(&mut self, quad: &Quad) -> Result<(), JsError> {
        self.push_inner(quad).map_err(|e| JsError::new(&e))
    }

    /// `finish()` — run the protocol's forward-reference resolution and return the
    /// resulting dataset. The sink is consumed; further `push`/`finish` is an error.
    #[wasm_bindgen(js_name = finish)]
    pub fn finish(&mut self) -> Result<Dataset, JsError> {
        let mut sink = self
            .inner
            .take()
            .ok_or_else(|| JsError::new("the sink has already been finished"))?;
        sink.finish().map_err(|e| JsError::new(&e.to_string()))?;
        let dataset = sink
            .into_dataset()
            .ok_or_else(|| JsError::new("the sink produced no dataset"))?;
        Ok(Dataset {
            inner: MutableDataset::new(dataset),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::term::Term;

    fn named(iri: &str) -> Term {
        Term::from_inner(TermInner::Named(iri.to_owned()))
    }

    fn triple_quad(s: &str, p: &str, o: &str) -> Quad {
        Quad::from_parts(
            named(s),
            named(p),
            named(o),
            Term::from_inner(TermInner::DefaultGraph),
        )
    }

    #[test]
    fn push_then_finish_builds_a_dataset() {
        let mut sink = Sink::new();
        sink.push_inner(&triple_quad("https://e/s", "https://e/p", "https://e/o"))
            .unwrap();
        sink.push_inner(&triple_quad("https://e/s2", "https://e/p", "https://e/o2"))
            .unwrap();
        let ds = sink.finish().unwrap();
        assert_eq!(ds.size(), 2);
    }

    #[test]
    fn repeated_terms_are_deduplicated() {
        let mut sink = Sink::new();
        // Same subject + predicate across two quads → those term ids are reused.
        sink.push_inner(&triple_quad("https://e/s", "https://e/p", "https://e/o1"))
            .unwrap();
        sink.push_inner(&triple_quad("https://e/s", "https://e/p", "https://e/o2"))
            .unwrap();
        // s, p, o1, o2 → 4 distinct term ids (s and p reused on the second push).
        assert_eq!(sink.next_id, 4);
        let ds = sink.finish().unwrap();
        assert_eq!(ds.size(), 2);
    }

    #[test]
    fn quoted_triple_term_streams_through_the_protocol() {
        use purrdf::RdfTriple;
        // << s p o >> as the object of an outer quad — the RDF-1.2 wedge through the
        // event protocol (components emitted before the triple term, no forward ref).
        let inner = RdfTriple::new(
            RdfTerm::iri("https://e/s"),
            "https://e/p",
            RdfTerm::iri("https://e/o"),
        );
        let quoted = Term::from_inner(TermInner::Quoted(Box::new(inner)));
        let q = Quad::from_parts(
            named("https://e/stmt"),
            named("https://e/asserts"),
            quoted,
            Term::from_inner(TermInner::DefaultGraph),
        );
        let mut sink = Sink::new();
        sink.push_inner(&q).unwrap();
        let ds = sink.finish().unwrap();
        assert_eq!(ds.size(), 1);
    }

    #[test]
    fn finish_twice_is_an_error() {
        let mut sink = Sink::new();
        sink.push_inner(&triple_quad("https://e/s", "https://e/p", "https://e/o"))
            .unwrap();
        let _ = sink.finish().unwrap();
        // After finish, the protocol sink is consumed.
        assert!(
            sink.push_inner(&triple_quad("https://e/s", "https://e/p", "https://e/o"))
                .is_err()
        );
    }
}

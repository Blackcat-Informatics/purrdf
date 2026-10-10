// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

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

use purrdf::ir::{Nested, try_fold_nested};
use purrdf::{DatasetSink, TermValue};
use purrdf_core::FastMap;
use purrdf_events::{EventQuad, EventTerm, EventTermId, EventTriple, RdfEventSink, ScopeId};
use wasm_bindgen::prelude::*;

use crate::convert::quad_to_quad_values;
use crate::dataset::Dataset;
use crate::term::Quad;

/// An RDF/JS `Sink` — a streaming consumer that interns pushed quads through the
/// `purrdf-events` protocol and freezes them at `finish()`.
#[wasm_bindgen]
#[derive(Debug)]
pub struct Sink {
    /// `None` after `finish()` (the protocol sink is consumed to produce the dataset).
    inner: Option<DatasetSink>,
    /// Leaf values are declared once and their protocol ids reused.
    ids: FastMap<TermValue, EventTermId>,
    /// Triple identities are their canonical component ids, without copied suffix trees.
    triple_ids: FastMap<(EventTermId, EventTermId, EventTermId), EventTermId>,
    /// Largest opened protocol scope, preserving the core blank-node ordinal identity.
    opened_scope: u32,
    /// The next protocol-local [`EventTermId`] to mint (drive-global, monotonic).
    next_id: Option<u64>,
}

impl Default for Sink {
    fn default() -> Self {
        Self {
            inner: Some(DatasetSink::new()),
            ids: FastMap::default(),
            triple_ids: FastMap::default(),
            opened_scope: 0,
            next_id: Some(0),
        }
    }
}

impl Sink {
    fn mint(&mut self) -> Result<EventTermId, String> {
        let next = self
            .next_id
            .ok_or_else(|| purrdf_events::EventError::IdSpaceExhausted.to_string())?;
        self.next_id = next.checked_add(1);
        Ok(EventTermId(next))
    }

    /// Declare a term (deduplicated by value) and return its protocol id, emitting any
    /// nested triple-term components first so no forward reference is needed.
    ///
    /// A triple term is emitted over [`try_fold_nested`]'s work list: its subject, its
    /// predicate IRI and its object, each fully before the next, then the triple
    /// itself. Leaves are memoized by value, triples by their canonical component ids;
    /// a nested chain therefore retains one fixed-size key per triple.
    fn emit_term(&mut self, term: &TermValue) -> Result<EventTermId, String> {
        try_fold_nested(
            term,
            self,
            |sink, term| match term {
                TermValue::Triple { s, p, o } => Ok(Nested::Triple(&**s, &**p, &**o)),
                _ => sink.emit_leaf(term).map(Nested::Leaf),
            },
            |sink, _, s, p, o| {
                if let Some(id) = sink.triple_ids.get(&(s, p, o)) {
                    return Ok(*id);
                }
                let id = sink.mint()?;
                let _ = sink
                    .sink_mut()?
                    .term(id, EventTerm::Triple(EventTriple { s, p, o }))
                    .map_err(|e| e.to_string())?;
                sink.triple_ids.insert((s, p, o), id);
                Ok(id)
            },
        )
    }

    /// Declare a term that is not a triple term (deduplicated by value) and return its
    /// protocol id.
    fn emit_leaf(&mut self, term: &TermValue) -> Result<EventTermId, String> {
        if let Some(id) = self.ids.get(term) {
            return Ok(*id);
        }
        if let TermValue::Blank { scope, .. } = term {
            while self.opened_scope < scope.0 {
                self.opened_scope = self.sink_mut()?.open_scope().map_err(|e| e.to_string())?.0;
            }
        }
        let id = self.mint()?;
        let event = match term {
            TermValue::Triple { .. } => unreachable!("a triple term is emitted from its parts"),
            TermValue::Iri(iri) => EventTerm::Iri(iri),
            TermValue::Blank { label, scope } => EventTerm::Blank {
                label,
                scope: ScopeId(scope.0),
            },
            TermValue::Literal {
                lexical_form,
                datatype,
                language,
                direction,
            } => EventTerm::Literal {
                lexical: lexical_form,
                datatype,
                language: language.as_deref(),
                direction: *direction,
            },
        };
        let _ = self
            .sink_mut()?
            .term(id, event)
            .map_err(|e| e.to_string())?;
        self.ids.insert(term.clone(), id);
        Ok(id)
    }

    fn sink_mut(&mut self) -> Result<&mut DatasetSink, String> {
        self.inner
            .as_mut()
            .ok_or_else(|| "the sink has already been finished".to_owned())
    }

    fn push_inner(&mut self, quad: &Quad) -> Result<(), String> {
        let values = quad_to_quad_values(quad)?;
        let s = self.emit_term(&values.s)?;
        let p = self.emit_term(&values.p)?;
        let o = self.emit_term(&values.o)?;
        let g = values.g.as_ref().map(|g| self.emit_term(g)).transpose()?;
        let _ = self
            .sink_mut()?
            .quad(EventQuad { s, p, o, g })
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[wasm_bindgen]
impl Sink {
    /// `new Sink()` — an empty sink ready to accept quads via `push`.
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
        Dataset::from_frozen(dataset.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::term::Term;
    use crate::term::TermInner;
    use purrdf::RdfTerm;

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
        assert_eq!(sink.next_id, Some(4));
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
        let quoted = Term::from_rdf_term(&RdfTerm::Triple(Box::new(inner)));
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
    fn deep_triples_stream_with_one_fixed_size_key_per_level() {
        std::thread::Builder::new()
            .stack_size(128 * 1024)
            .spawn(|| {
                let depth = 20_000;
                let mut value = TermValue::iri("https://e/end");
                for _ in 0..depth {
                    value = TermValue::Triple {
                        s: purrdf::TermBox::new(TermValue::iri("https://e/s")),
                        p: purrdf::TermBox::new(TermValue::iri("https://e/p")),
                        o: purrdf::TermBox::new(value),
                    };
                }
                let object =
                    Term::from_value(value, crate::convert::BlankScopeMode::Keep).expect("valid");
                let quad = Quad::from_parts(
                    named("https://e/outer"),
                    named("https://e/p"),
                    object,
                    Term::from_inner(TermInner::DefaultGraph),
                );
                let mut sink = Sink::new();
                sink.push_inner(&quad).expect("deep terms stream");
                assert_eq!(sink.ids.len(), 4, "only fixed-size leaves are owned keys");
                assert_eq!(sink.triple_ids.len(), depth);
                let ids = sink.next_id;
                sink.push_inner(&quad).expect("repeated deep term streams");
                assert_eq!(sink.next_id, ids, "all component identities reused");
                let mut raw = sink.inner.take().expect("not finished");
                let error = raw.finish().expect_err("deep terms are refused at freeze");
                assert!(matches!(error, purrdf_events::EventError::Message(_)));
                assert!(
                    error.to_string().contains("rdf-ir-triple-nesting-limit"),
                    "{error}"
                );
            })
            .expect("thread")
            .join()
            .expect("no stack overflow");
    }

    #[test]
    fn scoped_blanks_stream_with_their_original_identity() {
        let values = purrdf::ir::QuadValues {
            s: TermValue::Blank {
                label: "b".to_owned(),
                scope: purrdf::BlankScope(3),
            },
            p: TermValue::iri("https://e/p"),
            o: TermValue::Blank {
                label: "b".to_owned(),
                scope: purrdf::BlankScope(1),
            },
            g: None,
        };
        let quad = crate::convert::quad_values_to_quad(&values).expect("scoped values");
        let mut sink = Sink::new();
        sink.push_inner(&quad).expect("scope declarations");
        let dataset = sink.finish().expect("finishes");
        let found = dataset.quads().expect("egress");
        assert_eq!(quad_to_quad_values(&found[0]).expect("lowers"), values);
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

#[cfg(test)]
mod term_walk_tests {
    //! The streaming sink's term declaration against its recursive reference.

    use purrdf::{RdfLiteral, RdfTerm, RdfTriple, TermValue};
    use purrdf_events::{EventTerm, EventTermId, EventTriple, RdfEventSink as _};

    use super::Sink;

    fn reference(sink: &mut Sink, term: &RdfTerm) -> Result<EventTermId, String> {
        if let Some(id) = sink.ids.get(&TermValue::from_rdf_term(term)) {
            return Ok(*id);
        }
        let RdfTerm::Triple(triple) = term else {
            return sink.emit_leaf(&TermValue::from_rdf_term(term));
        };
        let s = reference(sink, &triple.subject)?;
        let p = reference(sink, &RdfTerm::Iri(triple.predicate.clone()))?;
        let o = reference(sink, &triple.object)?;
        if let Some(id) = sink.triple_ids.get(&(s, p, o)) {
            return Ok(*id);
        }
        let id = sink.mint()?;
        let _ = sink
            .sink_mut()?
            .term(id, EventTerm::Triple(EventTriple { s, p, o }))
            .map_err(|e| e.to_string())?;
        sink.triple_ids.insert((s, p, o), id);
        Ok(id)
    }

    fn owned(value: &TermValue) -> RdfTerm {
        match value {
            TermValue::Triple { s, p, o } => {
                let TermValue::Iri(predicate) = &**p else {
                    unreachable!("the generator was asked for IRI predicates")
                };
                RdfTerm::triple(RdfTriple::new(owned(s), predicate.clone(), owned(o)))
            }
            TermValue::Iri(iri) => RdfTerm::iri(iri.clone()),
            TermValue::Blank { label, .. } => RdfTerm::blank_node(label.clone()),
            TermValue::Literal {
                lexical_form,
                datatype,
                language,
                direction,
            } => RdfTerm::literal(RdfLiteral {
                lexical_form: lexical_form.clone(),
                datatype: Some(datatype.clone()),
                language: language.clone(),
                direction: *direction,
            }),
        }
    }

    /// Declaring a run of generated terms — repeats and shared components included —
    /// mints the same protocol ids, in the same order, as the recursive reference.
    #[test]
    fn declaration_agrees_with_its_recursive_reference_on_generated_terms() {
        for seed in 0..200_u64 {
            let (mut found, mut expected) = (Sink::default(), Sink::default());
            for offset in [0, 1, 0, 2] {
                let mut state = seed * 7 + offset;
                let mut budget = 6;
                let term = owned(&purrdf_core::term_fixture::term_value(
                    &mut state,
                    purrdf_testkit::rng::splitmix64_next,
                    &mut budget,
                    purrdf_core::term_fixture::TermShape::IriPredicates,
                ));
                assert_eq!(
                    found.emit_term(&TermValue::from_rdf_term(&term)),
                    reference(&mut expected, &term),
                    "seed {seed}"
                );
            }
            assert_eq!(found.next_id, expected.next_id, "seed {seed}");
            assert_eq!(found.ids, expected.ids, "seed {seed}");
            assert_eq!(found.triple_ids, expected.triple_ids, "seed {seed}");
        }
    }

    #[test]
    fn event_ids_cross_u32_and_refuse_terminal_exhaustion_without_reuse() {
        let mut sink = Sink {
            next_id: Some(u64::from(u32::MAX) + 1),
            ..Sink::default()
        };
        assert_eq!(sink.mint().unwrap(), EventTermId(u64::from(u32::MAX) + 1));
        sink.next_id = Some(u64::MAX);
        assert_eq!(sink.mint().unwrap(), EventTermId(u64::MAX));
        for _ in 0..2 {
            assert_eq!(
                sink.mint().unwrap_err(),
                purrdf_events::EventError::IdSpaceExhausted.to_string()
            );
        }
    }
}

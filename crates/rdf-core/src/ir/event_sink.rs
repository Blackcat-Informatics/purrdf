// SPDX-FileCopyrightText: 2026 Blackcat Informatics Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Evented, ID-addressed OUTPUT of a frozen [`RdfDataset`] (#819 C6).
//!
//! [`RdfDatasetVisitor`] is the **frozen-dataset OUTPUT visitor**: it is the dual of
//! the permissive ingestion protocol (the `purrdf-events` `RdfEventSink`, purrdf
//! P6 #840) — where that ingestion sink folds an
//! *external* event stream *into* the
//! IR, [`RdfDataset::emit`] walks an *already-frozen* dataset and streams it *out* as
//! events, so downstream consumers — the chase materializer, SHACL result emission,
//! and projection writers — can receive the graph without each re-walking or
//! re-materializing it.
//!
//! The two are distinct on purpose: this visitor is **infallible** (every method
//! returns `()`), output-only, and driven by [`RdfDataset::emit`] over a validated
//! dataset; the ingestion `RdfEventSink` is **fallible** (forward references,
//! cancellation, unresolved-at-finish are all real outcomes) and driven by an
//! arbitrary, possibly out-of-order external source.
//!
//! The stream is **ID-addressed and self-declaring**: every [`TermId`] is declared
//! by a [`term`](RdfDatasetVisitor::term) event *before* any quad / reifier / annotation
//! / nested-triple-term references it. Because interning is bottom-up (a triple
//! term's components are interned before the triple term itself, so they hold lower
//! ids), emitting term declarations in ascending id order satisfies this invariant
//! by construction — a sink may resolve any referenced id against state it has
//! already seen.

use super::dataset::{QuadIds, RdfDataset, TermRef};
use super::term::TermId;

/// A receiver for the evented, ID-addressed output of a frozen [`RdfDataset`].
///
/// All methods default to no-ops (ISP): a sink implements only the events it cares
/// about — e.g. a projection writer that only needs quads ignores reifiers and
/// annotations. The driver is [`RdfDataset::emit`], which guarantees the
/// term-before-reference ordering documented on this module.
pub trait RdfDatasetVisitor {
    /// Declare a term and its resolved value. Emitted in ascending [`TermId`] order,
    /// so any component id of a triple term — and a literal's datatype id — has
    /// already been declared.
    fn term(&mut self, id: TermId, term: TermRef<'_>) {
        let _ = (id, term);
    }

    /// A quad row (ID-native). All four positions were declared by prior
    /// [`term`](Self::term) events.
    fn quad(&mut self, quad: QuadIds) {
        let _ = quad;
    }

    /// A `(reifier, triple-term)` binding (C0.4). Both ids were declared earlier.
    fn reifier(&mut self, reifier: TermId, triple: TermId) {
        let _ = (reifier, triple);
    }

    /// A `(reifier, predicate, object)` statement annotation. All three ids were
    /// declared earlier.
    fn annotation(&mut self, reifier: TermId, predicate: TermId, object: TermId) {
        let _ = (reifier, predicate, object);
    }
}

impl RdfDataset {
    /// Stream this frozen dataset to an [`RdfDatasetVisitor`] as an ID-addressed,
    /// self-declaring event stream: every term is declared before it is referenced
    /// (see the module docs). Zero allocation beyond what the sink itself does.
    pub fn emit<S: RdfDatasetVisitor + ?Sized>(&self, sink: &mut S) {
        // Term declarations first, in ascending id order — components and datatypes
        // (lower ids) precede the triple terms / literals that reference them.
        for i in 0..self.term_count() {
            let id = TermId::from_index(i as u32);
            sink.term(id, self.resolve(id));
        }
        for quad in self.quads() {
            sink.quad(quad);
        }
        for (reifier, triple) in self.reifiers() {
            sink.reifier(reifier, triple);
        }
        for (reifier, predicate, object) in self.annotations() {
            sink.annotation(reifier, predicate, object);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::RdfDatasetBuilder;
    use crate::RdfLiteral;
    use std::collections::HashSet;

    fn iri(b: &mut RdfDatasetBuilder, n: &str) -> TermId {
        b.intern_iri(&format!("http://example.org/{n}"))
    }

    /// A sink that records the stream and checks the term-before-reference
    /// invariant: any id referenced by a term/quad/reifier/annotation must have been
    /// declared by an earlier `term` event.
    #[derive(Default)]
    struct CollectSink {
        declared: HashSet<TermId>,
        term_count: usize,
        quads: Vec<QuadIds>,
        reifiers: Vec<(TermId, TermId)>,
        annotations: Vec<(TermId, TermId, TermId)>,
        violations: usize,
    }

    impl CollectSink {
        fn require(&mut self, id: TermId) {
            if !self.declared.contains(&id) {
                self.violations += 1;
            }
        }
    }

    impl RdfDatasetVisitor for CollectSink {
        fn term(&mut self, id: TermId, term: TermRef<'_>) {
            match term {
                TermRef::Triple { s, p, o } => {
                    self.require(s);
                    self.require(p);
                    self.require(o);
                }
                TermRef::Literal { datatype, .. } => self.require(datatype),
                _ => {}
            }
            self.declared.insert(id);
            self.term_count += 1;
        }
        fn quad(&mut self, quad: QuadIds) {
            self.require(quad.s);
            self.require(quad.p);
            self.require(quad.o);
            if let Some(g) = quad.g {
                self.require(g);
            }
            self.quads.push(quad);
        }
        fn reifier(&mut self, reifier: TermId, triple: TermId) {
            self.require(reifier);
            self.require(triple);
            self.reifiers.push((reifier, triple));
        }
        fn annotation(&mut self, reifier: TermId, predicate: TermId, object: TermId) {
            self.require(reifier);
            self.require(predicate);
            self.require(object);
            self.annotations.push((reifier, predicate, object));
        }
    }

    #[test]
    fn emit_is_complete_and_self_declaring() {
        let mut b = RdfDatasetBuilder::new();
        let (s, p, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
        // A literal (references a datatype term) and a triple term (references s,p,o).
        let lit = b.intern_literal(RdfLiteral::language_tagged("hi", "en"));
        let triple = b.intern_triple(s, p, o);
        let r = iri(&mut b, "r");
        let ap = iri(&mut b, "ap");
        b.push_quad(s, p, o, None);
        b.push_quad(s, p, lit, None);
        b.push_reifier(r, triple);
        b.push_annotation(r, ap, o);
        let ds = b.freeze().expect("valid");

        let mut sink = CollectSink::default();
        ds.emit(&mut sink);

        // Self-declaring: nothing referenced before it was declared.
        assert_eq!(sink.violations, 0, "term-before-reference invariant held");
        // Complete: every table is reproduced exactly.
        assert_eq!(sink.term_count, ds.term_count());
        assert_eq!(sink.quads, ds.quads().collect::<Vec<_>>());
        assert_eq!(sink.reifiers, ds.reifiers().collect::<Vec<_>>());
        assert_eq!(sink.annotations, ds.annotations().collect::<Vec<_>>());
    }

    #[test]
    fn default_sink_methods_are_noops() {
        // A sink that overrides nothing still drives to completion without panicking.
        struct Silent;
        impl RdfDatasetVisitor for Silent {}
        let mut b = RdfDatasetBuilder::new();
        let (s, p, o) = (iri(&mut b, "s"), iri(&mut b, "p"), iri(&mut b, "o"));
        b.push_quad(s, p, o, None);
        let ds = b.freeze().expect("valid");
        ds.emit(&mut Silent);
    }
}

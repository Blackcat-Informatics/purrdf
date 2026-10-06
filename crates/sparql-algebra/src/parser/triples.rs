// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Triples, property paths and terms, read by loops over explicit stacks.
//!
//! A triples block's nested constructs — blank-node property lists, collections,
//! triple terms, reifying triples and annotation blocks — reach one another but
//! nothing outside them: no expression and no group graph pattern is written inside a
//! triple. So they are one small pushdown machine of their own, [`Parser::run_triples`],
//! over [`TFrame`]s, called at a fixed depth from a group, a template or a quad block.
//! A property path, a quoted triple in a term position and a `VALUES` triple term are
//! each a loop over a stack of their own open brackets. None of them calls itself, so
//! how deeply a triple nests is bounded by memory alone.

use crate::algebra::{
    GraphPattern, NegatedPathElement, PropertyFunctionCall, PropertyPathExpression,
};
use crate::ast::{
    GroundTerm, GroundTriple, NamedNode, NamedNodePattern, TermPattern, TriplePattern,
};
use crate::error::{ParseError, Result};
use crate::lexer::Token;
use crate::tree::Child;
use crate::worklist::WorkList;

use super::{
    BlockSink, Parser, RDF_FIRST, RDF_NIL, RDF_REST, RDF_TYPE, SubjectArgs, TripleContext, Verb,
    simple_predicate,
};

/// One predicate edge, oriented relative to its endpoints.
#[derive(Clone, Copy)]
struct PathEdge<'a> {
    predicate: &'a NamedNode,
    inverse: bool,
}

impl PathEdge<'_> {
    /// The data triple connecting this edge's endpoints.
    fn triple(self, subject: TermPattern, object: TermPattern) -> TriplePattern {
        let (subject, object) = if self.inverse {
            (object, subject)
        } else {
            (subject, object)
        };
        TriplePattern {
            subject,
            predicate: NamedNodePattern::NamedNode(self.predicate.clone()),
            object,
        }
    }
}

/// The complete path consists of predicate edges, with or without alternatives.
enum PredicatePath<'a> {
    Linear(Vec<PathEdge<'a>>),
    Branching,
}

/// Certify the whole path before touching parser counters or blocks. Keep the
/// existing ordered-edge fast path for a linear word. Inversion is an involution
/// and reverses composition: `^(p/q)` walks `^q` then `^p`.
fn predicate_path(path: &PropertyPathExpression) -> Option<PredicatePath<'_>> {
    let mut pending = WorkList::<_, 16>::with((path, false));
    let mut edges = Vec::new();
    let mut branching = false;
    while let Some((path, inverse)) = pending.pop() {
        match path {
            PropertyPathExpression::NamedNode(predicate) => {
                if !branching {
                    edges.push(PathEdge { predicate, inverse });
                }
            }
            PropertyPathExpression::Reverse(inner) => pending.push((inner, !inverse)),
            PropertyPathExpression::Sequence(elements) => {
                let queued = pending.len();
                pending.extend(elements.iter().map(|element| (element, inverse)));
                if !inverse {
                    pending.reverse_top(pending.len() - queued);
                }
            }
            PropertyPathExpression::Alternative(elements) => {
                branching = true;
                edges.clear();
                pending.extend(elements.iter().map(|element| (element, inverse)));
            }
            PropertyPathExpression::ZeroOrMore(_)
            | PropertyPathExpression::OneOrMore(_)
            | PropertyPathExpression::ZeroOrOne(_)
            | PropertyPathExpression::NegatedPropertySet(_)
            | PropertyPathExpression::Range { .. }
            | PropertyPathExpression::Wildcard { .. } => return None,
        }
    }
    Some(if branching {
        PredicatePath::Branching
    } else {
        PredicatePath::Linear(edges)
    })
}

/// One step of the certified predicate path's relational translation.
enum PathTranslation<'a> {
    Path(&'a PropertyPathExpression, bool, TermPattern, TermPattern),
    Sequence(usize),
    Alternative(usize),
}

/// A predicate-object list in progress: its subject and the verb whose objects are
/// being read.
pub(super) struct PolState {
    subject: SubjectArgs,
    verb: Verb,
}

/// A triples construct waiting for the node, or the predicate-object list, it handed
/// over to.
pub(super) enum TFrame {
    /// A predicate-object list, waiting for the object of its current verb.
    Object(PolState),
    /// A predicate-object list, waiting for the list of an annotation block `{| … |}`
    /// that annotates `TriplePattern`.
    Annotation(PolState, TriplePattern),
    /// A blank-node property list `[ … ]`, waiting for its list; the node is its blank.
    PropertyList(TermPattern),
    /// A collection `( … )`, waiting for an element: its head, and the cell the
    /// element belongs to.
    Collection {
        head: TermPattern,
        cell: TermPattern,
    },
    /// A triple term `<<( … )>>` or reifying triple `<< … >>`, waiting for its subject
    /// (`None`) or its object (the subject and predicate read).
    TripleNode {
        is_triple_term: bool,
        subject: Option<(TermPattern, NamedNodePattern)>,
    },
}

/// What the triples machine reads next.
enum TGoal {
    /// A `GraphNode`: a term, a blank-node property list, a collection or a triple node.
    Node,
    /// A subject or object of a triple node: a term or a nested triple node.
    Component,
    /// A predicate-object list of this subject.
    PredicateObjectList(SubjectArgs),
}

/// What the triples machine does next.
enum TStep {
    Start(TGoal),
    /// A construct finished: its node, or `None` for a predicate-object list.
    Done(Option<TermPattern>),
}

/// One open level of a property path: the alternative and the sequence read so far
/// in it, and whether the element being read is inverted.
#[derive(Default)]
pub(super) struct PathLevel {
    alternative: Option<PropertyPathExpression>,
    sequence: Option<PropertyPathExpression>,
    inverse: bool,
}

impl Parser<'_, '_> {
    // ── the triples machine ─────────────────────────────────────────────────

    /// Read one `GraphNode` — a nested blank-node property list, a nested collection, a
    /// triple node, or a plain term — emitting the triples it holds into `sink`.
    pub(super) fn parse_graph_node(&mut self, sink: &mut BlockSink) -> Result<TermPattern> {
        let node = self.run_triples(TGoal::Node, sink)?;
        Ok(node.expect("a node goal yields a node"))
    }

    /// Read a predicate-object list of `subject`, emitting its triples into `sink`.
    pub(super) fn parse_predicate_object_list(
        &mut self,
        subject: SubjectArgs,
        sink: &mut BlockSink,
    ) -> Result<()> {
        self.run_triples(TGoal::PredicateObjectList(subject), sink)
            .map(drop)
    }

    /// Drive the triples machine from `goal` until it finishes, over the reusable frame
    /// stack.
    fn run_triples(&mut self, goal: TGoal, sink: &mut BlockSink) -> Result<Option<TermPattern>> {
        let mut frames = std::mem::take(&mut self.triple_frames);
        let result = self.drive_triples(goal, sink, &mut frames);
        frames.clear();
        self.triple_frames = frames;
        result
    }

    fn drive_triples(
        &mut self,
        goal: TGoal,
        sink: &mut BlockSink,
        frames: &mut Vec<TFrame>,
    ) -> Result<Option<TermPattern>> {
        let mut step = TStep::Start(goal);
        loop {
            step = match step {
                TStep::Start(TGoal::Node) => self.start_node(frames)?,
                TStep::Start(TGoal::Component) => self.start_component(frames)?,
                TStep::Start(TGoal::PredicateObjectList(subject)) => {
                    let verb = self.parse_verb()?;
                    self.objects(PolState { subject, verb }, frames, sink)?
                }
                TStep::Done(value) => match frames.pop() {
                    None => return Ok(value),
                    Some(frame) => self.resume_triples(frame, value, frames, sink)?,
                },
            };
        }
    }

    /// A `GraphNode` at the cursor.
    ///
    /// A blank-node property list `[ predicate object … ]` (RDF 1.1 §4.2, SPARQL §19.6
    /// `BlankNodePropertyListPath ::= '[' PropertyListPathNotEmpty ']'`) mints a fresh
    /// blank node, emits the embedded triples, and is that node. The production is
    /// `…NotEmpty`, so an empty pair is refused — see [`Self::empty_bracket_pair`] for
    /// why only the parser can refuse it; the anonymous `[]` is `ANON`, a single
    /// [`Token::Anon`] read as a term.
    ///
    /// A collection `( n1 n2 … )` (RDF 1.1 §4.3, SPARQL §19.5) desugars to the standard
    /// `rdf:first`/`rdf:rest` blank-node chain terminated by `rdf:nil`, and is its HEAD
    /// node. The SPARQL grammar requires at least one node inside the parentheses, but
    /// RDF's empty collection `()` is `rdf:nil`; it is accepted for robustness.
    fn start_node(&mut self, frames: &mut Vec<TFrame>) -> Result<TStep> {
        if self.at(&Token::LBracket) {
            self.expect(&Token::LBracket)?;
            if self.at(&Token::RBracket) {
                return Err(self.empty_bracket_pair());
            }
            let node = TermPattern::BlankNode(self.fresh_anon());
            frames.push(TFrame::PropertyList(node.clone()));
            return Ok(TStep::Start(TGoal::PredicateObjectList(SubjectArgs::Term(
                node,
            ))));
        }
        if self.at(&Token::LParen) {
            self.expect(&Token::LParen)?;
            if self.eat(&Token::RParen) {
                return Ok(TStep::Done(Some(TermPattern::NamedNode(
                    NamedNode::new_unchecked(RDF_NIL),
                ))));
            }
            let head = TermPattern::BlankNode(self.fresh_anon());
            frames.push(TFrame::Collection {
                cell: head.clone(),
                head,
            });
            return Ok(TStep::Start(TGoal::Node));
        }
        if self.at(&Token::TripleOpen) {
            return self.open_triple_node(frames);
        }
        Ok(TStep::Done(Some(self.parse_term_pattern()?)))
    }

    /// An RDF 1.2 triple node at its `<<`:
    ///
    /// * `<<( s p o )>>` — a **triple term** (a value), yielded directly; or
    /// * `<< s p o [~ reifier] >>` — a **reifying triple**, desugared to a reifier `R`
    ///   with `R rdf:reifies <<( s p o )>>` (R fresh unless given), and `R` is the term.
    fn open_triple_node(&mut self, frames: &mut Vec<TFrame>) -> Result<TStep> {
        self.expect(&Token::TripleOpen)?;
        let is_triple_term = self.eat(&Token::LParen);
        frames.push(TFrame::TripleNode {
            is_triple_term,
            subject: None,
        });
        Ok(TStep::Start(TGoal::Component))
    }

    /// One subject/object component of a triple node, in **graph-pattern** position.
    ///
    /// One reader serves the subject and the object of both spellings, because in a
    /// pattern the two positions carry the SAME production: a nested triple node is
    /// admissible in either. That is not laxity, it is the SPARQL 1.2 grammar —
    /// `TripleTermSubject` includes `TripleTerm`, exactly as SPARQL 1.1's `VarOrTerm`
    /// includes a literal in subject position. The W3C SPARQL 1.2 suite pins it as two
    /// **positive** syntax tests (`syntax-triple-terms-positive/nested-tripleterm-02.rq`
    /// and `syntax-triple-terms-positive/compound-tripleterm-subject.rq`), so refusing it
    /// here would be a non-conformance, not a tightening. A pattern is a *matcher*: one
    /// naming a term the RDF 1.2 term model cannot hold simply matches nothing.
    ///
    /// The term model is therefore enforced where a triple term becomes a **value**
    /// rather than a matcher, and those positions ARE separate readers:
    ///
    /// * ground data (`VALUES`, `BIND` of a constant) —
    ///   [`parse_ground_term`](Self::parse_ground_term) refuses a literal or a nested
    ///   triple term in the subject, which is what the suite's
    ///   `tripleterm-subject-01`..`-06` **negative** syntax tests require;
    /// * expression position (`ExprTripleTerm`, §17.4) refuses the same;
    /// * a `CONSTRUCT` / `UPDATE` template instantiated per solution row, where a
    ///   variable can bind a triple term no syntax mentions, so no parser could decide
    ///   it: `purrdf-sparql-eval`'s `template::positionally_ill_formed` skips the
    ///   instantiation, which is what SPARQL §16.2 mandates for an ill-formed
    ///   instantiation.
    ///
    /// Neither spelling admits an RDF collection or a populated blank-node property list
    /// in any position (both would emit auxiliary triples a single triple cannot carry);
    /// only the anonymous `[]` (a fresh blank node) is.
    fn start_component(&mut self, frames: &mut Vec<TFrame>) -> Result<TStep> {
        match self.peek() {
            Some(Token::TripleOpen) => self.open_triple_node(frames),
            Some(Token::LParen) => Err(ParseError::syntax(
                "an RDF collection is not allowed inside a triple term or reifying triple",
                self.span(),
            )),
            Some(Token::LBracket) => {
                self.expect(&Token::LBracket)?;
                if self.at(&Token::RBracket) {
                    return Err(self.empty_bracket_pair());
                }
                Err(ParseError::syntax(
                    "a populated blank-node property list is not allowed inside a \
                     triple term or reifying triple",
                    self.span(),
                ))
            }
            _ => Ok(TStep::Done(Some(self.parse_term_pattern()?))),
        }
    }

    /// A predicate-object list's verb: a variable, or a property path. A bare variable
    /// predicate is a simple triple predicate, not a property path — and never a
    /// property function, which is only ever a plain IRI.
    fn parse_verb(&mut self) -> Result<Verb> {
        if let Some(Token::Variable(_)) = self.peek() {
            return Ok(Verb::Simple(NamedNodePattern::Variable(self.expect_var()?)));
        }
        let path = self.parse_path()?;
        Ok(match simple_predicate(&path) {
            // A length-1 path is a plain predicate IRI, so it is the one shape that can
            // name a property function; a complex path (`p+`, `p1/p2`, `!(…)`, …) never
            // is.
            Some(NamedNodePattern::NamedNode(n)) if self.options.is_property_fn(n.as_str()) => {
                Verb::PropertyFn(n.as_str().to_owned())
            }
            Some(pred) => Verb::Simple(pred),
            None => Verb::Path(path),
        })
    }

    /// Read the objects of `state`'s verb: a property function's argument vectors are
    /// read here, one after another; any other verb's object is a `GraphNode`, handed
    /// over to.
    fn objects(
        &mut self,
        mut state: PolState,
        frames: &mut Vec<TFrame>,
        sink: &mut BlockSink,
    ) -> Result<TStep> {
        loop {
            let Verb::PropertyFn(iri) = &state.verb else {
                // The subject must be a term before the object is read.
                state.subject.as_term(self.span())?;
                frames.push(TFrame::Object(state));
                return Ok(TStep::Start(TGoal::Node));
            };
            // Both sides are argument VECTORS, captured structurally: an object
            // collection is the call's argument list, not a cons-cell chain, so the
            // `GraphNode` reader is bypassed here.
            let object_args = self.parse_prop_fn_args()?;
            let subject_args = state.subject.as_args();
            sink.push_property_function(PropertyFunctionCall {
                iri: iri.clone(),
                subject_args,
                object_args,
            });
            if self.at(&Token::Tilde) || self.at(&Token::AnnotationOpen) {
                return Err(ParseError::syntax(
                    "RDF 1.2 annotation syntax cannot annotate a property-function call \
                     (no triple is asserted)",
                    self.span(),
                ));
            }
            match self.next_object(state)? {
                Some(next) => state = next,
                None => return Ok(TStep::Done(None)),
            }
        }
    }

    /// After an object: the next object (`,`), the next verb (`;`), or the list's end
    /// (`None`).
    fn next_object(&mut self, state: PolState) -> Result<Option<PolState>> {
        if self.eat(&Token::Comma) {
            return Ok(Some(state));
        }
        if !self.eat(&Token::Semicolon) {
            return Ok(None);
        }
        // `PropertyListNotEmpty ::= Verb ObjectList ( ';' ( Verb ObjectList )? )*`
        // (and its path twin, [83]): the verb after a `;` is optional, so `;` may
        // repeat (`?s :p 1 ; ; :q 2`).
        while self.eat(&Token::Semicolon) {}
        // A trailing `;` before `.`/`}`/`]` (the last closes a blank-node property
        // list) ends the list.
        if self.at(&Token::Dot)
            || self.at(&Token::RBrace)
            || self.at(&Token::RBracket)
            || self.block_boundary()
        {
            return Ok(None);
        }
        let verb = self.parse_verb()?;
        Ok(Some(PolState {
            subject: state.subject,
            verb,
        }))
    }

    /// Continue a predicate-object list after an object.
    fn continue_objects(
        &mut self,
        state: PolState,
        frames: &mut Vec<TFrame>,
        sink: &mut BlockSink,
    ) -> Result<TStep> {
        match self.next_object(state)? {
            Some(next) => self.objects(next, frames, sink),
            None => Ok(TStep::Done(None)),
        }
    }

    /// RDF 1.2 annotation syntax trailing an asserted triple `base`: zero or more
    /// reifiers `~ [id]` and annotation blocks `{| predObjList |}`. Each emits a fresh
    /// (or given) reifier `R` with `R rdf:reifies <<( s p o )>>`; an annotation block
    /// additionally applies its predicate-object list to `R`.
    ///
    /// An annotation block binds to the reifier of the immediately preceding `~ id` if
    /// one is `pending` (so `~ :r {| … |}` annotates `:r` rather than a fresh node —
    /// important for DELETE templates, which forbid blank nodes); otherwise it mints a
    /// fresh blank reifier.
    fn annotations(
        &mut self,
        state: PolState,
        base: TriplePattern,
        mut pending: Option<TermPattern>,
        frames: &mut Vec<TFrame>,
        sink: &mut BlockSink,
    ) -> Result<TStep> {
        loop {
            if self.eat(&Token::Tilde) {
                let reifier = self.parse_reifier_id()?;
                self.emit_reifies(&reifier, &base, &mut sink.triples);
                pending = Some(reifier);
            } else if self.eat(&Token::AnnotationOpen) {
                let reifier = if let Some(r) = pending.take() {
                    r
                } else {
                    let r = TermPattern::BlankNode(self.fresh_anon());
                    self.emit_reifies(&r, &base, &mut sink.triples);
                    r
                };
                frames.push(TFrame::Annotation(state, base));
                return Ok(TStep::Start(TGoal::PredicateObjectList(SubjectArgs::Term(
                    reifier,
                ))));
            } else {
                break;
            }
        }
        self.continue_objects(state, frames, sink)
    }

    /// Hand `frame` the node (or the finished list) it was waiting for.
    fn resume_triples(
        &mut self,
        frame: TFrame,
        value: Option<TermPattern>,
        frames: &mut Vec<TFrame>,
        sink: &mut BlockSink,
    ) -> Result<TStep> {
        match frame {
            TFrame::Object(state) => {
                let object = value.expect("an object is a node");
                let subject = state.subject.as_term(self.span())?.clone();
                match &state.verb {
                    Verb::Simple(pred) => {
                        let pred = pred.clone();
                        sink.triples.push(TriplePattern {
                            subject: subject.clone(),
                            predicate: pred.clone(),
                            object: object.clone(),
                        });
                        // RDF 1.2 annotation syntax (`~ reifier`, `{| … |}`) may trail the
                        // object, reifying the triple just asserted.
                        let base = TriplePattern {
                            subject,
                            predicate: pred,
                            object,
                        };
                        self.annotations(state, base, None, frames, sink)
                    }
                    Verb::Path(path) => {
                        let translation = (sink.context == TripleContext::Pattern)
                            .then(|| predicate_path(path))
                            .flatten();
                        match translation {
                            Some(PredicatePath::Linear(edges)) => {
                                self.push_linear_path(subject, &edges, object, &mut sink.triples);
                            }
                            Some(PredicatePath::Branching) => {
                                sink.paths
                                    .push(self.translate_predicate_path(path, subject, object));
                            }
                            None => sink.paths.push(GraphPattern::Path {
                                subject,
                                path: path.clone(),
                                object,
                            }),
                        }
                        self.continue_objects(state, frames, sink)
                    }
                    Verb::PropertyFn(_) => {
                        unreachable!("a property function's objects are argument vectors")
                    }
                }
            }
            TFrame::Annotation(state, base) => {
                self.expect(&Token::AnnotationClose)?;
                self.annotations(state, base, None, frames, sink)
            }
            TFrame::PropertyList(node) => {
                self.expect(&Token::RBracket)?;
                Ok(TStep::Done(Some(node)))
            }
            TFrame::Collection { head, cell } => {
                let element = value.expect("a collection element is a node");
                sink.triples.push(TriplePattern {
                    subject: cell.clone(),
                    predicate: NamedNodePattern::NamedNode(NamedNode::new_unchecked(RDF_FIRST)),
                    object: element,
                });
                let rest = NamedNodePattern::NamedNode(NamedNode::new_unchecked(RDF_REST));
                if self.at(&Token::RParen) {
                    // Last element: terminate the chain with rdf:nil.
                    sink.triples.push(TriplePattern {
                        subject: cell,
                        predicate: rest,
                        object: TermPattern::NamedNode(NamedNode::new_unchecked(RDF_NIL)),
                    });
                    self.expect(&Token::RParen)?;
                    return Ok(TStep::Done(Some(head)));
                }
                // Another element follows: link to a fresh tail cell.
                let next = TermPattern::BlankNode(self.fresh_anon());
                sink.triples.push(TriplePattern {
                    subject: cell,
                    predicate: rest,
                    object: next.clone(),
                });
                frames.push(TFrame::Collection { head, cell: next });
                Ok(TStep::Start(TGoal::Node))
            }
            TFrame::TripleNode {
                is_triple_term,
                subject: None,
            } => {
                let subject = value.expect("a triple node's subject is a node");
                let predicate = self.parse_predicate_name()?;
                frames.push(TFrame::TripleNode {
                    is_triple_term,
                    subject: Some((subject, predicate)),
                });
                Ok(TStep::Start(TGoal::Component))
            }
            TFrame::TripleNode {
                is_triple_term,
                subject: Some((subject, predicate)),
            } => {
                let inner = TriplePattern {
                    subject,
                    predicate,
                    object: value.expect("a triple node's object is a node"),
                };
                if is_triple_term {
                    self.expect(&Token::RParen)?;
                    self.expect(&Token::TripleClose)?;
                    return Ok(TStep::Done(Some(TermPattern::Triple(Child::new(inner)))));
                }
                // Reifying triple: optional `~ reifier`, else a fresh blank reifier.
                let reifier = if self.eat(&Token::Tilde) {
                    self.parse_reifier_id()?
                } else {
                    TermPattern::BlankNode(self.fresh_anon())
                };
                self.expect(&Token::TripleClose)?;
                self.emit_reifies(&reifier, &inner, &mut sink.triples);
                Ok(TStep::Done(Some(reifier)))
            }
        }
    }

    /// Emit a certified linear path into the current BGP (§18.2.2.4), with one
    /// fresh hidden existential per join point. Pattern blanks already implement
    /// those variables and keep them out of `SELECT *`.
    fn push_linear_path(
        &mut self,
        subject: TermPattern,
        edges: &[PathEdge<'_>],
        object: TermPattern,
        triples: &mut Vec<TriplePattern>,
    ) {
        let (last, prefix) = edges.split_last().expect("a linear path has an IRI leaf");
        triples.reserve(edges.len());
        let mut current = subject;
        for edge in prefix {
            let next = TermPattern::BlankNode(self.fresh_anon());
            triples.push(edge.triple(current, next.clone()));
            current = next;
        }
        triples.push(last.triple(current, object));
    }

    /// Translate a certified predicate-only path as compact joins and bag unions
    /// (§18.2.2.4). A sequence introduces one hidden witness at each join point;
    /// alternatives share endpoints, retain repeated arms, and are never expanded
    /// into a Cartesian product of branches. All work and drop walks stay iterative.
    fn translate_predicate_path(
        &mut self,
        path: &PropertyPathExpression,
        subject: TermPattern,
        object: TermPattern,
    ) -> GraphPattern {
        let mut pending =
            WorkList::<_, 16>::with(PathTranslation::Path(path, false, subject, object));
        let mut results = Vec::new();
        while let Some(step) = pending.pop() {
            match step {
                PathTranslation::Path(path, inverse, subject, object) => match path {
                    PropertyPathExpression::NamedNode(predicate) => {
                        results.push(GraphPattern::Bgp {
                            patterns: vec![PathEdge { predicate, inverse }.triple(subject, object)],
                        });
                    }
                    PropertyPathExpression::Reverse(inner) => {
                        pending.push(PathTranslation::Path(inner, !inverse, subject, object));
                    }
                    PropertyPathExpression::Sequence(elements) => {
                        pending.push(PathTranslation::Sequence(elements.len()));
                        let first = pending.len();
                        let mut current = subject;
                        for index in 0..elements.len() {
                            let next = if index + 1 == elements.len() {
                                object.clone()
                            } else {
                                TermPattern::BlankNode(self.fresh_anon())
                            };
                            let element = &elements[if inverse {
                                elements.len() - index - 1
                            } else {
                                index
                            }];
                            pending.push(PathTranslation::Path(
                                element,
                                inverse,
                                current,
                                next.clone(),
                            ));
                            current = next;
                        }
                        pending.reverse_top(pending.len() - first);
                    }
                    PropertyPathExpression::Alternative(elements) => {
                        pending.push(PathTranslation::Alternative(elements.len()));
                        pending.extend(elements.iter().rev().map(|element| {
                            PathTranslation::Path(element, inverse, subject.clone(), object.clone())
                        }));
                    }
                    _ => unreachable!("the whole path was certified before translation"),
                },
                PathTranslation::Sequence(count) | PathTranslation::Alternative(count) => {
                    let parts = results.split_off(results.len() - count);
                    let alternative = matches!(step, PathTranslation::Alternative(_));
                    let result = parts.into_iter().reduce(if alternative {
                        GraphPattern::union
                    } else {
                        super::join
                    });
                    results.push(result.expect("a path chain has at least two elements"));
                }
            }
        }
        results.pop().expect("the path has a predicate leaf")
    }

    // ── property paths (§18.1.7 / §9) ────────────────────────────────────────

    /// Read a property path.
    ///
    /// ```text
    /// Path     ::= Sequence ( '|' Sequence )*
    /// Sequence ::= EltOrInverse ( '/' EltOrInverse )*
    /// EltOrInverse ::= '^'? Primary Modifier?
    /// Primary  ::= iri | 'a' | '!' NegatedPropertySet | '(' Path ')'
    /// ```
    ///
    /// One loop reads every element; a `(` opens a level on a stack and its `)` closes
    /// it, so how deeply a path's groups nest is bounded by memory. A chain (`p1 / p2 /
    /// …`, `p1 | p2 | …`) is ONE n-ary node (`PropertyPathExpression::sequence` /
    /// `alternative`) however long it is written; only what the grammar nests — a
    /// bracketed group, `^`, a postfix modifier — makes the tree taller.
    pub(super) fn parse_path(&mut self) -> Result<PropertyPathExpression> {
        let mut levels = std::mem::take(&mut self.path_levels);
        let result = self.read_path(&mut levels);
        levels.clear();
        self.path_levels = levels;
        result
    }

    fn read_path(&mut self, levels: &mut Vec<PathLevel>) -> Result<PropertyPathExpression> {
        let mut level = PathLevel::default();
        'element: loop {
            level.inverse = self.eat(&Token::Caret);
            let mut value =
                if self.peek_kw("a") && matches!(self.peek(), Some(Token::Word(w)) if *w == "a") {
                    self.pos += 1;
                    PropertyPathExpression::NamedNode(NamedNode::new_unchecked(RDF_TYPE))
                } else {
                    match self.peek() {
                        Some(Token::Iri(_) | Token::PrefixedName(_, _)) => {
                            PropertyPathExpression::NamedNode(self.expect_iri_node()?)
                        }
                        Some(Token::LParen) => {
                            self.pos += 1;
                            levels.push(std::mem::take(&mut level));
                            continue 'element;
                        }
                        Some(Token::Bang) => {
                            self.pos += 1;
                            self.parse_negated_property_set()?
                        }
                        other => {
                            return Err(ParseError::syntax(
                                format!("expected a property path, found {other:?}"),
                                self.span(),
                            ));
                        }
                    }
                };
            loop {
                value = match self.peek() {
                    Some(Token::Star) => {
                        self.pos += 1;
                        PropertyPathExpression::ZeroOrMore(Child::new(value))
                    }
                    Some(Token::Plus) => {
                        self.pos += 1;
                        PropertyPathExpression::OneOrMore(Child::new(value))
                    }
                    Some(Token::Question) => {
                        self.pos += 1;
                        PropertyPathExpression::ZeroOrOne(Child::new(value))
                    }
                    // `{n}` / `{n,}` / `{n,m}` / `{,m}` — bounded repetition (a PurRDF
                    // extension beyond SPARQL 1.1 §9; symmetric parse for the serializer).
                    Some(Token::LBrace) => self.parse_path_range(value)?,
                    _ => value,
                };
                if level.inverse {
                    value = PropertyPathExpression::Reverse(Child::new(value));
                }
                level.sequence = Some(match level.sequence.take() {
                    None => value,
                    Some(left) => PropertyPathExpression::sequence(left, value),
                });
                if self.eat(&Token::Slash) {
                    continue 'element;
                }
                let sequence = level
                    .sequence
                    .take()
                    .expect("a sequence holds the element just read");
                level.alternative = Some(match level.alternative.take() {
                    None => sequence,
                    Some(left) => PropertyPathExpression::alternative(left, sequence),
                });
                if self.eat(&Token::Pipe) {
                    continue 'element;
                }
                let path = level
                    .alternative
                    .take()
                    .expect("an alternative holds the sequence just read");
                let Some(outer) = levels.pop() else {
                    return Ok(path);
                };
                self.expect(&Token::RParen)?;
                level = outer;
                value = path;
            }
        }
    }

    /// Parse a bounded-repetition postfix `{n}` / `{n,}` / `{n,m}` / `{,m}` — a PurRDF
    /// extension beyond SPARQL 1.1 §9. The opening `{` is the current token. Hard-fails
    /// (no silent degradation) on an empty `{}`, a non-integer bound, or a lower bound
    /// exceeding the upper bound.
    fn parse_path_range(
        &mut self,
        primary: PropertyPathExpression,
    ) -> Result<PropertyPathExpression> {
        self.expect(&Token::LBrace)?;
        let lower = self.eat_integer()?;
        let has_comma = self.eat(&Token::Comma);
        let upper = if has_comma { self.eat_integer()? } else { None };
        self.expect(&Token::RBrace)?;

        let (min, max) = if has_comma {
            // `{,}` — both bounds absent — is a silent-degrade to `*`; hard-fail instead.
            if lower.is_none() && upper.is_none() {
                return Err(ParseError::syntax(
                    "empty path range {,} is not allowed (use * for zero-or-more)",
                    self.span(),
                ));
            }
            // `{n,}` / `{n,m}` / `{,m}` (missing lower ⇒ 0).
            (lower.unwrap_or(0), upper)
        } else {
            // `{n}` ⇒ exactly n; an empty `{}` is invalid.
            match lower {
                Some(n) => (n, Some(n)),
                None => {
                    return Err(ParseError::syntax(
                        "empty path range {} is not allowed",
                        self.span(),
                    ));
                }
            }
        };
        if let Some(m) = max
            && min > m
        {
            return Err(ParseError::syntax(
                format!("path range lower bound {min} exceeds upper bound {m}"),
                self.span(),
            ));
        }
        Ok(PropertyPathExpression::Range {
            inner: Child::new(primary),
            min,
            max,
        })
    }

    /// Consume an `Integer` token and parse it to `u32`, returning `Ok(None)` when the
    /// current token is not an integer (so the caller can distinguish a missing bound
    /// from a present one). An out-of-`u32`-range integer is a hard error.
    fn eat_integer(&mut self) -> Result<Option<u32>> {
        let Some(Token::Integer(lex)) = self.peek() else {
            return Ok(None);
        };
        let lex = *lex;
        match lex.parse::<u32>() {
            Ok(n) => {
                self.pos += 1;
                Ok(Some(n))
            }
            Err(_) => Err(ParseError::syntax(
                format!("path range bound {lex:?} is not a valid u32"),
                self.span(),
            )),
        }
    }

    fn parse_negated_property_set(&mut self) -> Result<PropertyPathExpression> {
        let mut nodes = Vec::new();
        if self.eat(&Token::LParen) {
            loop {
                nodes.push(self.parse_path_one_in_set()?);
                if !self.eat(&Token::Pipe) {
                    break;
                }
            }
            self.expect(&Token::RParen)?;
        } else {
            nodes.push(self.parse_path_one_in_set()?);
        }
        Ok(PropertyPathExpression::NegatedPropertySet(nodes))
    }

    fn parse_path_one_in_set(&mut self) -> Result<NegatedPathElement> {
        // `^iri` — an inverse link inside a negated property set (SPARQL 1.1 §18.2
        // `PathOneInPropertySet`) — excludes a *reverse* hop rather than a forward one;
        // see `NegatedPathElement` and the evaluator's decomposition into a
        // forward/reverse `Alternative`.
        let inverse = self.eat(&Token::Caret);
        if matches!(self.peek(), Some(Token::Word(w)) if *w == "a") {
            self.pos += 1;
            return Ok(NegatedPathElement {
                predicate: NamedNode::new_unchecked(RDF_TYPE),
                inverse,
            });
        }
        let predicate = self.expect_iri_node()?;
        Ok(NegatedPathElement { predicate, inverse })
    }

    // ── terms ────────────────────────────────────────────────────────────────

    /// Read an RDF term: a variable, IRI, blank node, literal, or a quoted triple
    /// `<<( s p o )>>` / `<< s p o >>` whose subject and object are terms again. The
    /// quoted triples open are kept on a stack of their own.
    pub(super) fn parse_term_pattern(&mut self) -> Result<TermPattern> {
        // Each open quoted triple: whether it is bracketted `<<( … )>>`, and its subject
        // and predicate once read.
        let mut open: Vec<(bool, Option<(TermPattern, NamedNodePattern)>)> = Vec::new();
        loop {
            let mut term = loop {
                if self.at(&Token::TripleOpen) {
                    self.expect(&Token::TripleOpen)?;
                    let parens = self.eat(&Token::LParen);
                    open.push((parens, None));
                    continue;
                }
                break self.parse_plain_term()?;
            };
            loop {
                let Some((parens, subject)) = open.last_mut() else {
                    return Ok(term);
                };
                let parens = *parens;
                match subject.take() {
                    None => {
                        let predicate = self.parse_predicate_name()?;
                        *subject = Some((term, predicate));
                        break;
                    }
                    Some((subject, predicate)) => {
                        open.pop();
                        if parens {
                            self.expect(&Token::RParen)?;
                        }
                        self.expect(&Token::TripleClose)?;
                        term = TermPattern::Triple(Child::new(TriplePattern {
                            subject,
                            predicate,
                            object: term,
                        }));
                    }
                }
            }
        }
    }

    /// A term that is not a quoted triple.
    fn parse_plain_term(&mut self) -> Result<TermPattern> {
        match self.peek() {
            Some(Token::Variable(_)) => Ok(TermPattern::Variable(self.expect_var()?)),
            Some(Token::Iri(_) | Token::PrefixedName(_, _)) => {
                Ok(TermPattern::NamedNode(self.expect_iri_node()?))
            }
            Some(Token::BlankNodeLabel(_)) => {
                let at = self.span();
                let Some(Token::BlankNodeLabel(l)) = self.bump() else {
                    unreachable!()
                };
                Ok(TermPattern::BlankNode(self.scoped_blank_label(l, at)?))
            }
            Some(Token::Anon) => {
                self.pos += 1;
                Ok(TermPattern::BlankNode(self.fresh_anon()))
            }
            Some(
                Token::StringLit(_)
                | Token::LongStringLit(_)
                | Token::Integer(_)
                | Token::Decimal(_)
                | Token::Double(_)
                | Token::Minus
                | Token::Plus,
            ) => Ok(TermPattern::Literal(self.parse_literal()?)),
            Some(Token::Word(w)) if super::boolean_keyword(w).is_some() => {
                Ok(TermPattern::Literal(self.parse_literal()?))
            }
            other => Err(ParseError::syntax(
                format!("expected an RDF term, found {other:?}"),
                self.span(),
            )),
        }
    }

    /// Read a `VALUES` ground term: an IRI, a literal, or a triple term whose subject is
    /// an IRI (never a literal or a nested triple term — only the *object* may nest).
    /// The triple terms open are kept on a stack of their own.
    pub(super) fn parse_ground_term(&mut self) -> Result<GroundTerm> {
        let mut open: Vec<(bool, Option<(GroundTerm, NamedNode)>)> = Vec::new();
        loop {
            let mut term = loop {
                match self.peek() {
                    Some(Token::Iri(_) | Token::PrefixedName(_, _)) => {
                        break GroundTerm::NamedNode(self.expect_iri_node()?);
                    }
                    Some(Token::TripleOpen) => {
                        self.expect(&Token::TripleOpen)?;
                        let parens = self.eat(&Token::LParen);
                        open.push((parens, None));
                    }
                    // Every other legal ground term — a string, a boolean, or a numeral
                    // (optionally signed: `-1`, `+0.5`) — is `parse_literal`'s grammar;
                    // an illegal token surfaces through its own catch-all error.
                    _ => break GroundTerm::Literal(self.parse_literal()?),
                }
            };
            loop {
                let Some((parens, subject)) = open.last_mut() else {
                    return Ok(term);
                };
                let parens = *parens;
                match subject.take() {
                    None => {
                        if matches!(term, GroundTerm::Triple(_) | GroundTerm::Literal(_)) {
                            return Err(ParseError::syntax(
                                "a literal or nested triple term may not be the subject of a \
                                 triple term",
                                self.span(),
                            ));
                        }
                        // The predicate is an IRI or the `a` keyword (rdf:type).
                        let predicate = if matches!(self.peek(), Some(Token::Word(w)) if *w == "a")
                        {
                            self.pos += 1;
                            NamedNode::new_unchecked(RDF_TYPE)
                        } else {
                            self.expect_iri_node()?
                        };
                        *subject = Some((term, predicate));
                        break;
                    }
                    Some((subject, predicate)) => {
                        open.pop();
                        if parens {
                            self.expect(&Token::RParen)?;
                        }
                        self.expect(&Token::TripleClose)?;
                        term = GroundTerm::Triple(Child::new(GroundTriple {
                            subject,
                            predicate,
                            object: term,
                        }));
                    }
                }
            }
        }
    }
}

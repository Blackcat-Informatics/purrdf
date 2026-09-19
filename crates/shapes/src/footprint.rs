// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The DEPENDENCY FOOTPRINT of a shapes graph: which graph changes can change
//! which focus node's verdict.
//!
//! Incremental validation has exactly one soundness obligation, and it is not the
//! one the validator itself discharges. Re-validating a bounded set of focus nodes
//! is correct only if that set already CONTAINS every node whose verdict the change
//! could move. A caller who expands a change Δ to `{s | (s, p, o) ∈ Δ}` — the
//! obvious expansion, and the wrong one — silently misses every violation reachable
//! through `sh:inversePath`, `sh:targetObjectsOf`, a sequence path, `sh:node`
//! recursion or a property-pair comparand. Nothing fails. The report just says
//! `conforms` about a node nobody re-checked.
//!
//! So the expansion is derived HERE, from the same total walk that lowers the
//! shapes graph ([`crate::plan`]), rather than left to a caller who cannot see the
//! shapes graph's shape.
//!
//! # The model: a trigger is a reversed access
//!
//! Validation reads triples, and every read a shapes graph performs is reached from
//! a focus node by a forward SHACL path — that is what a shapes graph IS. A
//! [`Trigger`] records one such read as the pair *(what the read matches, how the
//! read node was reached)*:
//!
//! * `predicate` — the predicate the read binds, or `None` for a read that binds no
//!   predicate at all (`sh:closed` inspects every outgoing triple of its node).
//! * `endpoint` — which end of the matched triple is the node the read happens AT.
//!   A forward predicate step reads `(node, p, ?)`, so the node is the SUBJECT; an
//!   `sh:inversePath` step reads `(?, p, node)`, so it is the OBJECT; and an RDF 1.2
//!   reifier declaration reads `(?, rdf:reifies, <<( node … )>>)`, where the node is
//!   the SUBJECT OF THE TRIPLE TERM the object carries.
//! * `reversed` — the path from that node BACK to the focus node, or `None` when
//!   the focus node IS it. The walk records the FORWARD chain
//!   ([`PendingTrigger::chain`]); the reversal happens once, below.
//!
//! Expansion then runs the trigger backwards: a changed triple that matches
//! `predicate` offers its `endpoint` as the read node, and [`Trigger::reversed`] is
//! the path from there back to every focus node that could have reached it. That is
//! why the answer is a superset rather than a guess — the reverse of a total forward
//! description is a total backward one.
//!
//! The reversal is [`crate::path::invert`], and it runs ONCE, at stage 0, on the way
//! into [`crate::plan::LoweredPath`] — the same treatment `sh:inversePath` over a
//! composite gets in the lowering itself. Expansion therefore walks a path whose
//! predicate steps are binding-row SLOTS, on the one evaluator the validation hot
//! path uses, and inverts nothing per changed row.
//!
//! Reversal is also why closures cost nothing extra: `sh:zeroOrMorePath ex:next`
//! prefixed onto a chain inverts to `(^ex:next)*`, which is exactly the transitive
//! over-approximation the rule calls for, expressed in the path language the
//! evaluator already has.
//!
//! # The two directions of being wrong
//!
//! Over-approximating is merely slow: a focus node that did not need re-validating
//! is re-validated and conforms. **Under-approximating is a silent drop** — the
//! failure class this workspace treats as first-class — so every construct whose
//! reads this walk cannot bound resolves to [`Footprint::opaque`], and the caller
//! is told the footprint is TOP instead of being handed a subset that looks
//! complete.
//!
//! The mirror error is just as real. A footprint that answered TOP for an ordinary
//! forward path would make incremental validation pointless while looking
//! impeccably careful, so TOP is reserved for a NAMED construct: opaque query text
//! (`sh:sparql`, `sh:select`, a constraint component's `sh:ask`/`sh:select`
//! validator, a `sh:SPARQLFunction` call), and a node expression whose nodes this
//! walk cannot tie back to the focus node by a path. Everything else — every core
//! path form, every core target, every core constraint — stays bounded.
//!
//! # What is deliberately NOT here
//!
//! SHACL-AF **rules** (`sh:rule`) are not walked, for the same reason
//! [`crate::plan`] does not walk them: validation does not run them.
//! `crate::apply_rules` is a separate entailment step that produces a new dataset,
//! and a caller who entails must expand against the entailed graph.

use crate::expression::{FnCall, NodeExpr};
use crate::model::{rdf, rdfs};
use crate::plan::LoweredPath;
use crate::shapes::{Constraint, Path, Target};
use crate::term::NamedNode;

// ── Opacity reasons ─────────────────────────────────────────────────────────────

/// A SHACL-SPARQL constraint or target: the nodes it reads live in query text this
/// walk does not parse.
pub(crate) const OPAQUE_QUERY_TEXT: &str = "a SHACL-SPARQL constraint, target or function reads through query text the \
     shapes walk does not interpret";

/// A custom constraint component: its `sh:ask` / `sh:select` validator is query
/// text, exactly like `sh:sparql`.
pub(crate) const OPAQUE_COMPONENT: &str = "a custom constraint component validates through an sh:ask / sh:select query \
     the shapes walk does not interpret";

/// A node expression that selects nodes from the whole graph rather than from the
/// focus node — `shnex:instancesOf` and `shnex:nodesMatching` — so a change
/// anywhere in the graph can move its result for EVERY focus node.
pub(crate) const OPAQUE_GLOBAL_EXPRESSION: &str =
    "a node expression selects nodes graph-wide rather than from the focus node";

/// A read reached through a node this walk cannot name a path to from the focus
/// node: a `sh:filterShape` over a computed node set, a reifier shape, a custom
/// function body, or a shape resolved out of the shapes-graph index.
pub(crate) const OPAQUE_UNROOTED: &str = "a shape reads from a node the shapes walk cannot reach from the focus node by \
     a path";

// ── The footprint ───────────────────────────────────────────────────────────────

/// Which end of a matched triple stands at the far end of a trigger's chain.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Endpoint {
    /// A forward read, `(node, p, ?)`.
    Subject,
    /// An inverse read, `(?, p, node)`.
    Object,
    /// An RDF 1.2 read THROUGH a triple term in the object position:
    /// `(?, p, <<( node q ? )>>)`. The matched row names the anchored node nowhere
    /// in its own three slots — it names a triple term whose SUBJECT is that node —
    /// so a matched row is unpacked one level before the chain is walked back, and a
    /// row whose object is not a triple term matches nothing.
    ///
    /// This exists for exactly one read, `sh:reifierShape` /
    /// `sh:reificationRequired`: a reifier declaration is the virtual quad
    /// `(reifier, rdf:reifies, <<( focus path value )>>)`, and the node whose verdict
    /// it moves is the triple term's subject.
    ObjectTripleSubject,
}

/// One way a changed triple can reach a focus node. See the module docs.
#[derive(Debug)]
pub(crate) struct Trigger {
    /// The predicate this read binds, or `None` for a read that binds none.
    pub(crate) predicate: Option<NamedNode>,
    /// Which end of the matched triple is the node the read happens at.
    pub(crate) endpoint: Endpoint,
    /// The path from the node the read happens at BACK to every focus node that
    /// could have reached it, lowered; `None` means the focus node IS that node.
    ///
    /// Already reversed. The forward chain the walk records is inverted and lowered
    /// exactly once, on the way out of the walk ([`crate::plan`]'s `ShapeWalk`), so
    /// expansion neither rebuilds a `Path` per changed row nor re-resolves the
    /// predicates of one.
    pub(crate) reversed: Option<LoweredPath>,
}

/// One trigger as the WALK records it: the forward chain, not yet reversed and not
/// yet lowered.
///
/// Separate from [`Trigger`] because the two halves are produced by different
/// parties. This walk observes the shapes graph and can name the forward hops; only
/// the lowering walk hands out the binding-row slots a [`LoweredPath`] is made of,
/// so the reversal happens where the slots live and this type is what travels
/// between them.
#[derive(Clone, Debug)]
pub(crate) struct PendingTrigger {
    /// As [`Trigger::predicate`].
    pub(crate) predicate: Option<NamedNode>,
    /// As [`Trigger::endpoint`].
    pub(crate) endpoint: Endpoint,
    /// The forward path from a focus node to the node the read happens at; `None`
    /// means the focus node is that node.
    pub(crate) chain: Option<Path>,
}

/// The complete dependency footprint of one shapes graph.
///
/// Derived once, at stage 0, from the shapes graph alone — so it is shared by every
/// dataset binding, exactly like the lowering it travels with.
#[derive(Debug, Default)]
pub(crate) struct Footprint {
    triggers: Vec<Trigger>,
    opaque: Option<&'static str>,
}

impl Footprint {
    /// Why this footprint is TOP, or `None` when it is bounded.
    pub(crate) fn opaque(&self) -> Option<&'static str> {
        self.opaque
    }

    /// Every reversed read, in walk order.
    pub(crate) fn triggers(&self) -> &[Trigger] {
        &self.triggers
    }
}

// ── The walk ────────────────────────────────────────────────────────────────────

/// Which node a read is anchored at.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Root {
    /// The node whose constraints are being walked — a property shape's VALUE
    /// node, or a node shape's focus node.
    Node,
    /// The node that DECLARED those constraints. Inside a property shape this is
    /// the focus node rather than the value node, which is what the property-pair
    /// constraints (`sh:equals` and friends) read their comparand from: SHACL
    /// §4.3 compares the value nodes against the objects of the comparand
    /// predicate ON THE FOCUS NODE, not on each value node.
    Declaring,
}

/// The chain state a `enter_*` call displaced, to be handed back to [`FootprintWalk::leave`].
pub(crate) struct ChainState {
    chain: Option<Vec<Path>>,
    declaring: Option<Vec<Path>>,
}

/// The footprint accumulator carried by the shapes-lowering walk.
///
/// It holds no lowering of its own: every recursion, every cycle guard and every
/// visit is the lowering walk's, and this type only observes what that walk
/// reaches. That is the whole point — a second traversal would be a second
/// transcription of the reachability rule, and the two would drift.
#[derive(Debug)]
pub(crate) struct FootprintWalk {
    /// Every read recorded so far, with its chain still pointing forwards.
    triggers: Vec<PendingTrigger>,
    /// Why this footprint is TOP, or `None` while it is still bounded.
    opaque: Option<&'static str>,
    /// Forward hops from a focus node to the node currently being walked, or
    /// `None` when that node is not reachable from a focus node by a path this
    /// walk can name.
    chain: Option<Vec<Path>>,
    /// The same, for the node that declared the constraints being walked.
    declaring: Option<Vec<Path>>,
}

impl Default for FootprintWalk {
    fn default() -> Self {
        Self {
            triggers: Vec::new(),
            opaque: None,
            chain: Some(Vec::new()),
            declaring: Some(Vec::new()),
        }
    }
}

impl FootprintWalk {
    /// Hand over every read recorded so far, for reversal and lowering.
    ///
    /// Taken rather than borrowed because the caller holds the slot row a
    /// [`LoweredPath`] indexes and needs it mutably while it lowers these chains;
    /// the walk is finished with them either way.
    pub(crate) fn take_triggers(&mut self) -> Vec<PendingTrigger> {
        std::mem::take(&mut self.triggers)
    }

    /// Seal the accumulator into the footprint it derived, given the reversed,
    /// lowered form of every trigger [`Self::take_triggers`] handed out.
    pub(crate) fn finish(self, triggers: Vec<Trigger>) -> Footprint {
        Footprint {
            triggers,
            opaque: self.opaque,
        }
    }

    /// Record the FIRST reason this footprint went TOP.
    ///
    /// First, not last, because the reason is reported to a caller who has to act
    /// on it, and the earliest construct in walk order is the one they can find.
    fn mark_opaque(&mut self, reason: &'static str) {
        if self.opaque.is_none() {
            self.opaque = Some(reason);
        }
    }

    /// Enter a shape whose focus node is the node currently being walked.
    pub(crate) fn enter_shape(&mut self) -> ChainState {
        let saved = self.save();
        self.declaring.clone_from(&self.chain);
        saved
    }

    /// Enter a property shape: record every step of `path` as a read from the node
    /// currently being walked, then move to that path's VALUE nodes.
    pub(crate) fn enter_values(&mut self, path: &Path) -> ChainState {
        let saved = self.save();
        self.record_path_steps(path, &[]);
        self.declaring.clone_from(&self.chain);
        if let Some(chain) = &mut self.chain {
            chain.push(path.clone());
        }
        saved
    }

    /// Enter a subtree whose nodes this walk cannot reach from a focus node.
    ///
    /// Not itself an opacity: a subtree that reads nothing costs nothing. The
    /// footprint goes TOP only if a read is actually emitted while unrooted, which
    /// is what keeps a `sh:filterShape` carrying only `sh:datatype` bounded.
    pub(crate) fn enter_unrooted(&mut self) -> ChainState {
        let saved = self.save();
        self.chain = None;
        self.declaring = None;
        saved
    }

    /// Restore the chain state a matching `enter_*` displaced.
    pub(crate) fn leave(&mut self, saved: ChainState) {
        self.chain = saved.chain;
        self.declaring = saved.declaring;
    }

    fn save(&self) -> ChainState {
        ChainState {
            chain: self.chain.clone(),
            declaring: self.declaring.clone(),
        }
    }

    // ── Emission ────────────────────────────────────────────────────────────────

    /// Record one read: predicate `predicate`, anchored at `root` extended by
    /// `prefix`, with the anchored node at `endpoint` of the matched triple.
    fn emit(
        &mut self,
        root: Root,
        prefix: &[Path],
        predicate: Option<NamedNode>,
        endpoint: Endpoint,
    ) {
        let base = match root {
            Root::Node => self.chain.clone(),
            Root::Declaring => self.declaring.clone(),
        };
        let Some(base) = base else {
            self.mark_opaque(OPAQUE_UNROOTED);
            return;
        };
        let mut hops = base;
        hops.extend_from_slice(prefix);
        self.triggers.push(PendingTrigger {
            predicate,
            endpoint,
            chain: compose(hops),
        });
    }

    /// Record every STEP of `path`, each with the prefix that reaches it.
    ///
    /// A step is a read, and the node it reads from is the one the earlier steps
    /// arrive at — so a sequence path `(ex:a ex:b)` contributes a read of `ex:a`
    /// from the focus node and a read of `ex:b` from the node one `ex:a` hop away.
    /// The match carries no wildcard: a new path form must decide what it reads.
    fn record_path_steps(&mut self, path: &Path, prefix: &[Path]) {
        match path {
            Path::Predicate(predicate) => {
                self.emit(
                    Root::Node,
                    prefix,
                    Some(predicate.clone()),
                    Endpoint::Subject,
                );
            }
            // `^p` reads `(?, p, node)`, so the anchored node is the OBJECT. An
            // inverse over a COMPOSITE is pushed inward first, by the same rewrite
            // the lowering uses, so there is exactly one description of what
            // `^(a/b)` means.
            //
            // This rewrite happens HERE, during the walk, and is not hoisted: it is
            // stage-0 work reached once per declared `sh:inversePath` over a
            // composite in the shapes graph, and it rewrites only the subtree that
            // needs it. Pre-normalizing the whole declared path before the walk
            // would visit every node of every path instead of just those, which is
            // strictly more work for the same answer. What must not invert per item
            // is the EVALUATION, and it does not: `Trigger::reversed` is inverted
            // and lowered once, on the way out of the lowering walk.
            Path::Inverse(inner) => match inner.as_ref() {
                Path::Predicate(predicate) => {
                    self.emit(
                        Root::Node,
                        prefix,
                        Some(predicate.clone()),
                        Endpoint::Object,
                    );
                }
                composite => self.record_path_steps(&crate::path::invert(composite), prefix),
            },
            Path::Sequence(parts) => {
                let mut prefix = prefix.to_vec();
                for part in parts {
                    self.record_path_steps(part, &prefix);
                    prefix.push(part.clone());
                }
            }
            Path::Alternative(parts) => {
                for part in parts {
                    self.record_path_steps(part, prefix);
                }
            }
            // A closure reads its inner path from every node the closure has
            // already reached, so the prefix that reaches one of ITS steps is the
            // closure itself. Reversed, that is `(^p)*` — the sound transitive
            // over-approximation, and `*` rather than `+` because a read at the
            // zero-length position is reached from the focus node directly.
            Path::ZeroOrMore(inner) | Path::OneOrMore(inner) => {
                let mut prefix = prefix.to_vec();
                prefix.push(Path::ZeroOrMore(inner.clone()));
                self.record_path_steps(inner, &prefix);
            }
            Path::ZeroOrOne(inner) => self.record_path_steps(inner, prefix),
        }
    }

    /// Record the reads that decide SHACL class membership of the node at `root`.
    ///
    /// Two, because membership is `rdf:type` composed with `rdfs:subClassOf*` and a
    /// change to either moves it:
    ///
    /// 1. a changed `(node, rdf:type, ?)` names the node directly; and
    /// 2. a changed `(C, rdfs:subClassOf, ?)` moves the membership of every node
    ///    typed with `C` or with any subclass of `C` — reached backwards from `C`
    ///    by `(^rdfs:subClassOf)* / ^rdf:type`, which is the reverse of the chain
    ///    recorded here.
    fn record_class_membership(&mut self, root: Root) {
        let rdf_type = NamedNode::new_unchecked(rdf::TYPE);
        let sub_class_of = NamedNode::new_unchecked(rdfs::SUB_CLASS_OF);
        self.emit(root, &[], Some(rdf_type.clone()), Endpoint::Subject);
        self.emit(
            root,
            &[
                Path::Predicate(rdf_type),
                Path::ZeroOrMore(Box::new(Path::Predicate(sub_class_of.clone()))),
            ],
            Some(sub_class_of),
            Endpoint::Subject,
        );
    }

    // ── The four visit points ───────────────────────────────────────────────────

    /// Record the read that decides whether the value triples of a property shape
    /// carry a REIFIER at all.
    ///
    /// `sh:reifierShape` and `sh:reificationRequired` both begin by asking the RDF
    /// 1.2 statement layer which reifiers exist for `<<( focus path value )>>`, and
    /// that question is answered by rows this walk sees nowhere else: a reifier
    /// declaration lives in a side table, surfaces as the virtual quad
    /// `(reifier, rdf:reifies, <<( focus path value )>>)`, and names neither the
    /// focus node nor the value node in a slot an ordinary trigger inspects.
    ///
    /// It is recorded whether or not any reifier shape follows, because the
    /// EXISTENCE answer alone moves a verdict in both directions: gaining a reifier
    /// retracts an `sh:reificationRequired` violation, and gaining one also submits a
    /// new node to every `sh:reifierShape`. Recording it only when a reifier shape
    /// happens to emit a read of its own would leave `sh:reificationRequired` — which
    /// reaches no nested shape at all — described by nothing.
    ///
    /// The anchor is [`Root::Declaring`]: the triple term's subject is the node that
    /// DECLARED the property shape, not one of the value nodes the path arrives at.
    pub(crate) fn record_reification(&mut self) {
        self.emit(
            Root::Declaring,
            &[],
            Some(NamedNode::new_unchecked(rdf::REIFIES)),
            Endpoint::ObjectTripleSubject,
        );
    }

    // ── The three constraint-side visit points ──────────────────────────────────

    /// Record what one TARGET declaration reads to decide membership.
    ///
    /// The match carries no wildcard for the reason [`Self::record_constraint`]'s
    /// does not.
    pub(crate) fn record_target(&mut self, target: &Target) {
        match target {
            Target::Class(_) | Target::ImplicitClass(_) => {
                self.record_class_membership(Root::Node);
            }
            Target::SubjectsOf(predicate) => {
                self.emit(Root::Node, &[], Some(predicate.clone()), Endpoint::Subject);
            }
            Target::ObjectsOf(predicate) => {
                self.emit(Root::Node, &[], Some(predicate.clone()), Endpoint::Object);
            }
            // A constant focus node reads nothing to BE one. Whether it violates is
            // decided by its constraints, which record their own reads.
            Target::Node(_) => {}
            Target::Sparql { .. } => self.mark_opaque(OPAQUE_QUERY_TEXT),
        }
    }

    /// Record what ONE constraint reads, at the node the lowering walk currently
    /// stands on.
    ///
    /// Only the constraint's OWN reads: every constraint that reaches a nested
    /// shape, a sibling shape list or a node expression is descended into by the
    /// lowering walk, which visits this function again at the right node.
    ///
    /// The match carries no wildcard on purpose, and the reason is the whole point
    /// of this module: a constraint kind nobody classified here would read triples
    /// no trigger describes, the expansion would quietly come up short, and every
    /// existing test would still pass.
    pub(crate) fn record_constraint(&mut self, constraint: &Constraint) {
        match constraint {
            Constraint::Class(_) => self.record_class_membership(Root::Node),
            // `sh:closed` inspects EVERY outgoing triple of its node, so it binds
            // no predicate — bounded all the same, because it still binds the
            // subject to a node the chain reaches.
            Constraint::Closed { .. } => self.emit(Root::Node, &[], None, Endpoint::Subject),
            // SHACL §4.3: the comparand is read from the node that DECLARED the
            // property shape, never from its value nodes.
            Constraint::Equals(predicate)
            | Constraint::Disjoint(predicate)
            | Constraint::LessThan(predicate)
            | Constraint::LessThanOrEquals(predicate) => self.emit(
                Root::Declaring,
                &[],
                Some(predicate.clone()),
                Endpoint::Subject,
            ),
            Constraint::Sparql { .. } => self.mark_opaque(OPAQUE_QUERY_TEXT),
            Constraint::Component { .. } => self.mark_opaque(OPAQUE_COMPONENT),
            // Value-local: each of these judges a value node's own identity —
            // its term kind, its lexical form, its datatype, its language tag, its
            // numeric order, its membership in a listed set — and reads no triple
            // beyond the ones the path already read to produce it.
            Constraint::Datatype(_)
            | Constraint::NodeKind(_)
            | Constraint::MinCount(_)
            | Constraint::MaxCount(_)
            | Constraint::In(_)
            | Constraint::HasValue(_)
            | Constraint::Pattern { .. }
            | Constraint::MinLength(_)
            | Constraint::MaxLength(_)
            | Constraint::UniqueLang(_)
            | Constraint::LanguageIn(_)
            | Constraint::MinInclusive(_)
            | Constraint::MaxInclusive(_)
            | Constraint::MinExclusive(_)
            | Constraint::MaxExclusive(_) => {}
            // Structural: the reads belong to what these REACH, and the lowering
            // walk reaches it.
            Constraint::Not(_)
            | Constraint::And(_)
            | Constraint::Or(_)
            | Constraint::Xone(_)
            | Constraint::Node(_)
            | Constraint::QualifiedValueShape { .. }
            | Constraint::Expression { .. }
            | Constraint::NodeByExpression { .. } => {}
        }
    }

    /// Record what ONE node expression reads, at the node the lowering walk
    /// currently stands on.
    ///
    /// Wildcard-free for the same reason as [`Self::record_constraint`]. The nested
    /// `matches!` tests are deliberately NOT a second dispatch on `NodeExpr`: they
    /// ask whether one operand is literally `shnex:this`, and a variant nobody
    /// taught them about answers "no" and falls to the conservative branch, which
    /// over-approximates. A wildcard in the OUTER match would fall the other way.
    pub(crate) fn record_expression(&mut self, expr: &NodeExpr) {
        match expr {
            // A path expression is evaluated from the current focus node, so its
            // steps are read exactly as a property shape's are.
            NodeExpr::Path(path) => self.record_path_steps(path, &[]),
            // `shnex:nodes` names the expression the path starts from. Rooted at
            // `shnex:this` it is the same read; anything else starts somewhere this
            // walk cannot name, and the reads are recorded as unrooted.
            NodeExpr::PathValues { focus, path } => {
                if matches!(focus.as_ref(), NodeExpr::This) {
                    self.record_path_steps(path, &[]);
                } else {
                    let saved = self.enter_unrooted();
                    self.record_path_steps(path, &[]);
                    self.leave(saved);
                }
            }
            // Graph-wide selections: their result moves when ANY node's type or
            // conformance moves, so no path from a focus node describes them.
            NodeExpr::InstancesOf(_) | NodeExpr::NodesMatching(_) => {
                self.mark_opaque(OPAQUE_GLOBAL_EXPRESSION);
            }
            // Query text, by construction.
            NodeExpr::Select { .. } => self.mark_opaque(OPAQUE_QUERY_TEXT),
            // A builtin and a `sparql:` operator are rendered as SPARQL
            // EXPRESSIONS over their already-evaluated operands — no graph pattern,
            // so no read. A `sh:SPARQLFunction` body is a SELECT that may carry
            // one, and this walk does not read it.
            NodeExpr::Call(call) => match call {
                FnCall::Builtin { .. } | FnCall::Sparql { .. } => {}
                FnCall::UserDefined { .. } => self.mark_opaque(OPAQUE_QUERY_TEXT),
            },
            // Value-producing or purely structural: every read belongs to an
            // operand the lowering walk descends into.
            NodeExpr::Constant(_)
            | NodeExpr::This
            | NodeExpr::Empty
            | NodeExpr::Var(_)
            | NodeExpr::Arg(_)
            | NodeExpr::List(_)
            | NodeExpr::Filter { .. }
            | NodeExpr::FindFirst { .. }
            | NodeExpr::MatchAll { .. }
            | NodeExpr::ConformsToShape { .. }
            | NodeExpr::CustomCall { .. }
            | NodeExpr::Remove { .. }
            | NodeExpr::FlatMap { .. }
            | NodeExpr::Union(_)
            | NodeExpr::Intersection(_)
            | NodeExpr::Concat(_)
            | NodeExpr::If { .. }
            | NodeExpr::Count { .. }
            | NodeExpr::Distinct(_)
            | NodeExpr::Min(_)
            | NodeExpr::Max(_)
            | NodeExpr::Sum(_)
            | NodeExpr::Limit { .. }
            | NodeExpr::Offset { .. }
            | NodeExpr::OrderBy { .. }
            | NodeExpr::Exists(_) => {}
        }
    }
}

/// Whether the shape a node expression applies stays anchored at the current node.
///
/// `shnex:filterShape` and `shnex:conformsToShape` judge the nodes their operand
/// produces. When that operand is literally `shnex:this` the judged node is the one
/// the walk already stands on and the shape's reads keep their chain; otherwise the
/// judged nodes are computed and the shape is walked unrooted.
pub(crate) fn applies_to_current_node(nodes: &NodeExpr) -> bool {
    matches!(nodes, NodeExpr::This)
}

/// Fold hops into the single path that traverses them in order.
fn compose(mut hops: Vec<Path>) -> Option<Path> {
    match hops.len() {
        0 => None,
        1 => hops.pop(),
        _ => Some(Path::Sequence(hops)),
    }
}

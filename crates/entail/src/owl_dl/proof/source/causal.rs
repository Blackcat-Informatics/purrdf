// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrowed graph capabilities created only by question-bound native verification.

use super::{
    ResolvedAxiomSource, ResolvedConcept, ResolvedReserved, ResolvedRole, ResolvedTerm, origin,
    role, term,
};
use crate::owl_dl::graph::changes::{
    Cause, Mutation, Origin, PathKind, ReadFact, ReadOwner, RoleWitness, UniversalLicence,
    slice::{self, Observation, Support, Vertex},
};
use crate::owl_dl::proof::{DlProof, DlProofError, RefutationReplay, capture::Capture};

mod branch;
mod schema;
pub use schema::CausalSchema;
mod syntax;
pub use branch::{CausalAlternative, CausalBranch, CausalGround};
#[cfg(test)]
mod tests;
use crate::owl_dl::{
    Kb,
    input_source::{Input, Source},
};
pub use syntax::{
    CausalRoleSource, CausalRoleSourceKind, CausalSyntaxKind, CausalSyntaxNode, CausalSyntaxRead,
    CausalTriple,
};

/// Stable causal refusal category. Native stopping and allocation retain their original errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CausalSupportKind {
    /// No complete native proof was recorded.
    Recording,
    /// A legacy clash lacks current-support evidence.
    CurrentSupport,
    /// The selected native run did not decide its question.
    Undecided,
    /// Some refutation branch has no fully replayed closure.
    Unclosed,
    /// A required native fact version is absent.
    Fact,
    /// A selected role path lacks its license.
    Path,
    /// A selected native effect lacks its complete license.
    Effect,
    /// Key-equality support is missing or inconsistent.
    Key,
    /// An original key membership lacks checked support.
    Membership,
    /// The selected terminal clash is absent.
    Clash,
    /// A required original input anchor is absent.
    Input,
    /// Original expression syntax is unavailable.
    Syntax,
    /// A checked schema license is absent or belongs to another owner.
    Schema,
    /// The original role compiler's license is unavailable.
    RoleLicence,
}

pub(crate) fn support_error(run: usize, clash: usize, error: &slice::Error) -> DlProofError {
    use crate::owl_dl::roles::LicenceError;
    let kind = match error {
        slice::Error::Storage(original)
        | slice::Error::RoleLicence(LicenceError::Storage(original)) => {
            return DlProofError::Storage(*original);
        }
        slice::Error::Stopped | slice::Error::RoleLicence(LicenceError::Stopped) => {
            return DlProofError::Stopped;
        }
        slice::Error::MissingFact => CausalSupportKind::Fact,
        slice::Error::MissingPath => CausalSupportKind::Path,
        slice::Error::MissingEffect => CausalSupportKind::Effect,
        slice::Error::MissingKey(_) => CausalSupportKind::Key,
        slice::Error::MissingMembership { .. } => CausalSupportKind::Membership,
        slice::Error::MissingClash => CausalSupportKind::Clash,
        slice::Error::MissingInput(_) => CausalSupportKind::Input,
        slice::Error::MissingSyntax => CausalSupportKind::Syntax,
        slice::Error::MissingSchema(_) | slice::Error::UnexpectedSchema => {
            CausalSupportKind::Schema
        }
        slice::Error::RoleLicence(_) => CausalSupportKind::RoleLicence,
    };
    DlProofError::CausalSupport {
        run: Some(run),
        clash: Some(clash),
        kind,
        detail: format!("{error:?}"),
    }
}

/// A complete original refutation and its already checked, selected dependencies.
#[derive(Clone, Copy)]
pub struct CausalRefutation<'a> {
    proof: &'a DlProof,
    kb: &'a Kb,
    replay: &'a RefutationReplay,
    capture: &'a Capture,
}

impl<'a> CausalRefutation<'a> {
    pub(crate) const fn checked(
        proof: &'a DlProof,
        kb: &'a Kb,
        replay: &'a RefutationReplay,
        capture: &'a Capture,
    ) -> Self {
        Self {
            proof,
            kb,
            replay,
            capture,
        }
    }

    /// Original proof, whose branch licenses were checked before this capability existed.
    #[must_use]
    pub const fn proof(&self) -> &'a DlProof {
        self.proof
    }

    /// Original closure counts and explicit trust classification.
    #[must_use]
    pub const fn replay(&self) -> &'a RefutationReplay {
        self.replay
    }

    /// Original exhaustive branch licenses, resolved without regenerating alternatives.
    pub fn branches(&self) -> impl ExactSizeIterator<Item = CausalBranch<'a>> + 'a + use<'a> {
        let this = *self;
        (0..self.proof.branches.len())
            .map(move |ordinal| CausalBranch::checked(this.proof, this.kb, ordinal))
    }

    /// Checked clause and inequality-closing merge leaves in original traversal order.
    pub fn clashes(&self) -> impl ExactSizeIterator<Item = CausalClash<'a>> + 'a + use<'a> {
        let this = *self;
        this.capture.clashes.iter().map(move |clash| CausalClash {
            proof: this.proof,
            kb: this.kb,
            index: clash.index,
            support: clash
                .support
                .as_ref()
                .expect("causal capability requires complete selected support"),
        })
    }
}

impl std::fmt::Debug for CausalRefutation<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CausalRefutation")
            .field("replay", &self.replay)
            .field("capture", &self.capture)
            .finish()
    }
}

/// One selected clash graph, sealed to its native proof and consumer source.
#[derive(Clone, Copy)]
pub struct CausalClash<'a> {
    proof: &'a DlProof,
    kb: &'a Kb,
    index: usize,
    support: &'a Support,
}

impl<'a> CausalClash<'a> {
    /// Whether this leaf is an independently checked inequality-closing merge.
    /// Its index then addresses the original proof's merges rather than clauses.
    #[must_use]
    pub fn is_merge(&self) -> bool {
        matches!(self.root().observation(), CausalObservation::Merge { .. })
    }
    /// Original clause-clash or merge ordinal, as distinguished by `is_merge`.
    /// Not a source term or a cross-proof identity.
    #[must_use]
    pub const fn index(&self) -> usize {
        self.index
    }

    /// The terminal dependency vertex. Empty graphs cannot produce this capability.
    #[must_use]
    pub fn root(&self) -> CausalStep<'a> {
        self.step(self.support.derivation.root)
    }

    fn step(&self, id: usize) -> CausalStep<'a> {
        let graph = &self.support.derivation;
        let at = graph
            .vertices
            .binary_search_by_key(&id, |vertex| vertex.id)
            .expect("selected parents belong to the checked dependency closure");
        CausalStep {
            scope: *self,
            vertex: &graph.vertices[at],
        }
    }

    /// Exactly the backwards-reachable vertices, in original replay order.
    pub fn steps(&self) -> impl ExactSizeIterator<Item = CausalStep<'a>> + 'a + use<'a> {
        let scope = *self;
        self.support
            .derivation
            .vertices
            .iter()
            .map(move |vertex| CausalStep { scope, vertex })
    }

    /// Original asserted, literal and key-inference inputs selected by the native derivation.
    pub fn inputs(&self) -> impl ExactSizeIterator<Item = CausalInput<'a>> + 'a + use<'a> {
        let kb = self.kb;
        self.support.inputs.iter().map(move |input| CausalInput {
            kb,
            input: input.input,
            source: input.source,
        })
    }

    /// Actual schema-bound derivation checked for this leaf, when present.
    #[must_use]
    pub fn schema(&self) -> Option<CausalSchema<'a>> {
        if self.is_merge() {
            return None;
        }
        self.proof.clashes[self.index]
            .schema
            .as_ref()
            .map(|evidence| CausalSchema {
                scope: *self,
                evidence,
            })
    }

    /// Selected original RDF expression/list/range syntax, preserving shared DAG roots.
    pub fn syntax_roots(
        &self,
    ) -> impl ExactSizeIterator<Item = CausalSyntaxNode<'a>> + 'a + use<'a> {
        let scope = *self;
        self.support
            .syntax
            .iter()
            .map(move |&(space, ordinal)| CausalSyntaxNode::checked(scope, space, ordinal))
    }

    /// Selected native role-compiler source operations and their original RDF reads.
    pub fn role_sources(
        &self,
    ) -> impl ExactSizeIterator<Item = CausalRoleSource<'a>> + 'a + use<'a> {
        let scope = *self;
        self.support
            .roles
            .iter()
            .map(move |&input| CausalRoleSource::checked(scope, input))
    }
}

impl std::fmt::Debug for CausalClash<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CausalClash")
            .field("index", &self.index)
            .field("graph", &self.support.derivation)
            .finish()
    }
}

/// Source ownership of one selected input, retaining exact consumer RDF values.
#[derive(Clone, Copy)]
pub struct CausalInput<'a> {
    kb: &'a Kb,
    input: Input,
    source: Source,
}

/// Original source operation; a key inference is not an asserted equality triple.
#[derive(Debug)]
pub enum CausalInputSource<'a> {
    /// Original authored axiom or native/query normalization operation.
    Axiom(ResolvedAxiomSource<'a>),
    /// Exact RDF literal denotation, including datatype, language and direction.
    Literal(ResolvedTerm<'a>),
    /// Original native key equality; its ordinal has no RDF term meaning.
    Key {
        /// Original key ordinal in the consumer's prepared input.
        key: usize,
        /// First original key member, with exact RDF metadata.
        left: ResolvedTerm<'a>,
        /// Second original key member, with exact RDF metadata.
        right: ResolvedTerm<'a>,
    },
}

impl<'a> CausalInput<'a> {
    /// Resolve against the sealed consumer term table, never a producer-supplied label.
    #[must_use]
    pub fn source(&self) -> CausalInputSource<'a> {
        match self.source {
            Source::Anchor(source) => CausalInputSource::Axiom(origin::resolve(self.kb, source)),
            Source::LiteralValue(id) => CausalInputSource::Literal(term(self.kb, id)),
            Source::KeyInference { key, left, right } => CausalInputSource::Key {
                key,
                left: term(self.kb, left),
                right: term(self.kb, right),
            },
        }
    }

    /// Original pre-key membership warrants for a selected key inference.
    /// Every member's native refutation was checked before the equality was appended.
    #[must_use]
    pub fn key_memberships(&self) -> Option<[CausalMembership<'a>; 2]> {
        let Source::KeyInference { key, left, right } = self.source else {
            return None;
        };
        let crate::owl_dl::key_proofs::Records::Ready(evidence) = &self.kb.key_proofs else {
            unreachable!("selected key effects require their original membership owner");
        };
        Some([left, right].map(|individual| {
            CausalMembership {
                kb: self.kb,
                member: evidence
                    .member(key, individual)
                    .expect("selected key membership was checked in its original phase"),
            }
        }))
    }
    /// Original authored key definition that licensed this selected equality.
    #[must_use]
    pub fn key_definition(&self) -> Option<Self> {
        let Source::KeyInference { key, .. } = self.source else {
            return None;
        };
        Some(prepared_input(self.kb, Input::Key(key)))
    }

    /// Actual original common-value witnesses, in declared key-property order.
    /// Non-key inputs have no value witnesses; missing key evidence refuses before publication.
    pub fn key_values(
        &self,
    ) -> Option<impl ExactSizeIterator<Item = CausalKeyValue<'a>> + 'a + use<'a>> {
        let Source::KeyInference { .. } = self.source else {
            return None;
        };
        let Input::Equality(equality) = self.input else {
            unreachable!("key-inference inputs address their original prepared equality");
        };
        let values = self
            .kb
            .inputs
            .key_values
            .as_ref()
            .and_then(|reads| reads.get(equality))
            .expect("selected key equalities retain their original value evidence");
        let kb = self.kb;
        Some(values.iter().map(move |value| CausalKeyValue { kb, value }))
    }
}

fn prepared_input(kb: &Kb, input: Input) -> CausalInput<'_> {
    let source = kb
        .inputs
        .get(input)
        .expect("selected key inputs retain their original parser owner");
    CausalInput { kb, input, source }
}

/// One native common-value decision and the exact two original asserted rows it used.
#[derive(Clone, Copy)]
pub struct CausalKeyValue<'a> {
    kb: &'a Kb,
    value: &'a crate::owl_dl::key_values::Common,
}

impl<'a> CausalKeyValue<'a> {
    /// Original key property in the consumer's vocabulary.
    #[must_use]
    pub fn property(&self) -> ResolvedRole<'a> {
        role(
            self.kb,
            crate::owl_dl::concept::Role::Named(self.value.property),
        )
    }

    /// Native value whose equality licensed the selected pair of assertions.
    #[must_use]
    pub fn value(&self) -> ResolvedTerm<'a> {
        term(self.kb, self.value.value)
    }

    /// Original first-member assertion, retaining its exact RDF terms.
    #[must_use]
    pub fn left(&self) -> CausalInput<'a> {
        prepared_input(self.kb, Input::Role(self.value.left))
    }

    /// Original second-member assertion, retaining its exact RDF terms.
    #[must_use]
    pub fn right(&self) -> CausalInput<'a> {
        prepared_input(self.kb, Input::Role(self.value.right))
    }
}

impl std::fmt::Debug for CausalKeyValue<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CausalKeyValue")
            .field("property", &self.property())
            .field("value", &self.value())
            .field("left", &self.left())
            .field("right", &self.right())
            .finish()
    }
}

/// Original membership warrant, retaining its pre-key proof and every checked branch.
#[derive(Clone, Copy)]
pub struct CausalMembership<'a> {
    kb: &'a Kb,
    member: &'a crate::owl_dl::key_proofs::Membership,
}

impl<'a> CausalMembership<'a> {
    /// Exact original source member.
    #[must_use]
    pub fn individual(&self) -> ResolvedTerm<'a> {
        term(self.kb, self.member.individual)
    }

    /// Original normalized key class whose membership was established.
    #[must_use]
    pub fn class(&self) -> ResolvedConcept<'a> {
        ResolvedConcept::of(self.kb, self.member.class)
    }

    /// Already checked original negative-membership refutation, without another search.
    #[must_use]
    pub fn refutation(&self) -> CausalRefutation<'a> {
        let crate::owl_dl::key_proofs::Verification::Closed(replay, capture) =
            &self.member.verification
        else {
            unreachable!("selected key effects require fully checked original membership branches");
        };
        CausalRefutation::checked(&self.member.proof, self.kb, replay, capture)
    }
}

impl std::fmt::Debug for CausalMembership<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CausalMembership")
            .field("individual", &self.individual())
            .field("class", &self.class())
            .finish()
    }
}

impl std::fmt::Debug for CausalInput<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.source().fmt(f)
    }
}

/// A vertex handle is meaningful only inside its borrowed checked graph.
#[derive(Clone, Copy)]
pub struct CausalStep<'a> {
    scope: CausalClash<'a>,
    vertex: &'a Vertex,
}

/// The selected observation that established one dependency vertex.
#[derive(Debug)]
pub enum CausalObservation<'a> {
    /// Exact native write, with its original cause owner and head position.
    Effect {
        /// Original native source or checked-application owner.
        cause: CausalCause<'a>,
        /// Actual primitive recorded by the native graph.
        mutation: CausalMutation<'a>,
    },
    /// Original checked application; the ordinal addresses this clash's checked prefix.
    Application {
        /// Application ordinal within this clash's checked prefix.
        application: usize,
        /// Clause ordinal in the independently prepared calculus.
        clause: usize,
        /// Original branch and alternative ordinals, checked against the refutation tree.
        branch: Option<(usize, usize)>,
    },
    /// Native counting head's selected pre-state clique.
    Counting {
        /// Original checked application, also retained as this vertex's parent.
        application: usize,
        /// Original at-least head-atom ordinal.
        atom: usize,
    },
    /// Reachable native identification closed by an existing checked inequality.
    Merge {
        /// Original checked final application.
        application: usize,
        /// Original identifying head atom.
        atom: usize,
    },
    /// Terminal checked empty-head clause.
    Clash {
        /// Empty-head clause ordinal in the independently prepared calculus.
        clause: usize,
    },
}

impl<'a> CausalStep<'a> {
    /// Original replay-local vertex ordinal, never a source identity.
    #[must_use]
    pub const fn ordinal(&self) -> usize {
        self.vertex.id
    }

    /// Actual immutable parent versions; no unrelated earlier application is added.
    pub fn parents(&self) -> impl ExactSizeIterator<Item = CausalStep<'a>> + 'a + use<'a> {
        let scope = self.scope;
        scope.support.derivation.parents[self.vertex.parents.clone()]
            .iter()
            .map(move |&id| scope.step(id))
    }

    /// Exact native effect or checked read owner selected in the existing replay.
    #[must_use]
    pub fn observation(&self) -> CausalObservation<'a> {
        match self.vertex.observation {
            Observation::Effect(index) => {
                let event = self.scope.support.effects[index].1;
                CausalObservation::Effect {
                    cause: CausalCause {
                        scope: self.scope,
                        cause: event.cause,
                    },
                    mutation: mutation(self.scope, event.mutation),
                }
            }
            Observation::Reads(index) => match self.scope.support.owners[index] {
                ReadOwner::Application {
                    index,
                    clause,
                    branch,
                } => CausalObservation::Application {
                    application: index,
                    clause,
                    branch,
                },
                ReadOwner::Clash { clause } => CausalObservation::Clash { clause },
                ReadOwner::Counting { application, atom } => {
                    CausalObservation::Counting { application, atom }
                }
                ReadOwner::Merge { application, atom } => {
                    CausalObservation::Merge { application, atom }
                }
            },
        }
    }

    /// Original successful body reads, in their recorded order before this vertex's writes.
    pub fn reads(&self) -> impl ExactSizeIterator<Item = CausalFact<'a>> + 'a + use<'a> {
        let scope = self.scope;
        scope.support.derivation.reads[self.vertex.reads.clone()]
            .iter()
            .map(move |read| fact(scope, read.fact))
    }

    /// Original accepted automaton steps. Ordinals address the read's original path span.
    pub fn paths(&self) -> impl ExactSizeIterator<Item = CausalPathStep<'a>> + 'a + use<'a> {
        let scope = self.scope;
        let span = self.vertex.paths.clone();
        scope.support.path_ordinals[span.clone()]
            .iter()
            .copied()
            .zip(scope.support.paths[span].iter())
            .map(move |(ordinal, step)| CausalPathStep {
                ordinal,
                role: role(scope.kb, step.role),
                from: CausalNode {
                    scope,
                    node: step.from.0,
                },
                from_state: step.from.1,
                to: CausalNode {
                    scope,
                    node: step.to.0,
                },
                to_state: step.to.1,
                transition: step.transition,
                kind: match step.kind {
                    PathKind::Edge { index, letter } => CausalPathKind::Edge {
                        ordinal: index,
                        role: role(scope.kb, letter),
                    },
                    PathKind::Epsilon => CausalPathKind::Epsilon,
                    PathKind::Universal(letter) => {
                        CausalPathKind::Universal(role(scope.kb, letter))
                    }
                },
            })
    }

    /// Original input licenses of the hierarchy reads made at this vertex.
    pub fn rule_inputs(&self) -> impl ExactSizeIterator<Item = CausalInput<'a>> + 'a + use<'a> {
        let scope = self.scope;
        scope.support.rules[self.vertex.rules.clone()]
            .iter()
            .map(move |&input| input_view(scope, input))
    }

    /// Original shared transitive-composition witnesses, without expanding joins.
    pub fn transitive_steps(
        &self,
    ) -> impl ExactSizeIterator<Item = CausalTransitiveStep<'a>> + 'a + use<'a> {
        let scope = self.scope;
        scope.support.transitive_nodes[self.vertex.transitive_nodes.clone()]
            .iter()
            .map(move |step| CausalTransitiveStep {
                root: step.root,
                ordinal: step.id,
                kind: match step.path {
                    crate::owl_dl::graph::changes::reach::Path::Step(edge) => {
                        CausalTransitiveKind::Edge {
                            ordinal: edge.edge,
                            from: CausalNode {
                                scope,
                                node: edge.from,
                            },
                            to: CausalNode {
                                scope,
                                node: edge.to,
                            },
                            role: role(scope.kb, edge.letter),
                        }
                    }
                    crate::owl_dl::graph::changes::reach::Path::Join { first, second } => {
                        CausalTransitiveKind::Join { first, second }
                    }
                },
            })
    }
}

/// One original immutable native transitive-path node.
#[derive(Debug)]
pub struct CausalTransitiveStep<'a> {
    /// Selected read's original transitive root.
    pub root: usize,
    /// Original path node ordinal within that replay.
    pub ordinal: usize,
    /// Original edge or shared composition.
    pub kind: CausalTransitiveKind<'a>,
}

/// Shared native transitive composition, preserving exact original edge orientation.
#[derive(Debug)]
pub enum CausalTransitiveKind<'a> {
    /// Original checked edge.
    Edge {
        /// Original native raw-edge ordinal.
        ordinal: usize,
        /// Original source endpoint.
        from: CausalNode<'a>,
        /// Original destination endpoint.
        to: CausalNode<'a>,
        /// Actual signed edge role.
        role: ResolvedRole<'a>,
    },
    /// Original composition; children precede this path node.
    Join {
        /// Original first path node ordinal.
        first: usize,
        /// Original second path node ordinal.
        second: usize,
    },
}

fn input_view(scope: CausalClash<'_>, input: Input) -> CausalInput<'_> {
    let selected = scope
        .support
        .inputs
        .iter()
        .find(|selected| selected.input == input)
        .expect("selected rule inputs were bound before causal publication");
    CausalInput {
        kb: scope.kb,
        input: selected.input,
        source: selected.source,
    }
}

/// Native all-object-pairs license chosen by the actual role reader.
#[derive(Debug)]
pub enum CausalUniversal<'a> {
    /// Fixed meaning of the standard top object property.
    Fixed,
    /// Original simple hierarchy path from this signed top role.
    Hierarchy(ResolvedRole<'a>),
    /// Original compiled equivalence component containing this top role.
    Component(ResolvedRole<'a>),
}

/// Selected native reachability evidence; ordinals remain local to this checked graph.
#[derive(Debug)]
pub enum CausalRoleWitness<'a> {
    /// Actual raw edge ordinal and original signed role.
    Edge {
        /// Original native edge ordinal.
        ordinal: usize,
        /// Original role and edge orientation.
        role: ResolvedRole<'a>,
    },
    /// Original journal span, resolved through this vertex's `paths` iterator.
    Language(std::ops::Range<usize>),
    /// Native transitive witness root and original transitive role.
    Transitive {
        /// Original root of the native transitive composition graph.
        root: usize,
        /// Actual transitive role whose closure established the read.
        via: ResolvedRole<'a>,
    },
    /// Native all-object-pairs decision.
    Universal(CausalUniversal<'a>),
}

/// One successful native body read, expressed in the sealed consumer vocabulary.
#[derive(Debug)]
pub enum CausalFact<'a> {
    /// Original generative birth and alias paths licensing nominal introduction.
    Predecessor {
        /// Tree child whose generating predecessor pressed the nominal bound.
        child: CausalNode<'a>,
        /// Original parent recorded at that birth.
        parent: CausalNode<'a>,
        /// Current nominal representative of the generated child.
        node: CausalNode<'a>,
        /// Current blockable representative of the original parent.
        predecessor: CausalNode<'a>,
        /// Counted role of the original at-most restriction.
        role: ResolvedRole<'a>,
        /// Original signed role making the parent a neighbour of the nominal.
        letter: ResolvedRole<'a>,
    },
    /// Existing replay-local node.
    Node(CausalNode<'a>),
    /// Membership in the native object domain.
    ObjectDomain(CausalNode<'a>),
    /// Universal object-domain class membership.
    Top(CausalNode<'a>),
    /// Original pairwise inequality used by the native selected clique.
    Distinct(CausalNode<'a>, CausalNode<'a>),
    /// Selected normalized class membership.
    Concept(CausalNode<'a>, ResolvedConcept<'a>),
    /// Exact selected term denotation.
    Denotes(CausalNode<'a>, ResolvedTerm<'a>),
    /// Selected native role reachability and its original witness.
    Role {
        /// Original source endpoint.
        from: CausalNode<'a>,
        /// Original destination endpoint.
        to: CausalNode<'a>,
        /// Requested role in the consumer's vocabulary.
        role: ResolvedRole<'a>,
        /// Native reachability witness actually accepted for this read.
        witness: CausalRoleWitness<'a>,
    },
    /// Original normalized TBox inclusion and authored/query source operation.
    Inclusion {
        /// Original normalized antecedent.
        antecedent: ResolvedConcept<'a>,
        /// Original normalized consequent.
        consequent: ResolvedConcept<'a>,
        /// Authored or explicit native/query normalization operation.
        source: ResolvedAxiomSource<'a>,
    },
    /// Clause guard whose actual bound derivation is exposed by the enclosing clash.
    SchemaGuard {
        /// Prepared clause whose bound derivation was checked.
        clause: usize,
    },
    /// Original parser-owned asymmetric/disjoint-role constraint.
    RoleConstraint(CausalInput<'a>),
    /// Original automaton obligation and compiler-generated rule kind.
    RoleState {
        /// Original normalized universal restriction.
        source: ResolvedConcept<'a>,
        /// Actual compiler rule emitted for this obligation.
        kind: CausalRoleStateKind,
    },
}

/// Native rule emitted from a selected universal role-state obligation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CausalRoleStateKind {
    /// Introduce the initial role-state obligation.
    Enter,
    /// Discharge an accepting state to the filler.
    Accept,
    /// Original compiled transition ordinal.
    Transition(usize),
}

fn fact(scope: CausalClash<'_>, fact: ReadFact) -> CausalFact<'_> {
    let node = |node| CausalNode { scope, node };
    let concept = |id| ResolvedConcept::of(scope.kb, id);
    match fact {
        ReadFact::Node(id) => CausalFact::Node(node(id)),
        ReadFact::ObjectDomain(id) => CausalFact::ObjectDomain(node(id)),
        ReadFact::Top(id) => CausalFact::Top(node(id)),
        ReadFact::Predecessor {
            child,
            parent,
            node: current,
            predecessor,
            role: requested,
            letter,
        } => CausalFact::Predecessor {
            child: node(child),
            parent: node(parent),
            node: node(current),
            predecessor: node(predecessor),
            role: role(scope.kb, requested),
            letter: role(scope.kb, letter),
        },
        ReadFact::Distinct { left, right } => CausalFact::Distinct(node(left), node(right)),
        ReadFact::Label {
            node: id,
            concept: idc,
        } => CausalFact::Concept(node(id), concept(idc)),
        ReadFact::Denotes {
            node: id,
            individual,
        } => CausalFact::Denotes(node(id), term(scope.kb, individual)),
        ReadFact::Role {
            from,
            to,
            role: selected,
            witness,
        } => CausalFact::Role {
            from: node(from),
            to: node(to),
            role: role(scope.kb, selected),
            witness: match witness {
                RoleWitness::Edge { index, letter } => CausalRoleWitness::Edge {
                    ordinal: index,
                    role: role(scope.kb, letter),
                },
                RoleWitness::Language { start, end } => CausalRoleWitness::Language(start..end),
                RoleWitness::Transitive { via, path } => CausalRoleWitness::Transitive {
                    root: path,
                    via: role(scope.kb, via),
                },
                RoleWitness::Universal(licence) => CausalRoleWitness::Universal(match licence {
                    UniversalLicence::Fixed => CausalUniversal::Fixed,
                    UniversalLicence::Hierarchy { via } => {
                        CausalUniversal::Hierarchy(role(scope.kb, via))
                    }
                    UniversalLicence::Component { top } => {
                        CausalUniversal::Component(role(scope.kb, top))
                    }
                }),
            },
        },
        ReadFact::Tbox(index) => {
            let (left, right) = scope.kb.tbox[index];
            CausalFact::Inclusion {
                antecedent: concept(left),
                consequent: concept(right),
                source: origin::resolve(scope.kb, scope.kb.tbox_sources[index]),
            }
        }
        ReadFact::SchemaGuard(clause) => CausalFact::SchemaGuard { clause },
        ReadFact::RoleConstraint(input) => CausalFact::RoleConstraint(input_view(scope, input)),
        ReadFact::RoleClause(source) => {
            let program = scope
                .kb
                .role_program
                .as_ref()
                .expect("selected role-state licenses require their original program");
            let obligation = &program.obligations[source.obligation];
            CausalFact::RoleState {
                source: concept(obligation.source),
                kind: match source.kind {
                    crate::owl_dl::roles::RoleClauseKind::Enter => CausalRoleStateKind::Enter,
                    crate::owl_dl::roles::RoleClauseKind::Accept => CausalRoleStateKind::Accept,
                    crate::owl_dl::roles::RoleClauseKind::Transition(index) => {
                        CausalRoleStateKind::Transition(index)
                    }
                },
            }
        }
    }
}

/// One actual selected transition in the native role automaton's product traversal.
#[derive(Debug)]
pub struct CausalPathStep<'a> {
    /// Original journal ordinal, used by the read's original path span.
    pub ordinal: usize,
    /// Role whose compiled automaton accepted this path.
    pub role: ResolvedRole<'a>,
    /// Original source graph endpoint.
    pub from: CausalNode<'a>,
    /// Original source automaton state.
    pub from_state: usize,
    /// Original destination graph endpoint.
    pub to: CausalNode<'a>,
    /// Original destination automaton state.
    pub to_state: usize,
    /// Original compiled transition ordinal.
    pub transition: usize,
    /// Actual native edge, epsilon or top-role operation.
    pub kind: CausalPathKind<'a>,
}

/// Actual native transition operation, without invented source edges.
#[derive(Debug)]
pub enum CausalPathKind<'a> {
    /// Original raw edge and signed role.
    Edge {
        /// Original raw edge ordinal.
        ordinal: usize,
        /// Actual signed role used by this automaton transition.
        role: ResolvedRole<'a>,
    },
    /// Automaton epsilon transition; it does not assert a source edge.
    Epsilon,
    /// Standard top-object-role transition.
    Universal(ResolvedRole<'a>),
}

impl std::fmt::Debug for CausalStep<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CausalStep")
            .field("ordinal", &self.vertex.id)
            .field("observation", &self.observation())
            .finish()
    }
}

/// Original native cause, scoped to this checked graph rather than an RDF name.
#[derive(Clone, Copy)]
pub struct CausalCause<'a> {
    scope: CausalClash<'a>,
    cause: Cause,
}

/// Native effect owner category; ordinal payloads never denote source terms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CausalCauseKind {
    /// Existence of the object domain.
    ObjectDomain,
    /// Original term denotation.
    TermDenotation,
    /// Original prepared class assertion.
    PreparedType,
    /// Original prepared role assertion.
    PreparedRole,
    /// Original prepared asserted or key-derived equality.
    PreparedEquality,
    /// Original prepared inequality.
    PreparedDistinct,
    /// Selected question's explicit class assumption.
    AssumedType,
    /// Selected question's explicit role assumption.
    AssumedRole,
    /// Selected question's anonymous class assumption.
    AssumedAnonymousType,
    /// Anonymous refutation inhabitant.
    RefutationRoot,
    /// Original internalized TBox formula.
    Internalized,
    /// Checked application and native head position.
    Application,
}

impl<'a> CausalCause<'a> {
    /// Native owner category, without interpreting an ordinal as an RDF identity.
    #[must_use]
    pub fn kind(&self) -> CausalCauseKind {
        match self.cause.origin {
            Origin::ObjectDomain => CausalCauseKind::ObjectDomain,
            Origin::TermDenotation => CausalCauseKind::TermDenotation,
            Origin::PreparedType => CausalCauseKind::PreparedType,
            Origin::PreparedRole => CausalCauseKind::PreparedRole,
            Origin::PreparedEquality => CausalCauseKind::PreparedEquality,
            Origin::PreparedDistinct => CausalCauseKind::PreparedDistinct,
            Origin::AssumedType => CausalCauseKind::AssumedType,
            Origin::AssumedRole => CausalCauseKind::AssumedRole,
            Origin::AssumedAnonymousType => CausalCauseKind::AssumedAnonymousType,
            Origin::RefutationRoot => CausalCauseKind::RefutationRoot,
            Origin::Internalized => CausalCauseKind::Internalized,
            Origin::Application => CausalCauseKind::Application,
            Origin::Unattributed => unreachable!("native journal admission requires an owner"),
        }
    }

    /// Native owner ordinal within this graph's prepared input, question or prefix.
    #[must_use]
    pub const fn ordinal(&self) -> usize {
        self.cause.index
    }

    /// Native head-atom ordinal within a selected application.
    #[must_use]
    pub const fn atom(&self) -> usize {
        self.cause.atom
    }

    /// Selected original input owner; absent for non-input causes.
    #[must_use]
    pub fn input(&self) -> Option<CausalInput<'a>> {
        self.scope
            .support
            .inputs
            .iter()
            .find(|input| input.cause == Some(self.cause))
            .map(|input| CausalInput {
                kb: self.scope.kb,
                input: input.input,
                source: input.source,
            })
    }

    /// Original internalized formula and its source license, retained in the checked phase.
    /// This never reconstructs membership provenance from a later query's metadata.
    #[must_use]
    pub fn normalization(&self) -> Option<(ResolvedConcept<'a>, ResolvedAxiomSource<'a>)> {
        if self.cause.origin != Origin::Internalized {
            return None;
        }
        let &(concept, inclusion) = self
            .scope
            .support
            .internalized
            .iter()
            .find(|&&(concept, _)| concept as usize == self.cause.index)
            .expect("internalized source owners were captured in their original phase");
        Some((
            ResolvedConcept::of(self.scope.kb, concept),
            origin::resolve(self.scope.kb, self.scope.kb.tbox_sources[inclusion]),
        ))
    }
}

impl std::fmt::Debug for CausalCause<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CausalCause")
            .field("kind", &self.kind())
            .field("ordinal", &self.cause.index)
            .field("atom", &self.cause.atom)
            .finish()
    }
}

/// A replay-local node capability, distinct from a source term even after aliases.
#[derive(Clone, Copy)]
pub struct CausalNode<'a> {
    scope: CausalClash<'a>,
    node: usize,
}

impl<'a> CausalNode<'a> {
    fn birth(&self) -> crate::owl_dl::graph::changes::Event {
        let graph = &self.scope.support.derivation;
        let at = graph
            .births
            .binary_search_by_key(&self.node, |&(node, _)| node)
            .expect("selected nodes retain their original births");
        self.scope.support.effects[graph.births[at].1].1
    }

    /// Original typed reserved descriptor from the independently checked introducing case.
    /// Its recursive origin and set index are proof-local identities, never minted RDF names.
    #[must_use]
    pub fn reserved_identity(&self) -> Option<ResolvedReserved<'a>> {
        let birth = self.birth();
        if !matches!(birth.mutation, Mutation::Node { reserved: true, .. }) {
            return None;
        }
        let (branch, ordinal) = self
            .scope
            .support
            .owners
            .iter()
            .find_map(|owner| match owner {
                ReadOwner::Application { index, branch, .. } if *index == birth.cause.index => {
                    *branch
                }
                _ => None,
            })
            .expect("reserved birth retains its checked introducing application");
        let step = &self.scope.proof.branches[branch];
        let alternative = step
            .alternatives()
            .iter()
            .chain(step.introduced())
            .nth(ordinal)
            .expect("native introduction checked the original alternative");
        let crate::owl_dl::proof::ProofGround::EqualReserved { root, .. } =
            &alternative.atoms[birth.cause.atom]
        else {
            unreachable!("native reserved birth belongs to the checked reserved head")
        };
        Some(ResolvedReserved {
            kb: self.scope.kb,
            root,
        })
    }
    /// Original birth denotation, if this node was born with an RDF name or literal.
    /// Later aliases remain separate checked merge/transfer effects.
    #[must_use]
    pub fn term(&self) -> Option<ResolvedTerm<'a>> {
        let Mutation::Node { named, .. } = self.birth().mutation else {
            unreachable!("birth entries address the original node effect");
        };
        named.map(|id| term(self.scope.kb, id))
    }

    /// Original local ordinal, meaningful only with this borrowed graph capability.
    #[must_use]
    pub const fn ordinal(&self) -> usize {
        self.node
    }

    /// Whether two handles refer to the same node in the same checked graph.
    /// A false result is not a semantic inequality claim across different graphs.
    #[must_use]
    pub fn same_node(&self, other: &CausalNode<'_>) -> bool {
        std::ptr::eq(self.scope.support, other.scope.support) && self.node == other.node
    }
}

impl std::fmt::Debug for CausalNode<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CausalNode")
            .field("local", &self.node)
            .field("term", &self.term())
            .finish()
    }
}

/// Exact selected mutation, resolving vocabulary through the consumer context.
#[derive(Debug)]
pub enum CausalMutation<'a> {
    /// Birth flags are native identity/domain metadata, never minted RDF vocabulary.
    Node {
        /// Newly observed node in this checked replay.
        node: CausalNode<'a>,
        /// Original RDF denotation at birth, when present.
        named: Option<ResolvedTerm<'a>>,
        /// Original predecessor of an existential witness.
        parent: Option<CausalNode<'a>>,
        /// Original generating role, preserving inverse direction.
        incoming: Option<ResolvedRole<'a>>,
        /// Whether native nominal introduction reserved this root.
        reserved: bool,
        /// Whether the node belongs to the datatype-value domain.
        concrete: bool,
    },
    /// Normalized concept membership.
    Label {
        /// Node whose membership was established.
        node: CausalNode<'a>,
        /// Original normalized concept.
        concept: ResolvedConcept<'a>,
    },
    /// Original oriented role assertion.
    Edge {
        /// Original raw edge ordinal.
        ordinal: usize,
        /// Original source endpoint.
        from: CausalNode<'a>,
        /// Original destination endpoint.
        to: CausalNode<'a>,
        /// Native property in its recorded edge orientation.
        role: ResolvedRole<'a>,
    },
    /// Native forced inequality.
    Distinct {
        /// First node forced distinct.
        left: CausalNode<'a>,
        /// Second node forced distinct.
        right: CausalNode<'a>,
    },
    /// Actual representative identification.
    Merge {
        /// Representative retained by the native identification.
        keep: CausalNode<'a>,
        /// Representative identified with the retained node.
        discard: CausalNode<'a>,
    },
    /// Membership transferred by an identification.
    LabelTransfer {
        /// Representative receiving the membership.
        keep: CausalNode<'a>,
        /// Original owner of the transferred membership.
        discard: CausalNode<'a>,
        /// Actual normalized concept transferred.
        concept: ResolvedConcept<'a>,
    },
    /// Exact denotation transferred by an identification.
    NameTransfer {
        /// Representative receiving the denotation.
        keep: CausalNode<'a>,
        /// Original owner of the denotation.
        discard: CausalNode<'a>,
        /// Exact original RDF term, including blank scope or literal metadata.
        individual: ResolvedTerm<'a>,
    },
    /// Original inequality and representative changes.
    DistinctTransfer {
        /// Representative receiving the inequality.
        keep: CausalNode<'a>,
        /// Original owner of the inequality.
        discard: CausalNode<'a>,
        /// Other endpoint before forwarding through native aliases.
        previous_other: CausalNode<'a>,
        /// Other endpoint after forwarding through native aliases.
        other: CausalNode<'a>,
    },
    /// Object-domain fact withdrawn after identification with a concrete value.
    DomainTransfer {
        /// Representative whose previous domain fact is withdrawn.
        keep: CausalNode<'a>,
        /// Identified node supplying the concrete-domain interpretation.
        discard: CausalNode<'a>,
    },
    /// Native datatype-value partition, whose ordinal is not an RDF term.
    ValueClass {
        /// Node whose native value partition was recorded.
        node: CausalNode<'a>,
        /// Original native datatype-value partition ordinal, not an RDF term.
        class: u32,
        /// Original owner when inherited through identification.
        inherited_from: Option<CausalNode<'a>>,
    },
    /// Native membership withdrawal.
    WithdrawLabel {
        /// Node whose previous membership was withdrawn.
        node: CausalNode<'a>,
        /// Actual normalized concept withdrawn.
        concept: ResolvedConcept<'a>,
    },
}

fn mutation(scope: CausalClash<'_>, mutation: Mutation) -> CausalMutation<'_> {
    let node = |node| CausalNode { scope, node };
    let concept = |id| ResolvedConcept::of(scope.kb, id);
    match mutation {
        Mutation::Node {
            node: id,
            named,
            parent,
            incoming,
            reserved,
            concrete,
        } => CausalMutation::Node {
            node: node(id),
            named: named.map(|id| term(scope.kb, id)),
            parent: parent.map(node),
            incoming: incoming.map(|r| role(scope.kb, r)),
            reserved,
            concrete,
        },
        Mutation::Label {
            node: id,
            concept: idc,
        } => CausalMutation::Label {
            node: node(id),
            concept: concept(idc),
        },
        Mutation::Edge {
            index,
            from,
            to,
            property,
        } => CausalMutation::Edge {
            ordinal: index,
            from: node(from),
            to: node(to),
            role: role(scope.kb, crate::owl_dl::concept::Role::Named(property)),
        },
        Mutation::Distinct { left, right } => CausalMutation::Distinct {
            left: node(left),
            right: node(right),
        },
        Mutation::Merge { keep, discard } => CausalMutation::Merge {
            keep: node(keep),
            discard: node(discard),
        },
        Mutation::LabelTransfer {
            keep,
            discard,
            concept: id,
        } => CausalMutation::LabelTransfer {
            keep: node(keep),
            discard: node(discard),
            concept: concept(id),
        },
        Mutation::NameTransfer {
            keep,
            discard,
            individual,
        } => CausalMutation::NameTransfer {
            keep: node(keep),
            discard: node(discard),
            individual: term(scope.kb, individual),
        },
        Mutation::DistinctTransfer {
            keep,
            discard,
            previous_other,
            other,
        } => CausalMutation::DistinctTransfer {
            keep: node(keep),
            discard: node(discard),
            previous_other: node(previous_other),
            other: node(other),
        },
        Mutation::DomainTransfer { keep, discard } => CausalMutation::DomainTransfer {
            keep: node(keep),
            discard: node(discard),
        },
        Mutation::ValueClass {
            node: id,
            class,
            inherited_from,
        } => CausalMutation::ValueClass {
            node: node(id),
            class,
            inherited_from: inherited_from.map(node),
        },
        Mutation::WithdrawLabel {
            node: id,
            concept: idc,
        } => CausalMutation::WithdrawLabel {
            node: node(id),
            concept: concept(idc),
        },
    }
}

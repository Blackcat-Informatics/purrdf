// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! Borrow the actual native syntax and role-source DAGs selected by checked replay.

use super::{CausalClash, ResolvedRole, ResolvedTerm, role, term};
use crate::owl_dl::{roles, syntax};

/// Original RDF triple read, retaining every term's exact consumer identity.
#[derive(Debug)]
pub struct CausalTriple<'a> {
    /// Exact original subject, including scoped blank identity.
    pub subject: ResolvedTerm<'a>,
    /// Exact original predicate.
    pub predicate: ResolvedTerm<'a>,
    /// Exact original object, including literal datatype/language/direction.
    pub object: ResolvedTerm<'a>,
}

fn triple(scope: CausalClash<'_>, (s, p, o): (u32, u32, u32)) -> CausalTriple<'_> {
    CausalTriple {
        subject: term(scope.kb, s),
        predicate: term(scope.kb, p),
        object: term(scope.kb, o),
    }
}

/// Actual original parser operation; source expression syntax remains separate from normalization.
#[derive(Debug)]
pub enum CausalSyntaxKind<'a> {
    /// Original term-recognition read.
    Recognition(ResolvedTerm<'a>),
    /// Original class-expression read.
    Expression(ResolvedTerm<'a>),
    /// Original datatype-range read.
    Range(ResolvedTerm<'a>),
    /// Original role-expression read.
    Role(ResolvedTerm<'a>),
    /// Original ordered RDF-list read.
    List(ResolvedTerm<'a>),
    /// Shared group of the original operation's reads.
    Group,
}

/// Source- or query-authored syntax node in the original selected DAG.
#[derive(Clone, Copy)]
pub struct CausalSyntaxNode<'a> {
    scope: CausalClash<'a>,
    space: syntax::Space,
    ordinal: usize,
}

impl<'a> CausalSyntaxNode<'a> {
    pub(super) const fn checked(
        scope: CausalClash<'a>,
        space: syntax::Space,
        ordinal: usize,
    ) -> Self {
        Self {
            scope,
            space,
            ordinal,
        }
    }

    fn syntax(&self) -> &'a syntax::Syntax {
        match self.space {
            syntax::Space::Source => &self.scope.kb.syntax,
            syntax::Space::Query => &self.scope.kb.query_syntax,
        }
    }

    /// Whether the original parser read belongs to the selected question's syntax.
    #[must_use]
    pub fn is_query(&self) -> bool {
        self.space == syntax::Space::Query
    }

    /// Original DAG-local ordinal; meaningful only within this source/query graph.
    #[must_use]
    pub const fn ordinal(&self) -> usize {
        self.ordinal
    }

    /// Actual original parser operation and its exact source term.
    #[must_use]
    pub fn kind(&self) -> CausalSyntaxKind<'a> {
        match self.syntax().nodes[self.ordinal].kind {
            syntax::Kind::Recognition(id) => CausalSyntaxKind::Recognition(term(self.scope.kb, id)),
            syntax::Kind::Expression(id) => CausalSyntaxKind::Expression(term(self.scope.kb, id)),
            syntax::Kind::Range(id) => CausalSyntaxKind::Range(term(self.scope.kb, id)),
            syntax::Kind::Role(id) => CausalSyntaxKind::Role(term(self.scope.kb, id)),
            syntax::Kind::List(id) => CausalSyntaxKind::List(term(self.scope.kb, id)),
            syntax::Kind::Group => CausalSyntaxKind::Group,
        }
    }

    /// Exact original RDF reads and shared children, without recursive expansion or copying.
    pub fn reads(&self) -> impl ExactSizeIterator<Item = CausalSyntaxRead<'a>> + 'a + use<'a> {
        let this = *self;
        self.syntax().nodes[self.ordinal]
            .reads
            .iter()
            .map(move |read| match *read {
                syntax::Read::Triple(s, p, o) => {
                    CausalSyntaxRead::Triple(triple(this.scope, (s, p, o)))
                }
                syntax::Read::Child(ordinal) => CausalSyntaxRead::Child(Self { ordinal, ..this }),
            })
    }
}

impl std::fmt::Debug for CausalSyntaxNode<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CausalSyntaxNode")
            .field("query", &self.is_query())
            .field("ordinal", &self.ordinal)
            .field("kind", &self.kind())
            .finish()
    }
}

/// Original RDF syntax read or shared native parser sub-operation.
#[derive(Debug)]
pub enum CausalSyntaxRead<'a> {
    /// Exact RDF triple consumed by the parser.
    Triple(CausalTriple<'a>),
    /// Original shared child, retaining its own source/query scope.
    Child(CausalSyntaxNode<'a>),
}

/// Native role compiler's selected source operation, separate from its RDF spelling.
#[derive(Debug)]
pub enum CausalRoleSourceKind<'a> {
    /// Original chain ordinal in the checked native program.
    Chain(usize),
    /// Original normalized signed inclusion.
    Inclusion(ResolvedRole<'a>, ResolvedRole<'a>),
    /// Original global reflexivity source.
    Reflexive(ResolvedRole<'a>),
}

/// Original role-hierarchy source and its actual list/inverse-expression reads.
#[derive(Clone, Copy)]
pub struct CausalRoleSource<'a> {
    scope: CausalClash<'a>,
    input: roles::source::Input,
}

impl<'a> CausalRoleSource<'a> {
    pub(super) const fn checked(scope: CausalClash<'a>, input: roles::source::Input) -> Self {
        Self { scope, input }
    }

    fn sources(&self) -> &'a roles::source::Sources {
        &self
            .scope
            .kb
            .role_program
            .as_ref()
            .expect("selected role sources require their original compiler owner")
            .source
            .sources
    }

    fn origin(&self) -> &'a roles::source::Origin {
        self.sources()
            .get(self.input)
            .expect("selected native role source was resolved before publication")
    }

    /// Actual native compiler source operation, resolving roles through the consumer vocabulary.
    #[must_use]
    pub fn kind(&self) -> CausalRoleSourceKind<'a> {
        match self.input {
            roles::source::Input::Chain(ordinal) => CausalRoleSourceKind::Chain(ordinal),
            roles::source::Input::Inclusion(sub, sup) => {
                CausalRoleSourceKind::Inclusion(role(self.scope.kb, sub), role(self.scope.kb, sup))
            }
            roles::source::Input::Reflexive(property) => {
                CausalRoleSourceKind::Reflexive(role(self.scope.kb, property))
            }
        }
    }

    /// Original authored RDF anchor, not a normalized or invented source axiom.
    #[must_use]
    pub fn anchor(&self) -> CausalTriple<'a> {
        triple(self.scope, self.origin().anchor)
    }

    /// Shared and operation-local RDF reads in their original native order.
    pub fn reads(&self) -> impl Iterator<Item = CausalTriple<'a>> + 'a + use<'a> {
        let this = *self;
        let sources = self.sources();
        let origin = self.origin();
        sources.reads[origin.shared.clone()]
            .iter()
            .chain(sources.reads[origin.reads.clone()].iter())
            .map(move |&read| triple(this.scope, read))
    }
}

impl std::fmt::Debug for CausalRoleSource<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CausalRoleSource")
            .field("kind", &self.kind())
            .field("anchor", &self.anchor())
            .finish()
    }
}

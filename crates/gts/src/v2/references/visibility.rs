// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use crate::v2::fields::push;
use crate::v2::rdf::LineKey;
use crate::v2::{FoldError, Line, RdfFold, Term, TermId};
use crate::{FastMap, FastSet};

use super::{Binding, Digest, GroundNode, GroundTerm, Referent, ResolvedLeaf, Suppression};

/// The kinds of suppression entries in registry order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
#[non_exhaustive]
pub enum SuppressionKind {
    /// Term occurrence.
    Term = 1,
    /// Quad identity.
    Quad = 2,
    /// Reifier occurrence in an annotation or reifier binding.
    Reifier = 3,
    /// Named graph membership.
    Graph = 4,
    /// Blob name.
    Blob = 5,
    /// Reifier binding identity.
    ReifierBinding = 6,
    /// Annotation identity.
    Annotation = 7,
    /// Binding identity.
    Binding = 8,
}

/// A resolved suppression entry and its first frame.
#[derive(Debug)]
#[non_exhaustive]
pub struct SuppressionClaim {
    /// Byte offset of the first frame containing the entry.
    pub offset: usize,
    /// Resolved entry in the file's fold.
    pub entry: Suppression<ResolvedLeaf>,
}

/// A suppression kind that hid no content for at least one entry of a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SuppressionUnmatched {
    /// Byte offset of the source frame.
    pub offset: usize,
    /// Kind of at least one unmatched entry.
    pub kind: SuppressionKind,
}

/// The hidden members of a fold, leaving the underlying records intact.
#[derive(Debug, Default)]
pub struct VisibleFold {
    hidden_lines: FastSet<LineKey>,
    hidden_bindings: FastSet<usize>,
    hidden_blobs: FastSet<Digest>,
    /// Unmatched entries grouped by originating frame and kind.
    pub unmatched: Vec<SuppressionUnmatched>,
}

impl VisibleFold {
    pub(in crate::v2) fn apply(
        fold: &RdfFold,
        bindings: &[Binding<ResolvedLeaf>],
        blobs: impl IntoIterator<Item = Digest>,
        claims: &[SuppressionClaim],
    ) -> Result<Self, FoldError> {
        let mut result = Self::default();
        if claims.is_empty() {
            return Ok(result);
        }
        let mut index = Matches::new(claims, fold)?;
        for line in fold.lines() {
            if index.line(line) {
                result.hidden_lines.try_reserve(1)?;
                result.hidden_lines.insert(line.key());
            }
        }
        for (id, binding) in bindings.iter().enumerate() {
            if index.binding(binding) {
                result.hidden_bindings.try_reserve(1)?;
                result.hidden_bindings.insert(id);
            }
        }
        for digest in blobs {
            if index.mark(Target::Blob(digest)) {
                result.hidden_blobs.try_reserve(1)?;
                result.hidden_blobs.insert(digest);
            }
        }
        index.mark_term_targets(fold);
        for claim in claims {
            if !claim
                .entry
                .target()
                .is_some_and(|target| index.targets[&target])
            {
                push(
                    &mut result.unmatched,
                    SuppressionUnmatched {
                        offset: claim.offset,
                        kind: claim.entry.kind(),
                    },
                )?;
            }
        }
        result
            .unmatched
            .sort_unstable_by_key(|finding| (finding.offset, finding.kind as u8));
        result.unmatched.dedup();
        Ok(result)
    }

    /// Whether the suppression record hides this RDF line.
    #[must_use]
    pub fn hides_line(&self, line: Line) -> bool {
        self.hidden_lines.contains(&line.key())
    }

    /// Whether it hides the binding at this index in the reference fold.
    #[must_use]
    pub fn hides_binding(&self, position: usize) -> bool {
        self.hidden_bindings.contains(&position)
    }

    /// Whether it hides declarations of the named blob.
    #[must_use]
    pub fn hides_blob(&self, digest: &Digest) -> bool {
        self.hidden_blobs.contains(digest)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Target {
    Term(TermId),
    Line(LineKey),
    Reifier(TermId),
    Graph(TermId),
    Blob(Digest),
    Binding(TermId, Referent),
}

impl<L> Suppression<L> {
    pub(in crate::v2) const fn kind(&self) -> SuppressionKind {
        match self {
            Self::Term(_) => SuppressionKind::Term,
            Self::Quad(..) => SuppressionKind::Quad,
            Self::Reifier(_) => SuppressionKind::Reifier,
            Self::Graph(_) => SuppressionKind::Graph,
            Self::Blob(_) => SuppressionKind::Blob,
            Self::ReifierBinding(..) => SuppressionKind::ReifierBinding,
            Self::Annotation(..) => SuppressionKind::Annotation,
            Self::Binding(_) => SuppressionKind::Binding,
        }
    }
}

impl Suppression<ResolvedLeaf> {
    fn target(&self) -> Option<Target> {
        // P3 §9.2 leaves every suppression with an unnamed leaf unmatched.
        let graph = |leaf: &Option<ResolvedLeaf>| match leaf.as_ref() {
            None => Some(None),
            Some(leaf) => leaf.named().map(Some),
        };
        Some(match self {
            Self::Term(leaf) => Target::Term(leaf.named()?),
            Self::Quad([s, p, o], g) => Target::Line(LineKey::Quad(
                [s.named()?, p.named()?, o.named()?],
                graph(g)?,
            )),
            Self::Reifier(leaf) => Target::Reifier(leaf.named()?),
            Self::Graph(leaf) => Target::Graph(leaf.named()?),
            Self::Blob(digest) => Target::Blob(*digest),
            Self::ReifierBinding([r, t], g) => {
                Target::Line(LineKey::ReifierBinding([r.named()?, t.named()?], graph(g)?))
            }
            Self::Annotation([r, p, o], g) => Target::Line(LineKey::Annotation(
                [r.named()?, p.named()?, o.named()?],
                graph(g)?,
            )),
            Self::Binding(binding) => Target::Binding(binding.term.named()?, binding.referent),
        })
    }
}

impl ResolvedLeaf {
    const fn named(&self) -> Option<TermId> {
        match self {
            Self::Named(id) => Some(*id),
            Self::Ground(_) => None,
        }
    }
}

#[derive(Clone, Copy, Default)]
struct TermOccurrence {
    hides: bool,
    occurs: bool,
}

struct Matches {
    targets: FastMap<Target, bool>,
    terms: Option<Vec<TermOccurrence>>,
}

impl Matches {
    fn new(claims: &[SuppressionClaim], fold: &RdfFold) -> Result<Self, FoldError> {
        let mut targets = FastMap::default();
        targets.try_reserve(claims.len())?;
        targets.extend(
            claims
                .iter()
                .filter_map(|claim| claim.entry.target().map(|target| (target, false))),
        );
        let terms = if targets
            .keys()
            .any(|target| matches!(target, Target::Term(_)))
        {
            let mut terms = Vec::new();
            terms.try_reserve_exact(fold.terms().len())?;
            terms.resize(fold.terms().len(), TermOccurrence::default());
            for target in targets.keys() {
                if let Target::Term(term) = target {
                    terms[term.index()].hides = true;
                }
            }
            // Every triple's components precede its handle in the fold.
            for (id, term) in fold.terms().enumerate() {
                if let Term::Triple(children) = term {
                    let child_hides = children.iter().any(|child| terms[child.index()].hides);
                    terms[id].hides |= child_hides;
                }
            }
            Some(terms)
        } else {
            None
        };
        Ok(Self { targets, terms })
    }

    fn mark(&mut self, target: Target) -> bool {
        if let Some(matched) = self.targets.get_mut(&target) {
            *matched = true;
            true
        } else {
            false
        }
    }

    fn binding(&mut self, binding: &Binding<ResolvedLeaf>) -> bool {
        match &binding.term {
            ResolvedLeaf::Named(term) => {
                self.term(*term) | self.mark(Target::Binding(*term, binding.referent))
            }
            ResolvedLeaf::Ground(ground) => {
                let mut hidden = false;
                for node in &ground.0 {
                    if let GroundNode::Triple(children) = node {
                        for child in children {
                            if let GroundTerm::Named(term) = child {
                                // Every matching term must be marked, even after one hides the binding.
                                hidden |= self.term(*term);
                            }
                        }
                    }
                }
                hidden
            }
        }
    }

    fn term(&mut self, id: TermId) -> bool {
        self.terms.as_mut().is_some_and(|terms| {
            let term = &mut terms[id.index()];
            term.occurs = true;
            term.hides
        })
    }

    fn terms(&mut self, ids: impl IntoIterator<Item = TermId>) -> bool {
        let mut hidden = false;
        for id in ids {
            hidden |= self.term(id);
        }
        hidden
    }

    fn graph(&mut self, id: Option<TermId>) -> bool {
        id.is_some_and(|id| self.term(id) | self.mark(Target::Graph(id)))
    }

    fn line(&mut self, line: Line) -> bool {
        let exact = self.mark(Target::Line(line.key()));
        exact
            | match line {
                Line::Quad(terms, graph) => self.terms(terms) | self.graph(graph),
                Line::ReifierBinding(terms, graph) => {
                    self.terms(terms) | self.graph(graph) | self.mark(Target::Reifier(terms[0]))
                }
                Line::Annotation(terms, _, graph) => {
                    self.terms(terms) | self.graph(graph) | self.mark(Target::Reifier(terms[0]))
                }
                Line::EmptyGraph(id) => self.graph(Some(id)),
            }
    }

    fn mark_term_targets(&mut self, fold: &RdfFold) {
        let Some(terms) = self.terms.as_mut() else {
            return;
        };
        // Every reachable term contributes exactly once, even when many
        // lines or bindings share a nested triple.
        for (id, term) in fold.terms().enumerate().rev() {
            if terms[id].occurs
                && let Term::Triple(children) = term
            {
                for child in children {
                    terms[child.index()].occurs = true;
                }
            }
        }
        for (target, matched) in &mut self.targets {
            if let Target::Term(id) = target {
                *matched = terms[id.index()].occurs;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use core::num::NonZeroU64;
    use std::borrow::Cow;

    use crate::v2::references::{GroundLeaf, GroundNode, GroundTerm};

    use super::{
        Binding, Line, RdfFold, Referent, ResolvedLeaf, Suppression, SuppressionClaim,
        SuppressionKind, SuppressionUnmatched, Term, VisibleFold,
    };

    #[test]
    fn suppression_hides_nested_occurrences_and_preserves_the_original_fold() {
        let mut fold = RdfFold::default();
        let mut iri = |text| fold.intern_term(Term::Iri(Cow::Borrowed(text))).unwrap();
        let subject = iri("http://example.org/s");
        let predicate = iri("http://example.org/p");
        let target = iri("http://example.org/target");
        let datatype = iri("http://example.org/datatype");
        let inner = fold
            .intern_term(Term::Triple([subject, predicate, target]))
            .unwrap();
        let outer = fold
            .intern_term(Term::Triple([subject, predicate, inner]))
            .unwrap();
        let literal = fold
            .intern_term(Term::Typed {
                lexical: Cow::Borrowed("value"),
                datatype,
            })
            .unwrap();
        let nested = Line::Quad([subject, predicate, outer], None);
        let typed = Line::Quad([subject, predicate, literal], None);
        let annotation = Line::Annotation([subject, predicate, literal], NonZeroU64::MAX, None);
        for line in [nested, typed, annotation] {
            fold.add_line(line).unwrap();
        }
        let name = [5; 32];
        let ground_binding = || Binding {
            term: ResolvedLeaf::Ground(GroundLeaf(vec![
                GroundNode::Pair {
                    frame: [6; 32],
                    label: vec![0],
                },
                GroundNode::Triple([
                    GroundTerm::Ground(0),
                    GroundTerm::Named(predicate),
                    GroundTerm::Named(inner),
                ]),
            ])),
            referent: Referent::Blob(name),
        };
        let bindings = [
            Binding {
                term: ResolvedLeaf::Named(outer),
                referent: Referent::Blob(name),
            },
            ground_binding(),
        ];
        let unnamed = SuppressionClaim {
            offset: 20,
            entry: Suppression::Binding(ground_binding()),
        };
        let visible = VisibleFold::apply(&fold, &bindings, [], &[unnamed]).unwrap();
        assert!(!visible.hides_binding(1));
        assert_eq!(
            visible.unmatched,
            [SuppressionUnmatched {
                offset: 20,
                kind: SuppressionKind::Binding,
            }]
        );

        let named = ResolvedLeaf::Named;
        let claims = [
            SuppressionClaim {
                offset: 14,
                entry: Suppression::Term(named(target)),
            },
            SuppressionClaim {
                offset: 14,
                entry: Suppression::Quad([named(subject), named(predicate), named(outer)], None),
            },
            SuppressionClaim {
                offset: 14,
                entry: Suppression::Annotation(
                    [named(subject), named(predicate), named(literal)],
                    None,
                ),
            },
            SuppressionClaim {
                offset: 14,
                entry: Suppression::Blob(name),
            },
            SuppressionClaim {
                offset: 44,
                entry: Suppression::Term(named(datatype)),
            },
            SuppressionClaim {
                offset: 44,
                entry: Suppression::Term(named(datatype)),
            },
        ];

        let visible = VisibleFold::apply(&fold, &bindings, [name], &claims).unwrap();
        assert!(visible.hides_line(nested));
        assert!(visible.hides_line(annotation));
        assert!(!visible.hides_line(typed));
        assert!(visible.hides_binding(0));
        assert!(visible.hides_binding(1));
        assert!(visible.hides_blob(&name));
        assert_eq!(
            visible.unmatched,
            [SuppressionUnmatched {
                offset: 44,
                kind: SuppressionKind::Term
            }]
        );
        assert!(fold.lines().eq([nested, typed, annotation]));
    }
}

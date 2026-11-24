// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use crate::v2::FoldError;
use crate::v2::fields::push;

use super::{
    Binding, EntryAdmission, Leaf, LocalTerms, ReferenceClaims, ReferenceLocation, ScopedLeaf,
    Suppression,
};

/// Findings reported once for a folded binding or suppression frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReferenceFindings {
    /// Frame containing the affected binding or suppression entry.
    pub at: ReferenceLocation,
    /// At least one entry names a term that has not yet been defined.
    pub forward_reference: bool,
    /// At least one entry names an unreadable term or leaf.
    pub opaque_term: bool,
}

enum Entries {
    Bindings(Vec<EntryAdmission<Binding<ScopedLeaf>>>),
    Suppressions(Vec<EntryAdmission<Suppression<ScopedLeaf>>>),
}

struct Frame {
    at: ReferenceLocation,
    entries: Entries,
}

/// Preserve frame order while waiting for the next valid commit or segment end.
#[derive(Default)]
pub(in crate::v2) struct PendingReferences {
    frames: Vec<Frame>,
}

impl PendingReferences {
    pub(in crate::v2) fn bindings(
        &mut self,
        at: ReferenceLocation,
        bindings: Vec<Binding<Leaf<'_>>>,
        terms: &LocalTerms<'_>,
    ) -> Result<(), FoldError> {
        let entries = Entries::Bindings(admit(bindings, |binding| terms.binding(binding))?);
        push(&mut self.frames, Frame { at, entries }).map_err(FoldError::from)
    }

    pub(in crate::v2) fn suppressions(
        &mut self,
        at: ReferenceLocation,
        suppressions: Vec<Suppression<Leaf<'_>>>,
        terms: &LocalTerms<'_>,
    ) -> Result<(), FoldError> {
        let entries = Entries::Suppressions(admit(suppressions, |entry| terms.suppression(entry))?);
        push(&mut self.frames, Frame { at, entries }).map_err(FoldError::from)
    }

    pub(in crate::v2) fn settle(
        &mut self,
        committed: bool,
        fold: &mut ReferenceClaims,
        findings: &mut Vec<ReferenceFindings>,
    ) -> Result<(), FoldError> {
        for frame in self.frames.drain(..) {
            let at = frame.at;
            let mut report = ReferenceFindings {
                at,
                forward_reference: false,
                opaque_term: false,
            };
            match frame.entries {
                Entries::Bindings(entries) => settle(entries, committed, &mut report, |binding| {
                    fold.binding(at, binding)
                })?,
                Entries::Suppressions(entries) => {
                    settle(entries, committed, &mut report, |entry| {
                        fold.suppression(at, entry)
                    })?;
                }
            }
            if report.forward_reference || report.opaque_term {
                push(findings, report)?;
            }
        }
        Ok(())
    }
}

fn admit<I, T>(
    entries: Vec<I>,
    mut admit: impl FnMut(I) -> Result<EntryAdmission<T>, FoldError>,
) -> Result<Vec<EntryAdmission<T>>, FoldError> {
    let mut admitted = Vec::new();
    admitted.try_reserve_exact(entries.len())?;
    for entry in entries {
        admitted.push(admit(entry)?);
    }
    Ok(admitted)
}

fn settle<T>(
    entries: Vec<EntryAdmission<T>>,
    committed: bool,
    report: &mut ReferenceFindings,
    mut include: impl FnMut(T) -> Result<(), FoldError>,
) -> Result<(), FoldError> {
    for entry in entries {
        match entry {
            EntryAdmission::Accepted(entry) => include(entry)?,
            EntryAdmission::HeldUntilCommit(entry) if committed => include(entry)?,
            EntryAdmission::ForwardReference => report.forward_reference = true,
            EntryAdmission::HeldUntilCommit(_) | EntryAdmission::OpaqueTerm => {
                report.opaque_term = true;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use crate::v2::references::{FrameOwners, Referent, ResolvedLeaf, ScopedNode, StateCatalog};
    use crate::v2::{RdfFold, Term};

    use super::{
        Binding, Entries, EntryAdmission, Frame, PendingReferences, ReferenceClaims,
        ReferenceFindings, ReferenceLocation, ScopedLeaf,
    };

    #[test]
    fn a_commit_releases_only_bindings_held_by_a_broken_link() {
        let mut graph = RdfFold::default();
        let id = graph
            .intern_term(Term::Iri(Cow::Borrowed("http://example.org/id")))
            .unwrap();
        let at = ReferenceLocation {
            segment: 0,
            offset: 8,
        };
        for committed in [false, true] {
            let binding = Binding {
                term: ScopedLeaf(vec![ScopedNode::Term(id)]),
                referent: Referent::Blob([4; 32]),
            };
            let mut pending = PendingReferences {
                frames: vec![Frame {
                    at,
                    entries: Entries::Bindings(vec![
                        EntryAdmission::ForwardReference,
                        EntryAdmission::HeldUntilCommit(binding),
                        EntryAdmission::OpaqueTerm,
                    ]),
                }],
            };
            let mut claims = ReferenceClaims::default();
            let mut findings = Vec::new();
            pending
                .settle(committed, &mut claims, &mut findings)
                .unwrap();
            assert_eq!(
                findings,
                [ReferenceFindings {
                    at,
                    forward_reference: true,
                    opaque_term: true
                }]
            );
            let fold = claims
                .finish(
                    &mut graph,
                    &FrameOwners::default(),
                    &[],
                    &StateCatalog::default(),
                )
                .unwrap();
            if committed {
                let [binding] = fold.bindings.as_slice() else {
                    panic!("expected one released binding");
                };
                assert_eq!(binding.term, ResolvedLeaf::Named(id));
            } else {
                assert_eq!(fold.bindings, []);
            }
        }
    }
}

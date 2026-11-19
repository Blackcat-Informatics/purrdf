// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

//! The default canonical form: the nested-refinement labelling of a dataset
//! view.
//!
//! A view is lowered to typed lines, the engine of [`crate::canonical_labelling`]
//! labels their blank nodes, and the result is read back as the canonical
//! document (the canonical line sequence spelled as N-Quads, every blank node
//! `_:` and its position in decimal), the view's own blank-node labels, the
//! rewritten dataset, or the digest of the canonical line sequence.
//!
//! One path serves every presentation of the result: there is no overlay, no
//! flat presentation, no hash to select and no reserved vocabulary. A dataset
//! that holds a statement of any IRI is labelled like any other. The caller's
//! [`SearchPolicy`] becomes the engine's own limit: `Finite(n)` is `n` limited
//! units, `Unlimited` is the engine's documented limit. The engine refuses with
//! the typed refusal of its definition ([`CanonError::Refused`]) only for an
//! input it cannot label within that limit or an input that is not well
//! formed.
//!
//! The RDFC-1.0 family, which this module replaces as the default, is
//! [`crate::rdfc12`].

use std::collections::BTreeMap;

use purrdf_hash::hex::Digest32;

use super::control::SearchRun;
use super::result::ResultBuilder;
use super::{
    CanonError, CanonOutput, CanonPhase, CanonicalRelabeling, CanonicalizationControl,
    Canonicalized, RelabelSource, SearchConfiguration, SearchPolicy, rebuild_with_labels,
    rendering::ScratchGrant,
};
use crate::canonical_labelling::{
    InputReason, Labelling, Limit, Refusal, Scope, ViewError, label_view_with_memory,
    lines_of_view, write_line_with,
};
use crate::{
    DatasetView, FallibleDatasetView, RdfDataset, TermId, ViewCanonError, checkpointed_drain,
};

/// The name of the labelling that produces the default canonical form.
pub const CANON_LABELLING_NAME: &str = "nested-refinement-labelling";

/// The version of [`CANON_LABELLING_NAME`] this build implements. The name and
/// the version enter every identity and every position (7.15 of its
/// definition), so a consumer that records a digest records both.
pub const CANON_LABELLING_VERSION: u64 = crate::canonical_labelling::VERSION;

/// The identifier of the one presentation of the default canonical form: the
/// canonical line sequence spelled as N-Quads 1.2, in the order of the
/// sequence, with decimal blank-node labels.
pub const CANON_PRESENTATION_LINES_ID: &str = "line-sequence";

/// The version of [`CANON_PRESENTATION_LINES_ID`].
pub const CANON_PRESENTATION_LINES_VERSION: u32 = 1;

/// The bound first-party corpora and tests state when a policy is required:
/// one million limited units of the engine. A caller states its own; there is
/// no default.
pub const CANON_CORPUS_POLICY: SearchPolicy = SearchPolicy::Finite(1_000_000);

/// A canonical document with the digest of its line sequence.
type DocumentAndDigest<I, E> = Result<(Canonicalized<I>, Digest32), CanonError<E>>;

/// The result shape the fault-aware entry points share.
type CheckedResult<D, T> = Result<
    T,
    ViewCanonError<<D as FallibleDatasetView>::Error, <D as FallibleDatasetView>::Evidence>,
>;

/// The engine's limit for a caller's policy.
const fn limit_of(policy: SearchPolicy) -> Limit {
    match policy {
        SearchPolicy::Finite(units) => Limit::new(units),
        SearchPolicy::Unlimited => Limit::ENGINE,
    }
}

/// The name of a refusal's variant, for a message.
pub(super) const fn refusal_name(refusal: &Refusal) -> &'static str {
    match refusal {
        Refusal::Input { .. } => "the lines are not well formed (input)",
        Refusal::Graph { .. } => {
            "a graph an empty-graph line holds is carried by another line (graph)"
        }
        Refusal::Width { .. } => "a quantity does not fit in 64 bits (width)",
        Refusal::Limit { .. } => {
            "the engine's own limit tripped on an input that is not guaranteed (limit)"
        }
    }
}

/// A refusal of the labelling or an error of the run, as a [`CanonError`].
fn lift<E>(error: ViewError<E>) -> CanonError<E> {
    match error {
        ViewError::Read(error) => CanonError::Read(error),
        ViewError::ForeignId => CanonError::SourceInconsistent,
        ViewError::Refused(refusal) => CanonError::Refused(Box::new(refusal)),
        ViewError::Stopped => CanonError::Stopped,
        ViewError::Storage(error) => CanonError::Storage(error),
        ViewError::Lowering(refusal) => CanonError::Lowering(refusal),
    }
}

/// Label `view` within `scope` under the run's policy and stop signal, and
/// read the result with `finish`. The labelling's working state is admitted
/// once, before the engine starts, and released after `finish`.
fn labelled<D: DatasetView, T>(
    view: &D,
    scope: Scope<'_>,
    run: &SearchRun<'_>,
    finish: impl FnOnce(&Labelling<D::Id>) -> Result<T, CanonError<D::ReadError>>,
) -> Result<T, CanonError<D::ReadError>> {
    run.check_stop().map_err(|_| CanonError::Stopped)?;
    let reservation = view.reserve_workspace(0).map_err(CanonError::Read)?;
    ScratchGrant::new(reservation).run(|memory| {
        let (labelling, bytes) =
            label_view_with_memory(view, &[], scope, limit_of(run.policy()), run.stop(), memory)
                .map_err(|error| {
                    if let ViewError::Refused(Refusal::Limit { units, .. }) = &error {
                        run.record_units(*units);
                        run.observe_limit();
                    }
                    lift(error)
                })?;
        run.record_units(labelling.walked_units());
        run.phase(CanonPhase::Output);
        let out = finish(&labelling)?;
        // The lines the grant covers are destroyed before it is refunded.
        drop(labelling);
        memory.release_bytes(bytes).map_err(CanonError::Storage)?;
        Ok(out)
    })
}

/// The canonical document and the view's own blank-node labels.
fn document<D: DatasetView>(
    view: &D,
    labelling: &Labelling<D::Id>,
    run: &SearchRun<'_>,
) -> Result<Canonicalized<D::Id>, CanonError<D::ReadError>> {
    let mut builder = ResultBuilder::lines(view)?;
    let lines = &labelling.labelled().lines;
    // INVARIANT: the engine encodes every line it returns by 3.7 and the
    // lowering writes every composite record it reads, so a spelling failure
    // is unreachable; a refusal names the line kind it cannot read.
    let unreadable = |_| {
        CanonError::Refused(Box::new(Refusal::Input {
            reason: InputReason::Kind,
        }))
    };
    let composites = labelling.composites().map_err(unreadable)?;
    // One pass measures, one writes, so the document is reserved once.
    let mut text = String::new();
    let mut total = 0usize;
    for line in lines {
        run.check_stop().map_err(|_| CanonError::Stopped)?;
        text.clear();
        write_line_with(line, &composites, &mut text).map_err(unreadable)?;
        total = total
            .checked_add(text.len())
            .ok_or(CanonError::CounterCapacity)?;
    }
    builder.reserve_text(total)?;
    for line in lines {
        run.check_stop().map_err(|_| CanonError::Stopped)?;
        text.clear();
        write_line_with(line, &composites, &mut text).map_err(unreadable)?;
        if !text.is_empty() {
            builder.append(&text)?;
        }
    }
    let mut labels: Vec<(D::Id, u64)> = labelling.view_positions().collect();
    labels.sort_unstable_by_key(|&(id, _)| id);
    builder.reserve_labels(labels.len())?;
    for (id, position) in labels {
        builder.label(id, position.to_string().into_boxed_str())?;
    }
    builder.finish(run.stop())
}

/// Feed every line of the canonical document to `hasher` as it is spelled, in
/// the document's order, through one reused line buffer. The document is never
/// built: the bytes hashed are exactly the bytes [`document`] would append.
fn stream_document<D: DatasetView>(
    labelling: &Labelling<D::Id>,
    run: &SearchRun<'_>,
    hasher: &mut sha2::Sha256,
) -> Result<(), CanonError<D::ReadError>> {
    use sha2::Digest as _;
    // INVARIANT: as in `document`, a spelling failure is unreachable.
    let unreadable = |_| {
        CanonError::Refused(Box::new(Refusal::Input {
            reason: InputReason::Kind,
        }))
    };
    let composites = labelling.composites().map_err(unreadable)?;
    let mut text = String::new();
    for line in &labelling.labelled().lines {
        run.check_stop().map_err(|_| CanonError::Stopped)?;
        text.clear();
        write_line_with(line, &composites, &mut text).map_err(unreadable)?;
        hasher.update(text.as_bytes());
    }
    run.check_stop().map_err(|_| CanonError::Stopped)
}

/// Feed the canonical document of `view` to `hasher`, line by line, without
/// building it. The hasher then holds exactly the bytes of
/// [`try_canonicalize_view`]'s [`nquads`](Canonicalized::nquads), and the caller
/// continues the same hash. On an error the hasher's state is unspecified.
///
/// # Errors
/// As [`try_canonicalize_view`], except that no document is admitted to the
/// owned workspace.
pub(crate) fn try_hash_document_view<D: DatasetView>(
    view: &D,
    control: &mut CanonicalizationControl<'_>,
    hasher: &mut sha2::Sha256,
) -> Result<(), CanonError<D::ReadError>> {
    control.run(
        SearchConfiguration::labelling(None, CanonOutput::Sha256Digest),
        |run| {
            view.checked_read(|view| {
                labelled(view, Scope::Dataset, run, |labelling| {
                    stream_document::<D>(labelling, run, hasher)
                })
            })
            .map_err(CanonError::Read)?
        },
    )
}

/// The digest of the canonical line sequence.
fn digest(labelling: &Labelling<impl Copy>) -> Digest32 {
    Digest32::new(labelling.labelled().digest)
}

/// Canonicalize `ds`: its canonical document, labelled by the
/// nested-refinement labelling ([`CANON_LABELLING_NAME`] at
/// [`CANON_LABELLING_VERSION`]).
///
/// **This is the entry point for untrusted input.** Every refusal is a typed
/// value the caller can propagate; an input the engine cannot label within the
/// policy's limit never aborts the process.
///
/// # Errors
/// [`CanonError::Refused`] with the typed refusal of the definition (an
/// ill-formed input, a quantity past 64 bits, or the engine's own limit);
/// [`CanonError::Stopped`] when the caller's stop signal is set; storage and
/// admission refusals.
pub fn try_canonicalize(
    ds: &RdfDataset,
    control: &mut CanonicalizationControl<'_>,
) -> Result<Canonicalized, CanonError> {
    try_canonicalize_view(ds, control)
}

/// Canonicalize any [`DatasetView`]: the view-generic form of
/// [`try_canonicalize`], and the function that one delegates to.
///
/// Nothing is materialized: the view's quads, its RDF 1.2 reifier and
/// annotation rows and its term table are read in place, so a composite or
/// delta view canonicalizes without building an [`RdfDataset`]. The result's
/// [`nquads`](Canonicalized::nquads) carry no view-local ids, so a composite
/// view and the flat dataset holding the same content produce byte-equal
/// documents; [`labels`](Canonicalized::labels) is keyed on `view`'s own ids.
///
/// # Errors
/// As [`try_canonicalize`]; [`CanonError::Read`] retains the view's own read
/// error.
pub fn try_canonicalize_view<D: DatasetView>(
    view: &D,
    control: &mut CanonicalizationControl<'_>,
) -> Result<Canonicalized<D::Id>, CanonError<D::ReadError>> {
    control.run(
        SearchConfiguration::labelling(None, CanonOutput::Document),
        |run| {
            view.checked_read(|view| {
                labelled(view, Scope::Dataset, run, |labelling| {
                    document(view, labelling, run)
                })
            })
            .map_err(CanonError::Read)?
        },
    )
}

/// The digest of the canonical line sequence of `view`, with the document
/// never built.
///
/// # Errors
/// As [`try_canonicalize_view`].
pub fn try_canonical_digest_view<D: DatasetView>(
    view: &D,
    control: &mut CanonicalizationControl<'_>,
) -> Result<Digest32, CanonError<D::ReadError>> {
    control.run(
        SearchConfiguration::labelling(None, CanonOutput::LinesDigest),
        |run| {
            view.checked_read(|view| {
                labelled(view, Scope::Dataset, run, |labelling| Ok(digest(labelling)))
            })
            .map_err(CanonError::Read)?
        },
    )
}

/// The canonical document and the digest of its line sequence, from one
/// labelling.
///
/// # Errors
/// As [`try_canonicalize_view`].
pub fn try_canonicalize_digest_view<D: DatasetView>(
    view: &D,
    control: &mut CanonicalizationControl<'_>,
) -> DocumentAndDigest<D::Id, D::ReadError> {
    control.run(
        SearchConfiguration::labelling(None, CanonOutput::DocumentLinesDigest),
        |run| {
            view.checked_read(|view| {
                labelled(view, Scope::Dataset, run, |labelling| {
                    Ok((document(view, labelling, run)?, digest(labelling)))
                })
            })
            .map_err(CanonError::Read)?
        },
    )
}

/// Canonicalize the statements `view` asserts in the named graph `graph` (an
/// IRI), with the graph slot erased: the per-graph identity a carrier digest
/// pins.
///
/// A quad belongs to the graph its own slot names, and so does a reifier or
/// annotation row; each is labelled with no graph, so the result is a function
/// of the graph's content alone. A graph the view uses nowhere labels no line
/// and has the digest of the empty input: absence is an empty subgraph, never
/// an error.
///
/// # Errors
/// As [`try_canonicalize_view`].
pub fn try_canonicalize_graph_view<D: DatasetView>(
    view: &D,
    graph: &str,
    control: &mut CanonicalizationControl<'_>,
) -> Result<Canonicalized<D::Id>, CanonError<D::ReadError>> {
    control.run(
        SearchConfiguration::labelling(Some(graph), CanonOutput::Document),
        |run| {
            view.checked_read(|view| {
                labelled(view, Scope::Graph(graph), run, |labelling| {
                    document(view, labelling, run)
                })
            })
            .map_err(CanonError::Read)?
        },
    )
}

/// The digest of the canonical line sequence of one named graph of `view`.
///
/// # Errors
/// As [`try_canonicalize_graph_view`].
pub fn try_graph_digest_view<D: DatasetView>(
    view: &D,
    graph: &str,
    control: &mut CanonicalizationControl<'_>,
) -> Result<Digest32, CanonError<D::ReadError>> {
    control.run(
        SearchConfiguration::labelling(Some(graph), CanonOutput::LinesDigest),
        |run| {
            view.checked_read(|view| {
                labelled(view, Scope::Graph(graph), run, |labelling| {
                    Ok(digest(labelling))
                })
            })
            .map_err(CanonError::Read)?
        },
    )
}

/// [`try_canonicalize_view`] over a [`FallibleDatasetView`], bracketed by the
/// view's two status checkpoints ([`checkpointed_drain`]): a view at fault
/// before the run, or faulting between its internal passes, yields
/// [`ViewCanonError::NotReady`] and no result.
///
/// # Errors
/// [`ViewCanonError::Refused`] wrapping [`try_canonicalize_view`]'s errors;
/// [`ViewCanonError::NotReady`] when either checkpoint observes the view at
/// fault.
pub fn try_canonicalize_checked_view<D: FallibleDatasetView>(
    view: &D,
    control: &mut CanonicalizationControl<'_>,
) -> CheckedResult<D, Canonicalized<D::Id>> {
    control.run(
        SearchConfiguration::labelling(None, CanonOutput::Document),
        |run| match checkpointed_drain(view, |view| {
            labelled(view, Scope::Dataset, run, |labelling| {
                document(view, labelling, run)
            })
        }) {
            Ok(result) => result.map_err(ViewCanonError::Refused),
            Err(failure) => Err(failure.into()),
        },
    )
}

/// [`try_canonicalize_graph_view`] over a [`FallibleDatasetView`], bracketed
/// as [`try_canonicalize_checked_view`] is.
///
/// # Errors
/// As [`try_canonicalize_checked_view`].
pub fn try_canonicalize_checked_graph_view<D: FallibleDatasetView>(
    view: &D,
    graph: &str,
    control: &mut CanonicalizationControl<'_>,
) -> CheckedResult<D, Canonicalized<D::Id>> {
    control.run(
        SearchConfiguration::labelling(Some(graph), CanonOutput::Document),
        |run| match checkpointed_drain(view, |view| {
            labelled(view, Scope::Graph(graph), run, |labelling| {
                document(view, labelling, run)
            })
        }) {
            Ok(result) => result.map_err(ViewCanonError::Refused),
            Err(failure) => Err(failure.into()),
        },
    )
}

/// [`try_canonical_digest_view`] over a [`FallibleDatasetView`], bracketed as
/// [`try_canonicalize_checked_view`] is.
///
/// # Errors
/// As [`try_canonicalize_checked_view`].
pub fn try_canonical_digest_checked_view<D: FallibleDatasetView>(
    view: &D,
    control: &mut CanonicalizationControl<'_>,
) -> CheckedResult<D, Digest32> {
    control.run(
        SearchConfiguration::labelling(None, CanonOutput::LinesDigest),
        |run| match checkpointed_drain(view, |view| {
            labelled(view, Scope::Dataset, run, |labelling| Ok(digest(labelling)))
        }) {
            Ok(result) => result.map_err(ViewCanonError::Refused),
            Err(failure) => Err(failure.into()),
        },
    )
}

/// Whether `view` lowers to well-formed lines: the statements and records the
/// labelling is asked to label are admitted exactly when they are not refused
/// as `input` or `graph`. Screening at admission, before an identity is
/// minted, finds a view the labelling would refuse.
///
/// # Errors
/// [`CanonError::Refused`] with the typed refusal; [`CanonError::Read`] with
/// the view's own error.
pub fn check_admissible_view<D: DatasetView>(view: &D) -> Result<(), CanonError<D::ReadError>> {
    lines_of_view(view, &[]).map(|_| ()).map_err(lift)
}

/// [`check_admissible_view`] over a dataset.
///
/// # Errors
/// As [`check_admissible_view`].
pub fn check_admissible(ds: &RdfDataset) -> Result<(), CanonError> {
    check_admissible_view(ds)
}

/// [`check_admissible_view`] over a [`FallibleDatasetView`], bracketed as
/// [`try_canonicalize_checked_view`] is.
///
/// # Errors
/// [`ViewCanonError::Refused`] wrapping the refusal; [`ViewCanonError::NotReady`]
/// when either checkpoint observes the view at fault.
pub fn check_admissible_checked_view<D: FallibleDatasetView>(view: &D) -> CheckedResult<D, ()> {
    match checkpointed_drain(view, check_admissible_view) {
        Ok(result) => result.map_err(ViewCanonError::Refused),
        Err(failure) => Err(failure.into()),
    }
}

/// The decimal label of each blank node of `ds`, keyed by its id.
fn labels_of(labelling: &Labelling<TermId>) -> BTreeMap<TermId, Box<str>> {
    labelling
        .view_positions()
        .map(|(id, position)| (id, position.to_string().into_boxed_str()))
        .collect()
}

/// Relabel every blank node of `ds` to its canonical label at
/// [`BlankScope::DEFAULT`](crate::BlankScope::DEFAULT), returning a new frozen
/// dataset with every other term, quad, reifier, annotation, named-graph
/// declaration and source location preserved.
///
/// The label of a blank node is its position in decimal, as the canonical
/// document spells it (`_:0`, `_:1`, ...), and a blank node named inside a
/// composite literal is rewritten there to the same label, so no embedded
/// label dangles. A label is output only: the position order differs from the
/// byte order of labels (`10` precedes `2`), so a dataset read back from its
/// labels as names is not promised the identity map of the definition.
///
/// # Errors
/// As [`try_canonicalize`].
pub fn canonical_relabel(
    ds: &RdfDataset,
    control: &mut CanonicalizationControl<'_>,
) -> Result<RdfDataset, CanonError> {
    control.run(
        SearchConfiguration::labelling(None, CanonOutput::RelabeledDataset),
        |run| {
            labelled(ds, Scope::Dataset, run, |labelling| {
                rebuild_with_labels(ds, &labels_of(labelling), |_, _| {}, run)
            })
        },
    )
}

/// A canonical relabelling paired with the term mapping its rebuild
/// established, as [`canonical_relabel`] and one labelling.
///
/// # Errors
/// As [`try_canonicalize`].
pub fn canonical_relabel_with_mapping(
    ds: &RdfDataset,
    control: &mut CanonicalizationControl<'_>,
) -> Result<CanonicalRelabeling, CanonError> {
    control.run(
        SearchConfiguration::labelling(None, CanonOutput::RelabeledMapping),
        |run| {
            labelled(ds, Scope::Dataset, run, |labelling| {
                let mut terms = vec![None; ds.term_count()].into_boxed_slice();
                let dataset = rebuild_with_labels(
                    ds,
                    &labels_of(labelling),
                    |source, target| {
                        let source = match source {
                            RelabelSource::Term(id) => Some(id),
                            RelabelSource::IndirectIri(iri) => ds.term_id_by_iri(iri),
                        };
                        if let Some(source) = source {
                            let slot = &mut terms[source.index()];
                            assert!(
                                slot.is_none_or(|previous| previous == target),
                                "a canonical term rewrite must agree across occurrences"
                            );
                            *slot = Some(target);
                        }
                    },
                    run,
                )?;
                Ok(CanonicalRelabeling { dataset, terms })
            })
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canonical_labelling::label_view;
    use crate::{BlankScope, CanonPhase, RdfDatasetBuilder, SearchEvent};

    /// Two interchangeable blank nodes: the labelling must search to tell them
    /// from one another.
    fn twin_blanks() -> std::sync::Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let p = b.intern_iri("http://example.org/p").unwrap();
        let o = b.intern_iri("http://example.org/o").unwrap();
        for label in ["a", "b"] {
            let s = b.intern_blank(label, BlankScope::DEFAULT).unwrap();
            b.push_quad(s, p, o, None).unwrap();
        }
        b.freeze().unwrap()
    }

    /// A directed cycle of blank nodes: its one branching call makes twin
    /// tests, which the walk charges and the engine's explored units omit.
    fn blank_cycle(n: usize) -> std::sync::Arc<RdfDataset> {
        let mut b = RdfDatasetBuilder::new();
        let p = b.intern_iri("http://example.org/p").unwrap();
        let nodes: Vec<_> = (0..n)
            .map(|i| {
                b.intern_blank(&format!("n{i}"), BlankScope::DEFAULT)
                    .unwrap()
            })
            .collect();
        for i in 0..n {
            b.push_quad(nodes[i], p, nodes[(i + 1) % n], None).unwrap();
        }
        b.freeze().unwrap()
    }

    /// The identity a consumer records beside a digest is the name and version
    /// of the labelling and the line-sequence presentation, and every receipt of
    /// a run names the same pair.
    #[test]
    fn a_run_receipt_names_the_labelling_and_its_version() {
        assert_eq!(CANON_LABELLING_NAME, "nested-refinement-labelling");
        assert_eq!(CANON_LABELLING_VERSION, 1);
        assert_eq!(CANON_PRESENTATION_LINES_ID, "line-sequence");
        assert_eq!(CANON_PRESENTATION_LINES_VERSION, 1);

        let mut control = CanonicalizationControl::new(CANON_CORPUS_POLICY, None);
        let result = try_canonicalize(&twin_blanks(), &mut control).unwrap();
        assert_eq!(result.presentation(), None);
        let [receipt] = control.receipts() else {
            panic!("one run, one receipt");
        };
        assert_eq!(
            receipt.configuration,
            SearchConfiguration::Labelling {
                version: CANON_LABELLING_VERSION,
                selection: crate::CanonSelection::Dataset,
                output: CanonOutput::Document,
            }
        );
        assert!(receipt.completed);
        assert_eq!(receipt.phase, CanonPhase::Output);
        assert!(receipt.work.count(SearchEvent::LabellingUnit) > 0);
        assert_eq!(receipt.work.count(SearchEvent::RdfcFrame), 0);
    }

    /// A completed run records the units of the walk, the figure a refusal of
    /// the same limit reports, and not the engine's explored units.
    #[test]
    fn a_completed_run_records_the_units_of_the_walk() {
        let dataset = blank_cycle(6);
        let mut control = CanonicalizationControl::new(CANON_CORPUS_POLICY, None);
        try_canonicalize(&dataset, &mut control).unwrap();
        let [receipt] = control.receipts() else {
            panic!("one run, one receipt");
        };
        let labelling =
            label_view(&*dataset, &[], Scope::Dataset, Limit::new(1_000_000), None).unwrap();
        assert_eq!(
            receipt.work.count(SearchEvent::LabellingUnit),
            labelling.walked_units()
        );
        assert_ne!(
            labelling.walked_units(),
            labelling.labelled().explored.total(),
            "the walk charges twin units the explored figure leaves out"
        );
    }
}

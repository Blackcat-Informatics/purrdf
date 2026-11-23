// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::num::NonZeroU64;
use std::borrow::Cow;
use std::io::Write;

use purrdf_gts::v2::{
    FoldError, Line, LiteralDirection, RdfFold, SegmentWriteError, SegmentWriter, Term,
    TermId as WireId,
};
use purrdf_hash::error::NativeError;
use purrdf_lex::allocation::{Memory, Resident, retain_native_cause};
use purrdf_lex::walk::WorkList;

use crate::{QuadIds, RdfDataset, RdfDiagnostic, RdfTextDirection, TermId, TermRef};

/// Write the dataset as one committed GTS generation 2 segment.
///
/// The segment carries ordinary quads, reifier bindings, annotations and
/// explicitly empty named graphs. Blank-node identities remain distinct
/// across all scopes of the input dataset.
///
/// # Errors
/// Returns a diagnostic if the dataset cannot be represented or the sink
/// cannot accept the segment.
pub fn write_gts_v2(dataset: &RdfDataset, sink: impl Write) -> Result<(), RdfDiagnostic> {
    let mut fold = RdfFold::default();
    let ids = intern_terms(dataset, &mut fold)?;
    let map = |id: TermId| ids[id.index()].expect("the whole dictionary was interned");
    for QuadIds { s, p, o, g } in dataset.quads() {
        fold.add_line(Line::Quad([s, p, o].map(map), g.map(map)))
            .map_err(diagnostic)?;
    }
    for (reifier, triple, graph) in dataset.reifiers_with_graph() {
        fold.add_line(Line::ReifierBinding(
            [reifier, triple].map(map),
            graph.map(map),
        ))
        .map_err(diagnostic)?;
    }
    for QuadIds { s, p, o, g } in dataset.annotation_quads() {
        fold.add_line(Line::Annotation(
            [s, p, o].map(map),
            NonZeroU64::MIN,
            g.map(map),
        ))
        .map_err(diagnostic)?;
    }
    for graph in dataset.named_graphs() {
        fold.add_line(Line::EmptyGraph(map(graph)))
            .map_err(diagnostic)?;
    }
    let mut writer = SegmentWriter::new(sink)
        .map_err(SegmentWriteError::from)
        .map_err(diagnostic)?;
    writer.add_rdf(&fold).map_err(diagnostic)?;
    writer.add_commit().map_err(diagnostic)?;
    writer
        .into_inner()
        .flush()
        .map_err(SegmentWriteError::from)
        .map_err(diagnostic)?;
    Ok(())
}

fn intern_terms(
    dataset: &RdfDataset,
    fold: &mut RdfFold,
) -> Result<Vec<Option<WireId>>, RdfDiagnostic> {
    let mut ids = Vec::new();
    ids.try_reserve_exact(dataset.term_count())
        .map_err(FoldError::from)
        .map_err(diagnostic)?;
    ids.resize(dataset.term_count(), None);
    let mut pending = WorkList::<TermId, 32>::new();
    for index in 0..dataset.term_count() {
        if ids[index].is_some() {
            continue;
        }
        // RdfDataset's term count is bounded by its u32 identifier space.
        let id = TermId::from_index(u32::try_from(index).expect("dataset index fits its term ID"))
            .map_err(diagnostic)?;
        pending
            .try_push(id)
            .map_err(FoldError::from)
            .map_err(diagnostic)?;
        while let Some(id) = pending.pop() {
            if ids[id.index()].is_some() {
                continue;
            }
            let term = dataset.resolve(id);
            let dependencies = match term {
                TermRef::Literal {
                    datatype,
                    language: None,
                    ..
                } => [Some(datatype), None, None],
                TermRef::Triple { s, p, o } => [Some(s), Some(p), Some(o)],
                _ => [None; 3],
            };
            let missing = dependencies
                .into_iter()
                .rev()
                .flatten()
                .filter(|id| ids[id.index()].is_none());
            if missing.clone().next().is_some() {
                // Freezing the dataset checks that this dependency graph is acyclic.
                pending
                    .try_push(id)
                    .map_err(FoldError::from)
                    .map_err(diagnostic)?;
                for dependency in missing {
                    pending
                        .try_push(dependency)
                        .map_err(FoldError::from)
                        .map_err(diagnostic)?;
                }
            } else {
                let mapped = |id: TermId| ids[id.index()].expect("term dependencies were interned");
                let wire = fold
                    .intern_term(wire_term(term, mapped))
                    .map_err(diagnostic)?;
                ids[id.index()] = Some(wire);
            }
        }
    }
    Ok(ids)
}

fn wire_term(term: TermRef<'_>, mapped: impl Fn(TermId) -> WireId) -> Term<'_> {
    match term {
        TermRef::Iri(text) => Term::Iri(Cow::Borrowed(text)),
        TermRef::Blank { label, scope } => Term::Blank {
            segment: u64::from(scope.0),
            label: Cow::Borrowed(label.as_bytes()),
        },
        TermRef::Literal {
            lexical,
            datatype,
            language,
            direction,
        } => {
            if let Some(language) = language {
                Term::Language {
                    lexical: Cow::Borrowed(lexical),
                    language: Cow::Borrowed(language),
                    direction: direction.map(|direction| match direction {
                        RdfTextDirection::Ltr => LiteralDirection::LeftToRight,
                        RdfTextDirection::Rtl => LiteralDirection::RightToLeft,
                    }),
                }
            } else {
                Term::Typed {
                    lexical: Cow::Borrowed(lexical),
                    datatype: mapped(datatype),
                }
            }
        }
        TermRef::Triple { s, p, o } => Term::Triple([s, p, o].map(mapped)),
    }
}

fn diagnostic(cause: impl NativeError + 'static) -> RdfDiagnostic {
    let result = (|| {
        let cause = retain_native_cause(cause)?;
        RdfDiagnostic::error(
            "gts-v2-write",
            "could not write the GTS generation 2 dataset",
        )
        .with_native_cause(cause, &mut Memory::new(&mut Resident))
    })();
    result.unwrap_or_else(RdfDiagnostic::from)
}

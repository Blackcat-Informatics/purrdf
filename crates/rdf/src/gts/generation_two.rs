// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use core::fmt;
use std::sync::Arc;

use purrdf_core::FastMap;
use purrdf_core::blank_label::allocatable_scope;
use purrdf_core::cdt_blank::{BlankBinding, cdt_embedded_blanks_with_memory};
use purrdf_gts::v2::{
    Line, LiteralDirection, RdfFold, RdfGraph, ReadCapacity, ReadError, ReadGraph, SignatureInputs,
    Term, TermId as WireId, read_with_signature_inputs,
};
use purrdf_lex::allocation::{CapacitySpace, Memory, Resident, StorageError};

use crate::{BlankScope, RdfDataset, RdfDatasetBuilder, RdfDiagnostic, RdfTextDirection, TermId};

/// A native RDF dataset and the transport graph that supplied its lines.
#[derive(Debug)]
#[non_exhaustive]
pub struct GtsV2Dataset<'a> {
    /// RDF content for native RDF operations.
    pub dataset: Arc<RdfDataset>,
    /// Exact resolved GTS terms, counts, declarations and read evidence.
    pub graph: RdfGraph<'a>,
}

/// A generation 2 read or native RDF admission failure.
#[derive(Debug)]
#[non_exhaustive]
pub enum GtsV2DatasetError {
    /// The GTS decoder could not produce a graph.
    Read(ReadError),
    /// This entry point requires a generation 2 file.
    FirstGeneration,
    /// The resolved lines could not be admitted into the native dataset.
    Native(RdfDiagnostic),
}

impl fmt::Display for GtsV2DatasetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(cause) => cause.fmt(f),
            Self::FirstGeneration => f.write_str("the native GTS v2 reader requires generation 2"),
            Self::Native(cause) => cause.fmt(f),
        }
    }
}

purrdf_lex::error_source!(GtsV2DatasetError { Read, Native });
purrdf_lex::variant_from!(GtsV2DatasetError { Read(ReadError), Native(RdfDiagnostic) });

/// Read generation 2 and admit its resolved lines into a native RDF dataset.
///
/// The dataset uses native RDF validation and RDF set semantics. The graph
/// carries the original GTS terms, annotation counts, values, declaration
/// records and findings, including evidence associated with a partial read.
/// # Errors
/// Returns a typed generation, GTS read or native RDF admission failure.
#[expect(
    clippy::result_large_err,
    reason = "preserve the typed read failure without allocating on the refusal path"
)]
pub fn read_gts_v2_dataset<'a>(
    bytes: &'a [u8],
    capacity: ReadCapacity,
    inputs: &SignatureInputs<'_>,
) -> Result<GtsV2Dataset<'a>, GtsV2DatasetError> {
    let ReadGraph::Second(graph) = read_with_signature_inputs(bytes, capacity, inputs)? else {
        return Err(GtsV2DatasetError::FirstGeneration);
    };
    let dataset = dataset_from_gts_v2_fold(&graph.rdf)?;
    Ok(GtsV2Dataset { dataset, graph })
}

/// Admit the resolved RDF lines of a GTS v2 fold into the native RDF model.
///
/// Preserves named and explicitly empty graphs, reifier bindings, annotation
/// statements, triple terms, literal direction and distinct blank identities.
/// # Errors
/// Returns a native RDF diagnostic for invalid data or failed storage
/// admission. Composite literals with embedded blank nodes require native
/// scope binding and are refused at this boundary.
pub fn dataset_from_gts_v2_fold(fold: &RdfFold) -> Result<Arc<RdfDataset>, RdfDiagnostic> {
    let mut resident = Resident;
    let mut memory = Memory::new(&mut resident);
    let mut terms = Vec::new();
    let count = fold.terms().len();
    memory.reserve(&mut terms, count)?;
    terms.resize(count, TermSlot::Unused);
    mark_used_terms(fold, &mut terms)?;

    let mut builder = RdfDatasetBuilder::new();
    let mut scopes = FastMap::default();
    let mut label_text = String::new();
    for (index, term) in fold.terms().enumerate() {
        if !matches!(terms[index], TermSlot::Needed) {
            continue;
        }
        let native = match term {
            Term::Iri(iri) => builder.intern_iri_with_memory(iri, &mut memory)?,
            Term::Blank { segment, label } => {
                let scope = blank_scope(&mut scopes, *segment)?;
                let capacity = label
                    .len()
                    .checked_mul(2)
                    .and_then(|len| len.checked_add(1))
                    .ok_or(StorageError::SizeOverflow)?;
                label_text.clear();
                memory.reserve_string(&mut label_text, capacity)?;
                label_text.push('b');
                purrdf_hash::hex::encode_into(label, &mut label_text);
                builder.intern_blank_with_memory(&label_text, scope, &mut memory)?
            }
            Term::Typed { lexical, datatype } => {
                let Some(Term::Iri(datatype)) = fold.term(*datatype) else {
                    return Err(term_reference_error());
                };
                if !cdt_embedded_blanks_with_memory(lexical, datatype, &mut memory)?.is_empty() {
                    return Err(RdfDiagnostic::error(
                        "gts-v2-native-cdt-blank",
                        "a GTS composite literal with embedded blank nodes requires native scope binding",
                    ));
                }
                builder.intern_literal_parts_bound_with_memory(
                    lexical,
                    Some(datatype),
                    None,
                    None,
                    BlankBinding::Ambient(BlankScope::DEFAULT),
                    &mut memory,
                )?
            }
            Term::Language {
                lexical,
                language,
                direction,
            } => {
                let direction = direction.map(|direction| match direction {
                    LiteralDirection::LeftToRight => RdfTextDirection::Ltr,
                    LiteralDirection::RightToLeft => RdfTextDirection::Rtl,
                });
                builder.intern_literal_parts_with_memory(
                    lexical,
                    None,
                    Some(language),
                    direction,
                    &mut memory,
                )?
            }
            Term::Triple([s, p, o]) => builder.intern_triple_with_memory(
                native_id(&terms, *s)?,
                native_id(&terms, *p)?,
                native_id(&terms, *o)?,
                &mut memory,
            )?,
            _ => {
                return Err(RdfDiagnostic::error(
                    "gts-v2-native-term",
                    "the resolved GTS term has no native RDF term kind",
                ));
            }
        };
        terms[index] = TermSlot::Native(native);
    }
    memory.release_string(label_text)?;
    drop(scopes);
    for line in fold.lines() {
        let id = |id| native_id(&terms, id);
        let graph = |graph: Option<WireId>| graph.map(id).transpose();
        match line {
            Line::Quad([s, p, o], g) => {
                builder.push_quad_with_memory(id(s)?, id(p)?, id(o)?, graph(g)?, &mut memory)?;
            }
            Line::ReifierBinding([r, t], g) => {
                builder.push_reifier_in_graph_with_memory(
                    id(r)?,
                    id(t)?,
                    graph(g)?,
                    &mut memory,
                )?;
            }
            Line::Annotation([r, p, o], _, g) => {
                builder.push_annotation_in_graph_with_memory(
                    id(r)?,
                    id(p)?,
                    id(o)?,
                    graph(g)?,
                    &mut memory,
                )?;
            }
            Line::EmptyGraph(g) => builder.declare_named_graph_with_memory(id(g)?, &mut memory)?,
            _ => return Err(line_error()),
        }
    }
    memory.release_vec(terms)?;
    builder.freeze()
}

#[derive(Clone, Copy)]
enum TermSlot {
    Unused,
    Needed,
    Native(TermId),
}

fn mark_used_terms(fold: &RdfFold, terms: &mut [TermSlot]) -> Result<(), RdfDiagnostic> {
    let mut mark = |id: WireId| {
        *terms.get_mut(id.index()).ok_or_else(term_reference_error)? = TermSlot::Needed;
        Ok::<(), RdfDiagnostic>(())
    };
    for line in fold.lines() {
        let (roots, graph) = match line {
            Line::Quad([s, p, o], graph) | Line::Annotation([s, p, o], _, graph) => {
                ([Some(s), Some(p), Some(o)], graph)
            }
            Line::ReifierBinding([r, t], graph) => ([Some(r), Some(t), None], graph),
            Line::EmptyGraph(graph) => ([None; 3], Some(graph)),
            _ => return Err(line_error()),
        };
        for id in roots.into_iter().chain([graph]).flatten() {
            mark(id)?;
        }
    }
    for (index, term) in fold.terms().enumerate().rev() {
        if !matches!(terms[index], TermSlot::Needed) {
            continue;
        }
        let dependencies = match term {
            Term::Typed { datatype, .. } => [Some(*datatype), None, None],
            Term::Triple(terms) => terms.map(Some),
            _ => [None; 3],
        };
        for dependency in dependencies.into_iter().flatten() {
            *terms
                .get_mut(dependency.index())
                .ok_or_else(term_reference_error)? = TermSlot::Needed;
        }
    }
    Ok(())
}

fn native_id(terms: &[TermSlot], id: WireId) -> Result<TermId, RdfDiagnostic> {
    match terms.get(id.index()) {
        Some(TermSlot::Native(id)) => Ok(*id),
        _ => Err(term_reference_error()),
    }
}

fn blank_scope(
    scopes: &mut FastMap<u64, BlankScope>,
    segment: u64,
) -> Result<BlankScope, StorageError> {
    if let Some(&scope) = scopes.get(&segment) {
        return Ok(scope);
    }
    let scope = allocatable_scope(CapacitySpace::ImportBlankScopes, scopes.len() as u64 + 1)?;
    scopes
        .try_reserve(1)
        .map_err(|_| StorageError::AllocationFailed)?;
    scopes.insert(segment, scope);
    Ok(scope)
}

fn term_reference_error() -> RdfDiagnostic {
    RdfDiagnostic::error(
        "gts-v2-native-term-reference",
        "a resolved GTS term handle is unavailable",
    )
}

fn line_error() -> RdfDiagnostic {
    RdfDiagnostic::error(
        "gts-v2-native-line",
        "the resolved GTS line has no native RDF record kind",
    )
}

#[cfg(test)]
mod tests {
    use core::num::NonZeroU64;

    use purrdf_gts::v2::{FileEnd, SegmentWriter};

    use super::*;
    use crate::TermRef;

    #[test]
    fn generation_two_admits_only_used_composite_literals_with_native_grammar() {
        for (lexical, error_code) in [
            ("[1]", None),
            ("[_:local]", Some("gts-v2-native-cdt-blank")),
            ("[1,", Some("cdt-literal-malformed")),
        ] {
            let mut fold = RdfFold::default();
            let s = fold
                .intern_term(Term::Iri("https://example.org/s".into()))
                .unwrap();
            let p = fold
                .intern_term(Term::Iri("https://example.org/p".into()))
                .unwrap();
            let datatype = fold
                .intern_term(Term::Iri(purrdf_cdt::CDT_LIST.into()))
                .unwrap();
            fold.intern_term(Term::Typed {
                lexical: "[_:unused]".into(),
                datatype,
            })
            .unwrap();
            let o = fold
                .intern_term(Term::Typed {
                    lexical: lexical.into(),
                    datatype,
                })
                .unwrap();
            fold.add_line(Line::Quad([s, p, o], None)).unwrap();
            let mut writer = SegmentWriter::new(Vec::new()).unwrap();
            writer.add_rdf(&fold).unwrap();
            writer.add_commit().unwrap();
            let bytes = writer.into_inner();
            let result =
                read_gts_v2_dataset(&bytes, ReadCapacity::default(), &SignatureInputs::default());
            if let Some(code) = error_code {
                assert!(
                    matches!(result, Err(GtsV2DatasetError::Native(error)) if error.code == code)
                );
            } else {
                let decoded = result.unwrap();
                let row = decoded.dataset.quads().next().unwrap();
                assert!(matches!(
                    decoded.dataset.resolve(row.o),
                    TermRef::Literal { lexical: actual, .. } if actual == lexical
                ));
                assert!(matches!(decoded.graph.end, FileEnd::AtValidCommit));
            }
        }
    }

    #[test]
    fn generation_two_keeps_segment_blanks_distinct_and_retains_annotation_counts() {
        let mut fold = RdfFold::default();
        let s = fold
            .intern_term(Term::Blank {
                segment: 0,
                label: b"same".as_slice().into(),
            })
            .unwrap();
        let p = fold
            .intern_term(Term::Iri("https://example.org/p".into()))
            .unwrap();
        let o = fold
            .intern_term(Term::Iri("https://example.org/o".into()))
            .unwrap();
        let triple = fold.intern_term(Term::Triple([s, p, o])).unwrap();
        fold.add_line(Line::Quad([s, p, o], None)).unwrap();
        fold.add_line(Line::ReifierBinding([s, triple], None))
            .unwrap();
        let count = NonZeroU64::new(3).unwrap();
        fold.add_line(Line::Annotation([s, p, o], count, None))
            .unwrap();
        let mut writer = SegmentWriter::new(Vec::new()).unwrap();
        writer.add_rdf(&fold).unwrap();
        writer.add_commit().unwrap();
        let segment = writer.into_inner();
        let bytes = [segment.as_slice(), &segment].concat();
        let decoded =
            read_gts_v2_dataset(&bytes, ReadCapacity::default(), &SignatureInputs::default())
                .unwrap();
        let subjects: Vec<_> = decoded.dataset.quads().map(|row| row.s).collect();
        let [left, right] = subjects[..] else {
            panic!("both segments contribute a quad");
        };
        let TermRef::Blank { label, scope } = decoded.dataset.resolve(left) else {
            panic!("the first subject is blank");
        };
        assert_ne!(scope, BlankScope::DEFAULT);
        assert!(matches!(
            decoded.dataset.resolve(right),
            TermRef::Blank { label: other, scope: other_scope }
                if other == label && other_scope != scope && other_scope != BlankScope::DEFAULT
        ));
        assert_eq!(decoded.dataset.reifiers().count(), 2);
        assert_eq!(decoded.dataset.annotations().count(), 2);
        assert_eq!(
            decoded
                .graph
                .rdf
                .lines()
                .filter(|line| matches!(line, Line::Annotation(_, actual, _) if *actual == count))
                .count(),
            2
        );
    }
}

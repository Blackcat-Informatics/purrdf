// SPDX-FileCopyrightText: 2026 Blackcat Informatics® Inc. <paudley@blackcatinformatics.ca>
// SPDX-License-Identifier: MIT OR Apache-2.0 OR MulanPSL-2.0

use std::io::{self, Write};

use purrdf_lex::cbor::head;

use crate::v2::references::{LeafNode, write_blob_name, write_referent};
use crate::v2::{
    Binding, FoldError, FrameType, RdfFold, ReferenceFold, ResolvedLeaf, Suppression, TermId,
};

use super::rdf::{array, uint};
use super::{Buffer, SegmentWriteError, SegmentWriter};

// P7 §4.2 gives the entry count of a full reference frame.
const FRAME_ENTRIES: usize = 1_048_576;

impl<W: Write> SegmentWriter<W> {
    /// Append a resolved RDF fold and its bindings and suppression records.
    ///
    /// The supplied records use term handles from this fold. Named leaves
    /// receive the IDs assigned by the writer, and unnamed pairs retain their
    /// bytes. Frame, state, and blob names retain their original identifiers.
    /// Write any referenced declarations separately in the same file.
    ///
    /// # Errors
    /// Returns a fold, allocation, wire-range, or sink failure. If any item
    /// from this call has reached the sink, failure prevents further appends.
    pub fn add_rdf_with_references(
        &mut self,
        fold: &RdfFold,
        references: &ReferenceFold,
    ) -> Result<(), SegmentWriteError> {
        let ids = TermIds {
            base: self.term_base,
            len: fold.terms().len(),
        };
        let bindings = sorted_records(&references.bindings, |output, binding| {
            write_binding(output, binding, ids)
        })?;
        let suppressions = sorted_records(&references.suppressions, |output, claim| {
            write_suppression(output, &claim.entry, ids)
        })?;

        let previous = self.head;
        let result = self
            .add_rdf(fold)
            .and_then(|()| self.write_reference_section(FrameType::Bindings, &bindings))
            .and_then(|()| self.write_reference_section(FrameType::Suppress, &suppressions));
        if result.is_err() && self.head != previous {
            self.head = None;
        }
        result
    }

    fn write_reference_section(
        &mut self,
        kind: FrameType,
        entries: &[Vec<u8>],
    ) -> Result<(), SegmentWriteError> {
        let mut payload = Buffer::default();
        for batch in entries.chunks(FRAME_ENTRIES) {
            payload.0.clear();
            array(&mut payload, batch.len() as u64)?;
            for entry in batch {
                payload.write_all(entry)?;
            }
            self.append(kind, &payload.0, |output| {
                array(output, 1)?;
                uint(output, batch.len() as u64)
            })?;
        }
        Ok(())
    }
}

fn sorted_records<T>(
    entries: &[T],
    mut encode: impl FnMut(&mut Buffer, &T) -> Result<(), SegmentWriteError>,
) -> Result<Vec<Vec<u8>>, SegmentWriteError> {
    let mut records = Vec::new();
    records
        .try_reserve_exact(entries.len())
        .map_err(io::Error::other)?;
    for entry in entries {
        let mut encoded = Buffer::default();
        encode(&mut encoded, entry)?;
        records.push(encoded.0);
    }
    records.sort_unstable();
    records.dedup();
    Ok(records)
}

#[derive(Clone, Copy)]
struct TermIds {
    base: u128,
    len: usize,
}

impl TermIds {
    fn write(self, output: &mut impl Write, term: TermId) -> Result<(), SegmentWriteError> {
        if term.index() >= self.len {
            return Err(SegmentWriteError::ReferenceTerm(FoldError::TermHandle(
                term,
            )));
        }
        let id = u64::try_from(self.base + term.index() as u128)
            .map_err(|_| SegmentWriteError::DictionaryFull)?;
        variant(output, 1)?;
        uint(output, id)?;
        Ok(())
    }
}

fn write_binding(
    output: &mut Buffer,
    binding: &Binding<ResolvedLeaf>,
    ids: TermIds,
) -> Result<(), SegmentWriteError> {
    array(output, 2)?;
    write_leaf(output, &binding.term, ids)?;
    write_referent(output, &binding.referent)?;
    Ok(())
}

fn write_suppression(
    output: &mut Buffer,
    entry: &Suppression<ResolvedLeaf>,
    ids: TermIds,
) -> Result<(), SegmentWriteError> {
    variant(output, entry.kind() as u64)?;
    match entry {
        Suppression::Term(leaf) | Suppression::Reifier(leaf) | Suppression::Graph(leaf) => {
            write_leaf(output, leaf, ids)?;
        }
        Suppression::Quad(terms, graph) | Suppression::Annotation(terms, graph) => {
            write_row(output, terms, graph.as_ref(), ids)?;
        }
        Suppression::Blob(digest) => write_blob_name(output, digest)?,
        Suppression::ReifierBinding(terms, graph) => {
            write_row(output, terms, graph.as_ref(), ids)?;
        }
        Suppression::Binding(binding) => write_binding(output, binding, ids)?,
    }
    Ok(())
}

fn write_row(
    output: &mut Buffer,
    leaves: &[ResolvedLeaf],
    graph: Option<&ResolvedLeaf>,
    ids: TermIds,
) -> Result<(), SegmentWriteError> {
    array(output, leaves.len() as u64 + 1)?;
    for leaf in leaves {
        write_leaf(output, leaf, ids)?;
    }
    if let Some(graph) = graph {
        variant(output, 2)?;
        write_leaf(output, graph, ids)?;
    } else {
        array(output, 1)?;
        uint(output, 1)?;
    }
    Ok(())
}

fn write_leaf(
    output: &mut Buffer,
    leaf: &ResolvedLeaf,
    ids: TermIds,
) -> Result<(), SegmentWriteError> {
    leaf.visit(
        |node| {
            match node {
                LeafNode::Term(term) => ids.write(output, term)?,
                LeafNode::Blank { frame, label } => {
                    variant(output, 2)?;
                    array(output, 2)?;
                    head::write_bytes(output, frame)?;
                    head::write_bytes(output, label)?;
                }
                LeafNode::Triple => {
                    variant(output, 3)?;
                    array(output, 3)?;
                }
            }
            Ok(())
        },
        |error| io::Error::other(error).into(),
    )
}

fn variant(output: &mut impl Write, tag: u64) -> io::Result<()> {
    array(output, 2)?;
    uint(output, tag)
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use purrdf_ed25519::SigningKey;

    use crate::cose::SigningKeyRef;
    use crate::v2::signature::{ED25519, key_identifier};
    use crate::v2::{
        HeldKey, ReadCapacity, ReadGraph, Referent, ResolvedLeaf, SignatureInputs, Term,
        Visibility, read, read_with_signature_inputs,
    };

    use super::{
        Binding, Buffer, FrameType, RdfFold, SegmentWriter, Suppression, TermId, TermIds, array,
        uint, write_binding,
    };

    #[test]
    fn state_replay_retains_findings_from_folded_reference_frames() {
        let mut fold = RdfFold::default();
        let term = fold
            .intern_term(Term::Iri("https://example.org/term".into()))
            .unwrap();
        let mut writer = SegmentWriter::new(Vec::new()).unwrap();
        writer.add_rdf(&fold).unwrap();
        let digest = writer.add_blob(b"referent", None).unwrap();
        let mut payload = Buffer::default();
        array(&mut payload, 2).unwrap();
        for term in [term, TermId::from_index(1)] {
            write_binding(
                &mut payload,
                &Binding {
                    term: ResolvedLeaf::Named(term),
                    referent: Referent::Blob(digest),
                },
                TermIds { base: 0, len: 2 },
            )
            .unwrap();
        }
        writer
            .append(FrameType::Bindings, &payload.0, |output| {
                array(output, 1)?;
                uint(output, 2)
            })
            .unwrap();
        let commit = writer.add_commit().unwrap();
        let bytes = writer.into_inner();
        let ReadGraph::Second(graph) = read(&bytes, ReadCapacity::default()).unwrap() else {
            panic!("writer emitted a generation 2 opener");
        };
        let state = graph.read_state(&commit).unwrap().unwrap();
        assert_eq!(state.references.bindings.len(), 1);
        assert_eq!(state.reference_findings, graph.reference_findings);
        assert!(state.reference_findings[0].forward_reference);
    }

    #[test]
    fn reemitted_references_use_the_appended_dictionary_and_keep_hidden_rows() {
        let file = include_bytes!("../../../tests/fixtures/v2/P3/suppress-order-free.after.bin");
        let ReadGraph::Second(mut original) = read(file, ReadCapacity::default()).unwrap() else {
            panic!("fixture uses generation 2");
        };
        let Suppression::Quad(terms, None) = &original.references.suppressions[0].entry else {
            panic!("fixture suppresses a default-graph quad");
        };
        let ResolvedLeaf::Named(id) = terms[0] else {
            panic!("fixture names the suppressed subject");
        };
        let term = ResolvedLeaf::Named(id);
        let referent = Referent::Blob([7; 32]);
        original
            .references
            .bindings
            .push(Binding { term, referent });

        let mut prefix = RdfFold::default();
        prefix
            .intern_term(Term::Iri(Cow::Borrowed("http://example.org/prefix")))
            .unwrap();
        let mut writer = SegmentWriter::new(Vec::new()).unwrap();
        writer.add_rdf(&prefix).unwrap();
        writer
            .add_rdf_with_references(&original.rdf, &original.references)
            .unwrap();
        let key = SigningKey::from_bytes(&[42; 32]);
        writer
            .add_signed_commit(&[SigningKeyRef::Ed25519(&key)])
            .unwrap();
        let bytes = writer.into_inner();
        let public_key = key.verifying_key().to_bytes();
        let key_id = key_identifier(ED25519, &public_key);
        let keys = SignatureInputs::new(
            vec![key_id],
            vec![HeldKey {
                algorithm: ED25519,
                key_id,
                public_key: &public_key,
            }],
        );
        let ReadGraph::Second(output) =
            read_with_signature_inputs(&bytes, ReadCapacity::default(), &keys).unwrap()
        else {
            panic!("writer emitted a generation 2 opener");
        };
        assert_eq!(
            output.visibility,
            Visibility::Complete {
                unauthenticated_suppress: 0
            },
        );
        assert_eq!(output.references.bindings[0].referent, referent);
        assert_eq!(output.rdf.lines().count(), 2);
        assert_eq!(output.into_complete_rdf().unwrap().lines().count(), 1);
    }
}
